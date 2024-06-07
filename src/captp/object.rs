use std::sync::Arc;

use ed25519_dalek::VerifyingKey;
use futures::future::BoxFuture;
use syrup::{de::Sequence, literal, Encode, Symbol, TokenTree};

use super::{
    msg::{DescExport, DescImport},
    AbstractCapTpSession, CapTpDeliver, Delivery, GenericResolver, RemoteKey, SendError,
};
use crate::{
    async_compat::{mpsc, oneshot, OneshotRecvError},
    captp::msg::{OpDeliver, OpDeliverOnly},
};

mod bootstrap;
pub use bootstrap::*;

/// Sending half of an object pipe.
pub type DeliverySender<'args> = mpsc::UnboundedSender<Delivery<'args>>;
/// Receiving half of an object pipe.
pub type DeliveryReceiver<'args> = mpsc::UnboundedReceiver<Delivery<'args>>;

/// Returned by [`Object::deliver_only`]
pub enum ObjectOnlyError {}

/// Returned by [`Object`] functions.
#[derive(Debug, thiserror::Error)]
pub enum ObjectError {
    #[error(transparent)]
    Deliver(#[from] SendError),
    #[error(transparent)]
    Lex(#[from] syrup::de::LexError),
}

//impl<'i> ObjectError<'i> {
//    pub fn missing(position: usize, expected: &'static str) -> Self {
//        Self::MissingArgument { position, expected }
//    }
//
//    pub fn unexpected(
//        expected: &'static str,
//        position: usize,
//        received: syrup::TokenStream<'i>,
//    ) -> Self {
//        Self::UnexpectedArgument {
//            expected,
//            position,
//            received,
//        }
//    }
//}

pub trait Object {
    fn deliver_only(
        &self,
        session: Arc<dyn AbstractCapTpSession + Send + Sync>,
        args: Sequence<'static>,
    ) -> Result<(), ObjectError>;

    fn deliver<'object>(
        &'object self,
        session: Arc<dyn AbstractCapTpSession + Send + Sync>,
        args: Sequence<'static>,
        resolver: GenericResolver,
    ) -> BoxFuture<'object, Result<(), ObjectError>>;

    /// Called when this object is exported. By default, does nothing.
    #[allow(unused_variables)]
    fn exported(&self, remote_key: &VerifyingKey, position: DescExport) {}
}

// /// An object to which the answer to a Promise may be sent.
// pub struct RemoteResolver {
//     base: RemoteObject,
// }
//
// impl RemoteResolver {
//     pub async fn fulfill<'arg, Arg: Serialize + 'arg>(
//         &self,
//         args: impl IntoIterator<Item = &'arg Arg>,
//         answer_pos: Option<u64>,
//         resolve_me_desc: DescImport,
//     ) -> Result<(), SendError> {
//         self.base
//             .call("fulfill", args, answer_pos, resolve_me_desc)
//             .await
//     }
//
//     pub async fn break_promise(&self, error: impl Serialize) -> Result<(), SendError> {
//         self.base.call_only("break", &[error]).await
//     }
// }

pub type PromiseResult<'res> = Result<Sequence<'res>, TokenTree<'res>>;
pub type PromiseSender<'promise> = oneshot::Sender<PromiseResult<'promise>>;
pub type PromiseReceiver<'promise> = oneshot::Receiver<PromiseResult<'promise>>;

pub struct Resolver<'promise> {
    sender: parking_lot::Mutex<Option<PromiseSender<'promise>>>,
}

impl<'p> Resolver<'p> {
    fn resolve(&self, res: PromiseResult<'p>) -> Result<(), PromiseResult<'p>> {
        let Some(sender) = self.sender.lock().take() else {
            return Err(res);
        };
        sender.send(res)
    }
}

#[crate::impl_object(rexa = crate, tracing = ::tracing)]
impl<'args> Resolver<'args> {
    #[deliver()]
    async fn fulfill(
        &self,
        #[arg(args)] args: Sequence<'args>,
        #[arg(resolver)] resolver: GenericResolver,
    ) -> Result<(), ObjectError> {
        match self.resolve(Ok(args)) {
            Ok(_) => Ok(()),
            Err(_) => resolver
                .break_promise(literal![String; b"promise already resolved"])
                .await
                .map_err(From::from),
        }
    }

    #[deliver(symbol = "break")]
    async fn break_promise(
        &self,
        #[arg(syrup = arg)] reason: TokenTree<'args>,
        #[arg(resolver)] resolver: GenericResolver,
    ) -> Result<(), ObjectError> {
        match self.resolve(Err(reason)) {
            Ok(_) => Ok(()),
            Err(_) => resolver
                .break_promise(literal![String; b"promise already resolved"])
                .await
                .map_err(From::from),
        }
    }
}

impl<'p> Resolver<'p> {
    pub(crate) fn new() -> (Arc<Self>, Answer<'p>) {
        let (sender, receiver) = oneshot::channel();
        (
            Arc::new(Self {
                sender: Some(sender).into(),
            }),
            Answer { receiver },
        )
    }
}

/// An object representing the response to an [`OpDeliver`].
pub struct Answer<'promise> {
    receiver: PromiseReceiver<'promise>,
}

impl<'promise> std::future::Future for Answer<'promise> {
    type Output = <PromiseReceiver<'promise> as std::future::Future>::Output;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::future::Future::poll(std::pin::pin!(&mut self.receiver), cx)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum DeliverError<'input> {
    #[error(transparent)]
    Send(#[from] SendError),
    #[error(transparent)]
    Recv(#[from] OneshotRecvError),
    #[error("promise broken, reason: {0:?}")]
    Broken(syrup::TokenTree<'input>),
}

#[derive(Clone)]
pub struct RemoteObject {
    position: DescExport,
    session: Arc<dyn CapTpDeliver + Send + Sync + 'static>,
}

impl std::fmt::Debug for RemoteObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteObject")
            .field("position", &self.position)
            .finish_non_exhaustive()
    }
}

impl RemoteObject {
    pub(crate) fn new(
        session: Arc<dyn CapTpDeliver + Send + Sync + 'static>,
        position: DescExport,
    ) -> Self {
        Self { session, position }
    }

    pub fn session(&self) -> &Arc<dyn CapTpDeliver + Send + Sync + 'static> {
        &self.session
    }

    pub fn remote_vkey(&self) -> RemoteKey {
        self.session.remote_vkey()
    }

    pub async fn deliver_only<'i>(&self, args: Sequence<'i>) -> Result<(), SendError> {
        self.session
            .deliver_only(&OpDeliverOnly::new(self.position, args))
            .await
    }

    pub async fn deliver<'i>(
        &self,
        args: Sequence<'i>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        self.session
            .deliver(&OpDeliver::new(
                self.position,
                args,
                answer_pos,
                resolve_me_desc,
            ))
            .await
    }

    pub async fn deliver_and<'i>(
        &self,
        args: Sequence<'i>,
    ) -> Result<Sequence<'static>, DeliverError<'static>> {
        self.session.deliver_and(self.position, args).await
    }

    //pub async fn call_only<'arg>(
    //    &self,
    //    ident: impl Into<Symbol<'arg>>,
    //    args: Sequence<'arg>,
    //) -> Result<(), SendError> {
    //    args.stream.insert(0, ident.into().to_tokens());
    //    self.deliver_only(args).await.map_err(From::from)
    //}
    //
    //pub async fn call<'arg>(
    //    &self,
    //    ident: impl Into<Symbol<'arg>>,
    //    args: Sequence<'arg>,
    //    answer_pos: Option<u64>,
    //    resolve_me_desc: DescImport,
    //) -> Result<(), DeliverError<'static>> {
    //    args.stream.insert(0, ident.into().to_tokens());
    //    self.deliver(args, answer_pos, resolve_me_desc)
    //        .await
    //        .map_err(From::from)
    //}
    //
    //pub async fn call_and<'arg>(
    //    &self,
    //    ident: impl Into<Symbol<'arg>>,
    //    mut args: Sequence<'arg>,
    //) -> Result<Sequence<'static>, DeliverError<'static>> {
    //    args.stream.insert(0, ident.into().to_tokens());
    //    self.deliver_and(args).await
    //}

    // pub fn get_remote_object(&self, position: DescExport) -> Option<RemoteObject> {
    //     self.session.clone().into_remote_object(position)
    // }
    //
    // /// # Safety
    // /// - There must be an object exported at `position`.
    // #[allow(unsafe_code)]
    // pub unsafe fn get_remote_object_unchecked(&self, position: DescExport) -> RemoteObject {
    //     unsafe { self.session.clone().into_remote_object_unchecked(position) }
    // }
}

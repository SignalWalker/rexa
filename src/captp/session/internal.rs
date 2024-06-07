use super::{KeyMap, RecvError, SendError};
use crate::{
    async_compat::{AsyncRead, AsyncWrite, AsyncWriteExt},
    captp::{
        msg::{DescExport, DescImport, DescImportObject, DescImportPromise, Operation},
        object::Object,
        CapTpReadExt, IntoExport, RemoteKey,
    },
    locator::NodeLocator,
};
use dashmap::{DashMap, DashSet};
use ed25519_dalek::{SigningKey, VerifyingKey};
use futures::lock::Mutex;
use std::sync::{atomic::AtomicBool, Arc, RwLock};
use syrup::{
    de::{Literal, LiteralValue},
    Decode, Encode, Sequence, TokenTree,
};
use tracing::Instrument;

pub struct ExportManager {
    pub(super) remote_vkey: RemoteKey,
    /// Objects exported to the remote
    pub(super) exports: KeyMap<Arc<dyn Object + Send + Sync>>,
    /// Answers exported to the remote
    pub(super) answers: DashMap<u64, ()>,
}

impl ExportManager {
    fn new(remote_vkey: VerifyingKey) -> Self {
        Self {
            remote_vkey,
            // Bootstrap object handled internally.
            exports: KeyMap::with_initial(1),
            answers: DashMap::new(),
        }
    }

    pub fn export_object(&self, obj: impl IntoExport) -> DescImportObject {
        let obj = obj.into_export();
        let reserve = self.exports.reserve();
        obj.exported(&self.remote_vkey, reserve.key().into());
        DescImportObject {
            position: reserve.finalize(obj),
        }
    }

    pub fn export_answer(&self) -> DescImportPromise {
        todo!()
    }
}

pub(crate) struct CapTpSessionInternal<Reader, Writer> {
    reader: Mutex<Reader>,
    writer: Mutex<Writer>,
    pub(super) signing_key: SigningKey,

    pub(super) remote_vkey: RemoteKey,
    pub(super) remote_locator: NodeLocator<'static>,

    /// Objects imported from the remote
    pub(super) imports: DashSet<u64>,
    pub(super) exports: ExportManager,

    pub(super) aborted_by_remote: RwLock<Option<String>>,
    pub(super) aborted_locally: AtomicBool,
}

#[cfg(feature = "extra-diagnostics")]
impl<Reader, Writer> Drop for CapTpSessionInternal<Reader, Writer> {
    fn drop(&mut self) {
        if !self.is_aborted() {
            tracing::warn!(session = ?self, "dropping non-aborted session");
        }
    }
}

impl<Reader, Writer> std::fmt::Debug for CapTpSessionInternal<Reader, Writer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CapTpSessionInternal")
            .field("remote_vkey", &crate::hash(&self.remote_vkey))
            .finish_non_exhaustive()
    }
}

impl<Reader, Writer> CapTpSessionInternal<Reader, Writer> {
    pub(super) fn new(
        reader: Mutex<Reader>,
        writer: Mutex<Writer>,
        signing_key: SigningKey,
        remote_vkey: RemoteKey,
        remote_locator: NodeLocator<'static>,
    ) -> Self {
        Self {
            reader,
            writer,
            signing_key,

            remote_vkey,
            remote_locator,

            imports: DashSet::new(),
            exports: ExportManager::new(remote_vkey),
            aborted_by_remote: RwLock::default(),
            aborted_locally: false.into(),
        }
    }

    //#[tracing::instrument(skip(msg))]
    pub(super) fn send_msg<'fut, 'msg: 'fut>(
        &'fut self,
        msg: &'fut TokenTree<'msg>,
    ) -> impl std::future::Future<Output = Result<(), SendError>> + 'fut
    where
        Writer: AsyncWrite + Unpin,
    {
        async move {
            if self
                .aborted_locally
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                return Err(SendError::SessionAbortedLocally);
            }
            if let Some(reason) = self.aborted_by_remote.read().unwrap().as_ref() {
                return Err(SendError::SessionAborted(reason.clone()));
            }
            self.writer
                .lock()
                .await
                .write_all(&msg.encode())
                .await
                .map_err(SendError::from)
        }
    }

    //#[tracing::instrument]
    //async fn pop_tokens(&self) -> Result<TokenTree<'static>, RecvError>
    //where
    //    Reader: CapTpReadExt,
    //{
    //    self.reader
    //        .lock()
    //        .await
    //        .consume_syrup()
    //        .await
    //        .map_err(From::from)
    //}

    pub(super) async fn recv_msg<Msg>(&self) -> Result<Msg, RecvError>
    where
        Reader: CapTpReadExt + Send,
        Msg: Decode<'static>,
    {
        if self
            .aborted_locally
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return Err(RecvError::SessionAbortedLocally);
        }
        if let Some(reason) = self.aborted_by_remote.read().unwrap().as_ref() {
            return Err(RecvError::SessionAborted(reason.clone()));
        }
        self.reader
            .lock()
            .await
            .consume_syrup()
            .await?
            .decode::<Msg>()
            .map_err(From::from)
    }

    // pub(super) fn export(&self, val: Arc<dyn crate::captp::object::Object + Send + Sync>) -> u64 {
    // }

    pub(super) fn local_abort(&self) {
        self.aborted_locally
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn set_remote_abort(&self, reason: String) {
        *self.aborted_by_remote.write().unwrap() = Some(reason);
    }

    pub(super) fn is_aborted(&self) -> bool {
        self.aborted_locally
            .load(std::sync::atomic::Ordering::Acquire)
            || self.aborted_by_remote.read().unwrap().is_some()
    }

    // TODO :: propagate delivery errors
    pub(super) async fn recv_event(self: Arc<Self>) -> Result<super::Event, RecvError>
    where
        Reader: CapTpReadExt + Send + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        fn bootstrap_deliver_only<'args>(mut args: Sequence<'args>) -> super::Event {
            match args.stream.pop() {
                Some(TokenTree::Literal(Literal {
                    repr: LiteralValue::Symbol(ident),
                    ..
                })) => match &*ident {
                    b"deposit-gift" => todo!("bootstrap: deposit-gift"),
                    id => todo!(
                        "unrecognized bootstrap function: {}",
                        String::from_utf8_lossy(id)
                    ),
                },
                _ => todo!(),
            }
        }
        fn bootstrap_deliver<'args, Reader, Writer>(
            session: Arc<CapTpSessionInternal<Reader, Writer>>,
            mut args: Sequence<'args>,
            answer_pos: Option<u64>,
            resolve_me_desc: crate::captp::msg::DescImport,
        ) -> super::Event
        where
            Writer: AsyncWrite + Send + Unpin + 'static,
            Reader: Send + 'static,
        {
            match args.stream.pop() {
                Some(TokenTree::Literal(Literal {
                    repr: LiteralValue::Symbol(ident),
                    ..
                })) => match &*ident {
                    b"fetch" => {
                        let swiss = match args.stream.pop() {
                            Some(TokenTree::Literal(Literal {
                                repr: LiteralValue::Symbol(swiss),
                                ..
                            })) => swiss,
                            Some(s) => todo!("malformed swiss num: {s:?}"),
                            None => todo!("missing swiss num"),
                        };
                        super::Event::Bootstrap(crate::captp::BootstrapEvent::Fetch {
                            resolver: crate::captp::GenericResolver::new(
                                session,
                                answer_pos,
                                resolve_me_desc,
                            )
                            .into(),
                            swiss: swiss.into_owned(),
                        })
                    }
                    b"withdraw-gift" => todo!("bootstrap: withdraw-gift"),
                    id => todo!(
                        "unrecognized bootstrap function: {}",
                        String::from_utf8_lossy(id)
                    ),
                },
                _ => todo!(),
            }
        }
        loop {
            tracing::trace!("awaiting message");
            let msg = self
                .recv_msg::<crate::captp::msg::Operation<'static>>()
                .await?;
            tracing::debug!(?msg, "received message");
            match msg {
                Operation::DeliverOnly(del) => match del.to_desc.position {
                    0 => break Ok(bootstrap_deliver_only(del.args)),
                    pos => {
                        // let del = Delivery::DeliverOnly {
                        //     to_desc: del.to_desc,
                        //     args: del.args,
                        // };
                        // break Ok(Event::Delivery(del));
                        match self.exports.exports.get(&pos) {
                            Some(obj) => {
                                let span = tracing::info_span!("deliver_only");
                                let _guard = span.enter();
                                if let Err(error) = obj.deliver_only(self.clone(), del.args) {
                                    tracing::error!(pos, %error, "deliver_only");
                                }
                            }
                            None => break Err(RecvError::UnknownTarget(pos, del.args)),
                        }
                    }
                },
                Operation::Deliver(del) => match del.to_desc.position {
                    0 => {
                        break Ok(bootstrap_deliver(
                            self.clone(),
                            del.args,
                            del.answer_pos,
                            del.resolve_me_desc,
                        ))
                    }
                    pos => {
                        // let del = Delivery::Deliver {
                        //     to_desc: del.to_desc,
                        //     args: del.args,
                        //     resolver: GenericResolver {
                        //         session: self.clone(),
                        //         answer_pos: del.answer_pos,
                        //         resolve_me_desc: del.resolve_me_desc,
                        //     },
                        // };
                        // break Ok(Event::Delivery(del));
                        match self.exports.exports.get(&pos) {
                            Some(obj) => {
                                if let Err(error) = obj
                                    .deliver(
                                        self.clone(),
                                        del.args,
                                        crate::captp::GenericResolver::new(
                                            self.clone(),
                                            del.answer_pos,
                                            del.resolve_me_desc,
                                        ),
                                    )
                                    .instrument(tracing::info_span!("deliver").or_current())
                                    .await
                                {
                                    tracing::error!(pos, %error, "deliver");
                                }
                            }
                            None => break Err(RecvError::UnknownTarget(pos, del.args)),
                        }
                    }
                },
                Operation::Abort(crate::captp::msg::OpAbort { reason }) => {
                    self.set_remote_abort(reason.clone().into_owned());
                    break Ok(super::Event::Abort(reason.into_owned()));
                }
            }
        }
    }

    // fn gen_export(self: Arc<Self>) -> ObjectInbox<Socket> {
    //     let (sender, receiver) = futures::channel::mpsc::unbounded();
    //     let pos = self.exports.push(sender);
    //     ObjectInbox::new(pos, receiver)
    // }
}

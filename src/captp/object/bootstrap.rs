use std::sync::Arc;
use std::{borrow::Cow, future::Future};

use syrup::{call_sequence, sequence, Decode};

use super::{DeliverError, RemoteObject};
use crate::captp::msg::{DescHandoffReceive, DescImport};
use crate::captp::CapTpDeliver;
use crate::captp::{msg::DescExport, SendError};

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error(transparent)]
    Deliver(#[from] DeliverError<'static>),
    #[error(transparent)]
    Syrup(#[from] syrup::de::DecodeError<'static>),
}

pub trait Fetch: Sized {
    type Swiss<'swiss>;
    fn fetch<'swiss>(
        bootstrap: &RemoteBootstrap,
        swiss: Self::Swiss<'swiss>,
    ) -> impl Future<Output = Result<Self, FetchError>> + Send;
}

pub struct RemoteBootstrap {
    base: RemoteObject,
}

impl RemoteBootstrap {
    pub(crate) fn new(session: Arc<dyn CapTpDeliver + Send + Sync + 'static>) -> Self {
        Self {
            base: RemoteObject::new(session, 0.into()),
        }
    }
}

impl RemoteBootstrap {
    #[tracing::instrument(skip(self), fields(swiss_number = crate::hash(&swiss_number)))]
    pub async fn fetch(&self, swiss_number: &[u8]) -> Result<RemoteObject, FetchError> {
        tracing::trace!("fetching object");
        let mut args = self
            .base
            .deliver_and(call_sequence!["fetch", syrup::Bytes(swiss_number.into())])
            .await?;
        let session = self.base.session.clone();

        Ok(RemoteObject {
            position: args
                .stream
                .require(Cow::Borrowed("desc:export"))
                .and_then(DescExport::decode)?,
            session,
        })
    }

    #[tracing::instrument(skip(self), fields(swiss_number = crate::hash(&swiss_number)))]
    pub async fn fetch_to(
        &self,
        swiss_number: &[u8],
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), DeliverError<'static>> {
        tracing::trace!("fetching object");
        self.base
            .deliver(
                call_sequence!["fetch", syrup::Bytes(swiss_number.into())],
                answer_pos,
                resolve_me_desc,
            )
            .await
            .map_err(From::from)
    }

    pub fn fetch_with<'swiss, Obj: Fetch + 'swiss>(
        &'swiss self,
        swiss: Obj::Swiss<'swiss>,
    ) -> impl Future<Output = Result<Obj, FetchError>> + Send + 'swiss {
        Obj::fetch(self, swiss)
    }

    pub async fn deposit_gift(&self, gift_id: u64, desc: DescImport) -> Result<(), SendError> {
        self.base
            .deliver_only(call_sequence!["deposit_gift", gift_id, desc])
            .await
    }

    pub async fn withdraw_gift<'i>(
        self: Arc<Self>,
        handoff_receive: DescHandoffReceive<'i>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        self.base
            .deliver(
                call_sequence!["withdraw_gift", handoff_receive],
                answer_pos,
                resolve_me_desc,
            )
            .await
    }
}

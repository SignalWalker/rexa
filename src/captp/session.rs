use std::sync::Arc;

use ed25519_dalek::{SigningKey, VerifyingKey};
use syrup::Encode;

use super::{
    msg::{DescExport, OpAbort},
    object::{RemoteBootstrap, RemoteObject},
};
use crate::{
    async_compat::{AsyncRead, AsyncWrite},
    captp::{msg::DescImportObject, CapTpReadExt},
};

mod builder;
pub use builder::*;

mod core;
use core::*;

mod manager;
pub use manager::*;

mod error;
pub use error::*;

mod keymap;
pub(crate) use keymap::*;

mod registry;
pub use registry::*;

mod internal;
pub(crate) use internal::*;

mod resolver;
pub use resolver::*;

mod event;
pub use event::*;

mod traits;
pub use traits::*;

pub type RemoteKey = VerifyingKey;

pub struct CapTpSession<Reader, Writer> {
    base: Arc<CapTpSessionInternal<Reader, Writer>>,
}

impl<Reader, Writer> std::fmt::Debug for CapTpSession<Reader, Writer> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let key_hash = crate::hash(self.remote_vkey());
        f.debug_struct("CapTpSession")
            .field("remote_vkey", &key_hash)
            .finish_non_exhaustive()
    }
}

impl<Reader, Writer> std::clone::Clone for CapTpSession<Reader, Writer> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl<Reader, Writer> PartialEq for CapTpSession<Reader, Writer> {
    fn eq(&self, other: &Self) -> bool {
        self.remote_vkey() == other.remote_vkey() && self.signing_key() == other.signing_key()
    }
}

impl<Reader, Writer> From<Arc<CapTpSessionInternal<Reader, Writer>>>
    for CapTpSession<Reader, Writer>
{
    fn from(base: Arc<CapTpSessionInternal<Reader, Writer>>) -> Self {
        Self { base }
    }
}

impl<Reader, Writer> From<&'_ Arc<CapTpSessionInternal<Reader, Writer>>>
    for CapTpSession<Reader, Writer>
{
    fn from(base: &'_ Arc<CapTpSessionInternal<Reader, Writer>>) -> Self {
        Self { base: base.clone() }
    }
}

impl<Reader, Writer> CapTpSession<Reader, Writer> {
    pub fn as_dyn(&self) -> Arc<dyn AbstractCapTpSession + Send + Sync + 'static>
    where
        Reader: CapTpReadExt + Send + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        self.base.clone()
    }

    pub fn signing_key(&self) -> &SigningKey {
        &self.base.signing_key
    }

    pub fn remote_vkey(&self) -> &RemoteKey {
        &self.base.remote_vkey
    }

    pub fn export_object(&self, obj: impl IntoExport) -> DescImportObject {
        self.base.exports.export_object(obj)
    }

    pub fn is_aborted(&self) -> bool {
        self.base.is_aborted()
    }

    pub async fn abort<'reason>(&self, reason: impl Into<OpAbort<'reason>>) -> Result<(), SendError>
    where
        Writer: AsyncWrite + Unpin,
    {
        let res = self.base.send_msg(&reason.into().to_tokens()).await;
        self.base.local_abort();
        res
    }

    pub fn into_remote_object(self, position: DescExport) -> Option<RemoteObject>
    where
        Reader: Send + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        self.base.into_remote_object(position)
    }

    pub fn get_remote_bootstrap(self) -> RemoteBootstrap
    where
        Reader: Send + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        RemoteBootstrap::new(self.base.clone())
    }

    pub fn event_stream(&self) -> impl futures::stream::Stream<Item = Result<Event, RecvError>> + '_
    where
        Reader: CapTpReadExt + Send + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        futures::stream::unfold(self, |session| async move {
            Some((session.recv_event().await, session))
        })
    }

    pub fn into_event_stream(
        self,
    ) -> impl futures::stream::Stream<Item = Result<Event, RecvError>> + Unpin
    where
        Reader: CapTpReadExt + Send + 'static,
        Writer: AsyncWrite + Unpin + Send + 'static,
    {
        use futures::StreamExt;
        async fn recv<Reader, Writer>(
            session: CapTpSession<Reader, Writer>,
        ) -> Option<(Result<Event, RecvError>, CapTpSession<Reader, Writer>)>
        where
            Reader: CapTpReadExt + Send + 'static,
            Writer: AsyncWrite + Send + Unpin + 'static,
        {
            Some((session.recv_event().await, session))
        }
        futures::stream::unfold(self, recv).boxed()
    }

    // #[tracing::instrument()]
    pub async fn recv_event(&self) -> Result<Event, RecvError>
    where
        Reader: CapTpReadExt + Send + 'static,
        Writer: AsyncWrite + Send + Unpin + 'static,
    {
        self.base.clone().recv_event().await
    }
}

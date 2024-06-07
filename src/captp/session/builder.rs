use std::future::Future;

use ed25519_dalek::{SignatureError, Signer, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use syrup::{
    de::{DecodeError, LexError, LexErrorKind},
    Decode, Encode, TokenStream, TokenTree,
};

use super::CapTpSession;
use crate::{
    async_compat::{AsyncRead, AsyncWrite, AsyncWriteExt},
    captp::{
        msg::OpStartSession, session::CapTpSessionManager, CapTpRead, CapTpReadExt, ReadSyrupError,
    },
    locator::NodeLocator,
    CAPTP_VERSION,
};

#[derive(Debug, thiserror::Error)]
pub enum SessionInitError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Lex(#[from] LexError),
    #[error(transparent)]
    Decode(#[from] DecodeError<'static>),
    #[error("expected captp version {CAPTP_VERSION}, found {0}")]
    Version(String),
    #[error(transparent)]
    Signature(#[from] SignatureError),
}

impl<'i> From<ReadSyrupError> for SessionInitError {
    fn from(value: ReadSyrupError) -> Self {
        match value {
            ReadSyrupError::Io(io) => Self::Io(io),
            ReadSyrupError::Lex(lex) => Self::Lex(lex),
        }
    }
}

pub struct CapTpSessionBuilder<'manager, Reader, Writer> {
    manager: &'manager mut CapTpSessionManager<Reader, Writer>,
    reader: Reader,
    writer: Writer,
    signing_key: SigningKey,
    // registry: Option<Arc<super::SwissRegistry<Socket>>>,
}

impl<'m, Reader, Writer> CapTpSessionBuilder<'m, Reader, Writer> {
    pub fn new(
        manager: &'m mut CapTpSessionManager<Reader, Writer>,
        reader: Reader,
        writer: Writer,
    ) -> Self {
        Self {
            manager,
            reader,
            writer,
            signing_key: SigningKey::generate(&mut OsRng),
            // registry: None,
        }
    }

    // pub fn with_registry(mut self, registry: Option<Arc<super::SwissRegistry<Socket>>>) -> Self {
    //     self.registry = registry;
    //     self
    // }

    pub fn and_accept<'locator>(
        mut self,
        local_locator: NodeLocator<'locator>,
    ) -> impl Future<Output = Result<CapTpSession<Reader, Writer>, SessionInitError>> + 'm
    where
        Reader: CapTpReadExt + Send,
        Writer: AsyncWrite + Unpin,
    {
        let start_msg = self
            .generate_start_msg(local_locator)
            .to_tokens()
            .encode()
            .into_owned();

        async move {
            let (remote_vkey, remote_loc) = self.recv_start_session().await?;

            self.writer.write_all(&start_msg).await?;
            self.writer.flush().await?;

            Ok(self.manager.finalize_session(
                self.reader,
                self.writer,
                self.signing_key,
                remote_vkey,
                remote_loc,
            ))
        }
    }

    pub fn and_connect<'locator>(
        mut self,
        local_locator: NodeLocator<'locator>,
    ) -> impl Future<Output = Result<CapTpSession<Reader, Writer>, SessionInitError>> + 'm
    where
        Reader: CapTpReadExt + Send,
        Writer: AsyncWrite + Unpin,
    {
        let local_designator = local_locator.designator.clone().into_owned();
        tracing::debug!(local = %local_designator, "connecting with OpStartSession");

        let start_msg = self
            .generate_start_msg(local_locator)
            .to_tokens()
            .encode()
            .into_owned();

        async move {
            self.writer.write_all(&start_msg).await?;
            self.writer.flush().await?;

            tracing::debug!(local = %local_designator, "sent OpStartSession, receiving response");

            let (remote_vkey, remote_loc) = self.recv_start_session().await?;

            Ok(self.manager.finalize_session(
                self.reader,
                self.writer,
                self.signing_key,
                remote_vkey,
                remote_loc,
            ))
        }
    }

    fn generate_start_msg<'locator>(
        &self,
        local_locator: NodeLocator<'locator>,
    ) -> OpStartSession<'locator> {
        let location_sig = self
            .signing_key
            .sign(&(&local_locator).to_tokens().encode());
        OpStartSession::new(
            self.signing_key.verifying_key().into(),
            local_locator,
            location_sig.into(),
        )
    }

    pub(super) async fn recv_start_session(
        &mut self,
    ) -> Result<(VerifyingKey, NodeLocator<'static>), SessionInitError>
    where
        Reader: CapTpReadExt + Send,
    {
        let response = self
            .reader
            .consume_syrup()
            .await?
            .decode::<OpStartSession<'static>>()?;

        if response.captp_version != CAPTP_VERSION {
            return Err(SessionInitError::Version(
                response.captp_version.into_owned(),
            ));
        }

        response.verify_location()?;

        let res = (response.session_pubkey.ecc, response.acceptable_location);

        Ok(res)
    }
}

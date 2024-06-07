use std::sync::Arc;

use syrup::{literal, sequence, symbol, Encode, Sequence, TokenTree};

use super::CapTpDeliver;
use crate::captp::{
    msg::{DescExport, DescImport, DescImportObject, OpDeliver, OpDeliverOnly},
    object::DeliverError,
    SendError,
};

#[must_use]
#[derive(Clone)]
pub struct GenericResolver {
    session: std::sync::Arc<dyn CapTpDeliver + Send + Sync>,
    answer_pos: Option<u64>,
    resolve_me_desc: DescImport,
    #[cfg(feature = "extra-diagnostics")]
    resolved: bool,
}

#[cfg(feature = "extra-diagnostics")]
impl Drop for GenericResolver {
    fn drop(&mut self) {
        if !self.resolved {
            tracing::warn!(resolver = ?self, "dropping unresolved resolver");
        }
    }
}

impl std::fmt::Debug for GenericResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericResolver")
            // .field("session", &self.session)
            .field("answer_pos", &self.answer_pos)
            .field("resolve_me_desc", &self.resolve_me_desc)
            .finish()
    }
}

impl GenericResolver {
    pub(super) fn new(
        session: Arc<dyn CapTpDeliver + Send + Sync>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Self {
        Self {
            session,
            answer_pos,
            resolve_me_desc,
            #[cfg(feature = "extra-diagnostics")]
            resolved: false,
        }
    }

    fn position(&self) -> DescExport {
        use crate::captp::msg::DescImportPromise;
        match self.resolve_me_desc {
            DescImport::Object(DescImportObject { position })
            | DescImport::Promise(DescImportPromise { position }) => position.into(),
        }
    }

    pub async fn fulfill<'args>(
        mut self,
        mut args: Sequence<'args>,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }

        args.stream.insert(0, literal![Symbol; b"fulfill"]);

        self.session
            .deliver(&OpDeliver::new(
                self.position(),
                args,
                answer_pos,
                resolve_me_desc,
            ))
            .await
    }

    pub async fn fulfill_and<'args>(
        mut self,
        mut args: Sequence<'args>,
    ) -> Result<Sequence<'static>, DeliverError<'static>> {
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }

        args.stream.insert(0, literal![Symbol; b"fulfill"]);

        self.session.deliver_and(self.position(), args).await
    }

    pub async fn break_promise<'error>(
        mut self,
        error: TokenTree<'error>,
    ) -> Result<(), SendError> {
        #[cfg(feature = "extra-diagnostics")]
        {
            self.resolved = true;
        }
        self.session
            .deliver_only(&OpDeliverOnly::new(
                self.position(),
                sequence![symbol!["break"], error],
            ))
            .await
    }
}

#[must_use]
pub struct FetchResolver {
    base: GenericResolver,
}

impl std::fmt::Debug for FetchResolver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FetchResolver")
            .field("base", &self.base)
            .finish()
    }
}

impl std::clone::Clone for FetchResolver {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
        }
    }
}

impl FetchResolver {
    pub async fn fulfill(
        self,
        position: DescExport,
        answer_pos: Option<u64>,
        resolve_me_desc: DescImport,
    ) -> Result<(), SendError> {
        self.base
            .fulfill(sequence![position], answer_pos, resolve_me_desc)
            .await
    }

    pub async fn fulfill_and(
        self,
        position: DescExport,
    ) -> Result<Sequence<'static>, DeliverError<'static>> {
        self.base.fulfill_and(sequence![position]).await
    }

    pub async fn break_promise<'error>(self, error: TokenTree<'error>) -> Result<(), SendError> {
        self.base.break_promise(error).await
    }
}

impl From<GenericResolver> for FetchResolver {
    fn from(base: GenericResolver) -> Self {
        Self { base }
    }
}

use syrup::{Decode, Encode};

mod start_session;
pub use start_session::*;

mod abort;
pub use abort::*;

mod import_export {
    use syrup::{Decode, Encode, Symbol};

    #[derive(Clone, Copy, Encode, Decode)]
    #[syrup(label = "desc:export")]
    pub struct DescExport {
        pub position: u64,
    }

    impl From<u64> for DescExport {
        fn from(position: u64) -> Self {
            Self { position }
        }
    }

    impl std::fmt::Debug for DescExport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.to_tokens().fmt(f)
        }
    }

    #[derive(Clone, Copy, Encode, Decode)]
    #[syrup(label = "desc:import-object")]
    pub struct DescImportObject {
        pub position: u64,
    }

    impl std::fmt::Debug for DescImportObject {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.to_tokens().fmt(f)
        }
    }

    impl From<DescExport> for DescImportObject {
        fn from(position: DescExport) -> Self {
            Self {
                position: position.position,
            }
        }
    }

    #[derive(Clone, Copy, Encode, Decode)]
    #[syrup(label = "desc:import-promise")]
    pub struct DescImportPromise {
        pub position: u64,
    }

    impl std::fmt::Debug for DescImportPromise {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.to_tokens().fmt(f)
        }
    }

    impl From<u64> for DescImportPromise {
        fn from(position: u64) -> Self {
            Self { position }
        }
    }

    #[derive(Clone, Copy, Decode, Encode)]
    #[syrup(transparent)]
    pub enum DescImport {
        Object(DescImportObject),
        Promise(DescImportPromise),
    }

    impl Default for DescImport {
        fn default() -> Self {
            Self::Object(DescImportObject { position: 0 })
        }
    }

    impl std::fmt::Debug for DescImport {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                DescImport::Object(o) => o.fmt(f),
                DescImport::Promise(p) => p.fmt(f),
            }
        }
    }

    impl From<DescImportObject> for DescImport {
        fn from(value: DescImportObject) -> Self {
            Self::Object(value)
        }
    }

    impl From<DescImportPromise> for DescImport {
        fn from(value: DescImportPromise) -> Self {
            Self::Promise(value)
        }
    }
}
pub use import_export::*;

mod deliver {
    use super::{DescExport, DescImport};
    use syrup::{de::Sequence, Decode, Encode};

    #[derive(Clone, Encode, Decode)]
    #[syrup(label = "op:deliver-only")]
    pub struct OpDeliverOnly<'arg> {
        pub to_desc: DescExport,
        pub args: Sequence<'arg>,
    }

    impl<'arg> std::fmt::Debug for OpDeliverOnly<'arg> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.to_tokens().fmt(f)
        }
    }

    impl<'arg> OpDeliverOnly<'arg> {
        pub const fn new(to_desc: DescExport, args: Sequence<'arg>) -> Self {
            Self { to_desc, args }
        }
    }

    #[derive(Clone, Encode, Decode)]
    #[syrup(label = "op:deliver")]
    pub struct OpDeliver<'arg> {
        pub to_desc: DescExport,
        pub args: Sequence<'arg>,
        pub answer_pos: Option<u64>,
        pub resolve_me_desc: DescImport,
    }

    impl<'arg> std::fmt::Debug for OpDeliver<'arg> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.to_tokens().fmt(f)
        }
    }

    impl<'arg> OpDeliver<'arg> {
        pub const fn new(
            to_desc: DescExport,
            args: Sequence<'arg>,
            answer_pos: Option<u64>,
            resolve_me_desc: DescImport,
        ) -> Self {
            Self {
                to_desc,
                args,
                answer_pos,
                resolve_me_desc,
            }
        }
    }
}
pub use deliver::*;

mod handoff;
pub use handoff::*;

/// Used for [`CapTpSession::recv_event`](CapTpSession::recv_event).
#[derive(Clone, Encode, Decode)]
#[syrup(transparent)]
pub(super) enum Operation<'inner> {
    DeliverOnly(OpDeliverOnly<'inner>),
    Deliver(OpDeliver<'inner>),
    // Pick(OpPick),
    Abort(OpAbort<'inner>),
    // Listen(OpListen),
    // GcExport(OpGcExport),
    // GcAnswer(OpGcAnswer),
}

impl<'i> std::fmt::Debug for Operation<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

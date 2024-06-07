use syrup::{
    de::{DecodeError, LexError},
    Sequence,
};

use crate::captp::ReadSyrupError;

#[derive(Debug, thiserror::Error)]
pub enum RecvError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Lex(#[from] LexError),
    #[error(transparent)]
    Decode(#[from] DecodeError<'static>),
    #[error("received abort message from remote; reason: {0}")]
    SessionAborted(String),
    #[error("attempted recv on locally aborted session")]
    SessionAbortedLocally,
    #[error("unknown delivery target: {0}, args: {1:?}")]
    UnknownTarget(u64, Sequence<'static>),
}

impl From<ReadSyrupError> for RecvError {
    fn from(value: ReadSyrupError) -> Self {
        match value {
            ReadSyrupError::Io(io) => Self::Io(io),
            ReadSyrupError::Lex(lex) => Self::Lex(lex),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SendError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("received abort message from remote; reason: {0}")]
    SessionAborted(String),
    #[error("attempted send on locally aborted session")]
    SessionAbortedLocally,
}

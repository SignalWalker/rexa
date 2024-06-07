use std::borrow::Cow;

use syrup::{Decode, Encode};

#[derive(Clone, Encode, Decode)]
#[syrup(label = "op:abort")]
pub struct OpAbort<'reason> {
    pub reason: Cow<'reason, str>,
}

impl<'r> std::fmt::Debug for OpAbort<'r> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

impl<'r, S: Into<Cow<'r, str>>> From<S> for OpAbort<'r> {
    #[inline]
    fn from(reason: S) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

use std::borrow::Cow;

use super::PublicKey;
use crate::locator::NodeLocator;
use syrup::{Decode, Encode};

#[derive(Clone, Decode, Encode)]
#[syrup(label = "desc:handoff-give")]
pub struct DescHandoffGive<'input> {
    pub receiver_key: PublicKey,
    pub exporter_location: NodeLocator<'input>,
    pub session: Cow<'input, [u8]>,
    pub gifter_side: Cow<'input, [u8]>,
    pub gift_id: Cow<'input, [u8]>,
}

impl<'i> std::fmt::Debug for DescHandoffGive<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

#[derive(Clone, Decode, Encode)]
#[syrup(label = "desc:handoff-receive")]
pub struct DescHandoffReceive<'input> {
    pub receiving_session: Cow<'input, [u8]>,
    pub receiving_side: Cow<'input, [u8]>,
    pub handoff_count: u64,
    pub signed_give: DescHandoffGive<'input>,
}

impl<'i> std::fmt::Debug for DescHandoffReceive<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

use std::borrow::Cow;

use crate::{locator::NodeLocator, CAPTP_VERSION};
use ed25519_dalek::{SignatureError, VerifyingKey};
use syrup::{Decode, Encode};

#[derive(Clone, Encode, Decode)]
#[syrup(label = "public-key")]
pub struct PublicKey {
    #[syrup(with = syrup_ed25519::verifying_key)]
    pub ecc: VerifyingKey,
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

impl From<VerifyingKey> for PublicKey {
    fn from(value: VerifyingKey) -> Self {
        Self { ecc: value }
    }
}

#[derive(Clone, Encode, Decode)]
#[syrup(label = "sig-val")]
pub struct Signature {
    #[syrup(with = syrup_ed25519::signature)]
    pub eddsa: ed25519_dalek::Signature,
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

impl From<ed25519_dalek::Signature> for Signature {
    fn from(value: ed25519_dalek::Signature) -> Self {
        Self { eddsa: value }
    }
}

#[derive(Clone, Encode, Decode)]
#[syrup(label = "op:start-session")]
pub struct OpStartSession<'input> {
    pub captp_version: Cow<'input, str>,
    pub session_pubkey: PublicKey,
    pub acceptable_location: NodeLocator<'input>,
    pub acceptable_location_sig: Signature,
}

impl<'i> std::fmt::Debug for OpStartSession<'i> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_tokens().fmt(f)
    }
}

impl<'i> OpStartSession<'i> {
    pub fn new(
        session_pubkey: PublicKey,
        acceptable_location: NodeLocator<'i>,
        acceptable_location_sig: Signature,
    ) -> Self {
        Self {
            captp_version: CAPTP_VERSION.into(),
            session_pubkey,
            acceptable_location,
            acceptable_location_sig,
        }
    }

    pub fn verify_location(&self) -> Result<(), SignatureError> {
        self.session_pubkey.ecc.verify_strict(
            &(&self.acceptable_location).to_tokens().encode(),
            &self.acceptable_location_sig.eddsa,
        )
    }
}

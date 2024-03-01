use crate::{locator::NodeLocator, CAPTP_VERSION};
use ed25519_dalek::{SignatureError, VerifyingKey};
use syrup::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[syrup(name = "public-key")]
pub struct PublicKey {
    pub ecc: VerifyingKey,
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
    }
}

impl From<VerifyingKey> for PublicKey {
    fn from(value: VerifyingKey) -> Self {
        Self { ecc: value }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[syrup(name = "sig-val")]
pub struct Signature {
    pub eddsa: ed25519_dalek::Signature,
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
    }
}

impl From<ed25519_dalek::Signature> for Signature {
    fn from(value: ed25519_dalek::Signature) -> Self {
        Self { eddsa: value }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[syrup(name = "op:start-session", deserialize_bound = LocatorHKey: PartialEq + Eq + std::hash::Hash + Deserialize<'__de>; LocatorHVal: Deserialize<'__de>)]
pub struct OpStartSession<LocatorHKey, LocatorHVal> {
    pub captp_version: String,
    pub session_pubkey: PublicKey,
    pub acceptable_location: NodeLocator<LocatorHKey, LocatorHVal>,
    pub acceptable_location_sig: Signature,
}

impl<HKey, HVal> std::fmt::Debug for OpStartSession<HKey, HVal>
where
    Self: syrup::Serialize,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&syrup::ser::to_pretty(self).unwrap())
    }
}

impl<HKey, HVal> OpStartSession<HKey, HVal> {
    pub fn new(
        session_pubkey: PublicKey,
        acceptable_location: NodeLocator<HKey, HVal>,
        acceptable_location_sig: Signature,
    ) -> Self {
        Self {
            captp_version: CAPTP_VERSION.to_owned(),
            session_pubkey,
            acceptable_location,
            acceptable_location_sig,
        }
    }

    pub fn verify_location(&self) -> Result<(), SignatureError>
    where
        HKey: Serialize,
        HVal: Serialize,
    {
        self.session_pubkey.ecc.verify_strict(
            &syrup::ser::to_bytes(&self.acceptable_location).unwrap(),
            &self.acceptable_location_sig.eddsa,
        )
    }
}

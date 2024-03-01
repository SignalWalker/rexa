/// Secure messaging protocol
pub mod captp;
/// Representation of object references
pub mod locator;
/// Secure communication channels between sessions
pub mod netlayer;

pub use syrup;

pub mod async_compat;

pub const CAPTP_VERSION: &'static str = "1.0";

// FIX :: Remove this
#[doc(hidden)]
pub fn hash(h: &impl std::hash::Hash) -> u64 {
    use std::hash::{DefaultHasher, Hasher};
    let mut hasher = DefaultHasher::new();
    h.hash(&mut hasher);
    hasher.finish()
}

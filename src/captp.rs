//! - [Draft Specification](<https://github.com/ocapn/ocapn/blob/main/draft-specifications/CapTP Specification.md>)

pub mod msg;

mod session;
pub use session::*;

pub mod object;

mod io;
pub use io::*;

//! The central Reader of this crate.
//!
//! The reader [Stream] wraps any types implementing [std::io::Read] and an extra [Index].
//! [Stream] can be used and passed as common reader. When consumed, the [Stream] is gone and the extra [Index] can be used.
//!
//! If you want to read and query locations simutaneously, [Index] can be reborrowed from [Stream].
mod alias;
mod stream;

pub use alias::{Guard, IndexRef, MutGuard};
pub use stream::Stream;

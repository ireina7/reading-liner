//! This crate is for Reading and converting offset and line-column location while reading butes.  
//!
//! This crate supports a [Stream] reader which can convert between byte offset and line-column numbers.
//! Support any type which implements [std::io::Read].
//!
//! The whole design is based on an [Index],
//! which is composed of line information to convert between byte offsets and line-column locations.
//! One perticular usage is to use the [Stream] as a builder of [Index] or
//! you can also use it when lazily reading and convert locations at the same time.
//!
//! This lib should be used at *low-level abstraction*.
//! For detailed examples, please refer to
//! [README](https://github.com/ireina7/reading-liner/blob/main/README.md)

pub mod index;
pub mod location;
pub mod stream;

pub use index::{Index, Query};
pub use stream::Stream;

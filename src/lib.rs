//! # hxcvtr-file-cache
//!
//! This crate is a component of the Hxcvtr core engine.
//!
//! `hxcvtr-file-cache` provides three cache implementations for some
//! source in memory, where the primarily intended source is `std::fs::File`.
//! However, anything that implements `std::io::Read` and `std::io::Seek` can
//! be used as a source. Cache allows for faster, more efficient access to
//! data stored on a slow external source, such as a file on disk. Depending
//! on the cache type and configuration, part or all of the data will be
//! stored in memory. Cache is read-only. See the documentation for each
//! cache type for implementation details and use cases.
//!
//! This crate additionally provides the `CacheReader` type, which wraps a
//! cache and implements `std::io::Read` and `std::io::Seek`.

mod auto_cache;
mod full_cache;
mod swap_cache;
mod cache_reader;

#[cfg(test)]
mod tests;

pub use auto_cache::AutoCache;
pub use full_cache::FullCache;
pub use swap_cache::SwapCache;
pub use cache_reader::CacheReader;

use std::io::{Read, Seek};
use std::ops::RangeBounds;

#[derive(Debug)]
/// Error type for `hxcvtr-file-cache`
///
/// Errors can be either an IO error, a mutex poison error, or a zero cache error.
pub enum Error {
    /// Error emitted by `std::io::Read::read` or `std::io::Seek::seek`. These errors
    /// indicate that a problem was encountered reading the cache source. See the
    /// standard library documentation for more information.
    IO(std::io::Error),

    /// Error emitted by `std::sync::Mutex::lock`. Swap cache provides thread safe
    /// interior mutability by wrapping its primary functionality within a mutex.
    /// This error should only occur when the user passes a closure to
    /// `Cache::traverse_chunks` that panics.
    Poison(std::string::String),

    /// This error indicates that the cache was configured to have no cache
    /// memory. This will happen when `SwapCache` is constructed with zero bytes
    /// per page or zero frames, or `AutoCache` is constructed with zero maximum
    /// memory.
    ZeroCache(&'static str),
}

impl Error {
    fn from_io(e: std::io::Error) -> Self {
        Error::IO(e)
    }

    fn new_zero_cache(msg: &'static str) -> Self {
        Error::ZeroCache(msg)
    }

    fn from_poison<T>(e: std::sync::PoisonError<T>) -> Self {
        use std::error::Error;
        self::Error::Poison(std::string::String::from(e.description()))
    }

    /// Returns true if the error is an IO error, false otherwise.
    pub fn is_io_error(&self) -> bool {
        match self {
            Error::IO(_) => true,
            _ => false,
        }
    }

    /// Returns true if the error is a poison error, false otherwise.
    pub fn is_poison_error(&self) -> bool {
        match self {
            Error::Poison(_) => true,
            _ => false,
        }
    }

    /// Returns true if the error is a zero cache error, false otherwise.
    pub fn is_zero_cache_error(&self) -> bool {
        match self {
            Error::ZeroCache(_) => true,
            _ => false,
        }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::IO(e) => e.fmt(f),
            Error::Poison(msg) => write!(f, "Poison Error: {}", msg),
            Error::ZeroCache(msg) => write!(f, "Zero Cache Error: {}", msg),
        }
    }

}

/// A `std::result::Result` with `hxcvtr_file_cache::Error` as the error type.
pub type Result<T> = std::result::Result<T, Error>;

/// The common interface for the cache types in this crate.
pub trait Cache {
    /// The type of the source that is begin cached.
    type Source: Read + Seek;

    /// Destroys the cache and returns the contained source.
    fn into_inner(self) -> Result<Self::Source>;

    /// Returns the length of underlying source in bytes.
    fn len(&self) -> u64;

    /// Returns the amount of cache memory allocated in bytes. This is the
    /// amount of data from the source that is cached in memory at any given
    /// time, and does not include memory potentially allocated for cache
    /// management.
    fn cache_size(&self) -> usize;

    /// Calls a closure on a series of memory chunks that cover the passed
    /// range, where the range represents the start and end byte offsets
    /// into the source. Chunks passed to the closure are guaranteed to be
    /// entirely contained within the range, and are processed in ascending
    /// byte offset order. A range that reaches beyond the end of the source
    /// is valid, but traversal ends when the end of the source is reached.
    fn traverse_chunks<R: RangeBounds<u64>, F: FnMut(&[u8]) -> Result<()>>(&self, range: R, f: F) -> Result<()>;

    /// Fills a buffer with data from the source starting at the passed byte
    /// offset. Returns the number of bytes read into the buffer. The returned
    /// size will be less than the size of the buffer if the end of the source
    /// is reached before filling the buffer.
    fn read(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        use std::io::Write;
        let mut total = 0;
        self.traverse_chunks(offset..buffer.len() as u64, |chunk| {
            total += match (&mut buffer[total..]).write(chunk) {
                Ok(len) => len,
                Err(e) => return Err(Error::from_io(e)),
            };
            Ok(())
        })?;
        Ok(total)
    }
}

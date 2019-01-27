mod auto_cache;
mod full_cache;
mod swap_cache;

pub use self::auto_cache::AutoCache;
pub use self::full_cache::FullCache;
pub use self::swap_cache::SwapCache;

use std::io::{Read, Seek};
use std::ops::Range;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Poison(std::string::String),
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

    pub fn is_io_error(&self) -> bool {
        match self {
            Error::IO(_) => true,
            _ => false,
        }
    }

    pub fn is_poison_error(&self) -> bool {
        match self {
            Error::Poison(_) => true,
            _ => false,
        }
    }

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
        use std::fmt::Display;
        match self {
            Error::IO(e) => e.fmt(f),
            Error::Poison(msg) => write!(f, "Poison Error: {}", msg),
            Error::ZeroCache(msg) => write!(f, "Zero Cache Error: {}", msg),
        }
    }

}

pub type Result<T> = std::result::Result<T, Error>;

pub trait Cache {
    type Input: Read + Seek;

    fn into_inner(self) -> Result<Self::Input>;
    fn len(&self) -> u64;
    fn traverse_chunks<F: FnMut(&[u8]) -> Result<()>>(&self, range: Range<u64>, f: F) -> Result<()>;
}

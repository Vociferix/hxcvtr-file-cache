use super::{Cache, Error};
use std::io::{Read, Seek, SeekFrom};

/// Wrapper for `Cache` types that implements `std::io::Read` and `std::io::Seek`.
///
/// `CacheReader` has two main purposes:
///
/// * Allow the user to treat the cache the same as the original source, in
/// that the source also implements `std::io::Read` and `std::io::Seek`. This may
/// be useful when the user wants a drop in replacement, without the need for to
/// specialize code for the cache.
///
/// * Allow caches to be layered. One cache could contain another cache which
/// contains the original source. In theory, this could potentially improve
/// performance in certain use cases, due to CPU cache locality. Use caution and
/// be sure to benchmark your use case thoroughly when using this method, because
/// while it is possible for this to improve performance, there is also a strong
/// possibility that performance will be worse due to the added complexity. Also,
/// this should only be done with `SwapCache` as the cache type for each layer.
/// `FullCache` reads the entire source into memory, which defeats the purpose
/// of a layered cache.
pub struct CacheReader<C: Cache> {
    pos: u64,
    cache: C,
}

impl<C: Cache> CacheReader<C> {
    /// Creates a new `CacheReader` containing the passed cache, and has
    /// its position initialized to byte zero.
    pub fn new(cache: C) -> Self {
        Self {
            pos: 0,
            cache,
        }
    }

    /// Destroys a `CacheReader` and returns the contained cache.
    pub fn into_inner(self) -> C {
        self.cache
    }

    /// Returns an immutable reference to the contained cache.
    pub fn cache(&self) -> &C {
        &self.cache
    }

    /// Returns the position of the reader as a byte offset from the
    /// beginning of the source.
    pub fn position(&self) -> u64 {
        self.pos
    }
}

impl<C: Cache> Read for CacheReader<C> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let len = match self.cache.read(self.pos, buf) {
            Ok(len) => len as usize,
            Err(e) => {
                let is_io_error = e.is_io_error();
                if is_io_error {
                    match e {
                        Error::IO(e) => return Err(e),
                        _ => unreachable!(),
                    }
                } else {
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
                }
            }
        };
        self.pos += len as u64;
        Ok(len)
    }
}

impl<C: Cache> Seek for CacheReader<C> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let len = self.cache.len();
        self.pos = match pos {
            SeekFrom::Current(offset) => {
                if offset < 0 {
                    let offset = -offset as u64;
                    let pos = self.pos;
                    if offset > pos {
                        0
                    } else {
                        pos - offset
                    }
                } else {
                    let offset = self.pos + offset as u64;
                    if offset > len {
                        len
                    } else {
                        offset
                    }
                }
            },
            SeekFrom::End(offset) => {
                if offset < 0 {
                    let offset = -offset as u64;
                    if offset > len {
                        0
                    } else {
                        len - offset
                    }
                } else {
                    len
                }
            },
            SeekFrom::Start(offset) => {
                if offset > len {
                    len
                } else {
                    offset
                }
            },
        };
        Ok(self.pos)
    }
}
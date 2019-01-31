use super::Cache;
use std::io::{Read, Seek, SeekFrom};
use std::ops::{Bound, RangeBounds};

use super::{TraversalCode, Error, Result};

/// A simple cache that reads the entire source into contiguous memory.
///
/// `FullCache` reads the entire source into a buffer on creation and
/// never accesses the source again. Since the data is all in one
/// contiguous chunk of memory, all calls to `Cache::traverse_chunks`
/// on this type will result in at most a single chunk.
///
/// On its own, this cache type is relatively useless since it is
/// simple enough to read a file or other external source into a
/// buffer without the help of a crate. The purpose of `FullCache` is
/// primarily to be a variant of `AutoCache`. See the documentation
/// for `AutoCache` for more details.
pub struct FullCache<T: Read + Seek> {
    source: T,
    data: Vec<u8>,
}

impl<T: Read + Seek> FullCache<T> {
    /// Creates a new `FullCache` containing the passed source.
    pub fn new(source: T) -> Result<Self> {
        let mut source = source;
        let mut data = Vec::new();
        match source.seek(SeekFrom::Start(0)) {
            Err(e) => Err(Error::from_io(e)),
            _ => match source.read_to_end(&mut data) {
                Ok(_) => Ok(FullCache { source, data }),
                Err(e) => Err(Error::from_io(e)),
            },
        }
    }
}

impl<T: Read + Seek> Cache for FullCache<T> {
    type Source = T;

    fn into_inner(self) -> Result<T> {
        let mut source = self.source;
        match source.seek(SeekFrom::Start(0)) {
            Err(e) => Err(Error::from_io(e)),
            _ => Ok(source),
        }
    }

    fn len(&self) -> u64 {
        self.data.len() as u64
    }

    fn cache_size(&self) -> usize {
        self.data.len()
    }

    fn traverse_chunks<R: RangeBounds<u64>, F: FnMut(&[u8]) -> TraversalCode>(
        &self,
        range: R,
        f: F,
    ) -> Result<()> {
        let mut f = f;
        let len = self.data.len() as u64;
        let start = match range.start_bound() {
            Bound::Included(start) => {
                if *start >= len {
                    return Ok(());
                } else {
                    *start
                }
            }
            Bound::Excluded(start) => {
                let start = *start + 1;
                if start > len {
                    return Ok(());
                } else {
                    start
                }
            }
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(end) => {
                if *end >= len {
                    len
                } else {
                    *end + 1
                }
            }
            Bound::Excluded(end) => {
                if *end > len {
                    len
                } else {
                    *end
                }
            }
            Bound::Unbounded => len,
        };
        let _ = f(&self.data[start as usize..end as usize]);
        Ok(())
    }
}

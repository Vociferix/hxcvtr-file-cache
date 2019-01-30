use super::{Cache, FullCache, SwapCache};

use std::io::{Read, Seek, SeekFrom};
use std::ops::RangeBounds;

/// A cache that internally uses `FullCache` or `SwapCache` depending on source size.
///
/// `AutoCache` attempts to use the most appropriate cache type based on
/// a maximum memory usage and the size of the source. If the source is
/// larger than the requested memory usage, then `SwapCache` will be used,
/// otherwise `FullCache` will be used. `FullCache` is the more optimal
/// cache type since the source only needs to be accessed once for the
/// life of the cache, so `AutoCache` uses `FullCache` when possible.
/// When a `SwapCache` needs to be used, page size and frame count are
/// chosen to be the largest possible without exceeding the maximum memory
/// usage.
///
/// Generally, `AutoCache` is the cache type from this crate intended to
/// be used directly by users, even though all three cache types are public.
/// A cache allows more optimal random access to a file or other source,
/// especially when the file might be too large to simply read into memory.
/// The Hxcvtr core engine uses `AutoCache` to support working with very
/// large files.
pub enum AutoCache<T: Read + Seek> {
    Full(FullCache<T>),
    Swap(SwapCache<T>),
}

use self::AutoCache::Full;
use self::AutoCache::Swap;

use super::{Result, Error};

fn sqrt(n: usize) -> usize {
    let mut shift: isize = 2;
    let mut shifted = n >> shift;
    while shifted != 0 && shifted != n {
        shift += 2;
        shifted = n >> shift;
    }
    shift -= 2;
    let mut ret = 0;
    while shift >= 0 {
        ret <<= 1;
        let candidate = ret + 1;
        if candidate * candidate <= (n >> shift) {
            ret = candidate;
        }
        shift -= 2;
    }
    ret
}

impl<T: Read + Seek> AutoCache<T> {
    /// Creates a new `AutoCache` containing the passed source and with the passed maximum
    /// memory usage.
    pub fn new(source: T, mem_max: usize) -> Result<Self> {
        if mem_max == 0 {
            return Err(Error::new_zero_cache("AutoCache configured with no memory"));
        }
        let mut source = source;
        let len = match source.seek(SeekFrom::End(0)) {
            Ok(len) => len,
            Err(e) => return Err(Error::from_io(e)),
        };
        if len > mem_max as u64 {
            let page_sz = sqrt(mem_max);
            let mut frame_count = page_sz + 1;
            if page_sz * frame_count > mem_max {
                frame_count = page_sz;
            }
            Ok(Swap(SwapCache::new(source, page_sz, frame_count)?))
        } else {
            Ok(Full(FullCache::new(source)?))
        }
    }
}

impl<T: Read + Seek> Cache for AutoCache<T> {
    type Source = T;

    fn into_inner(self) -> Result<T> {
        match self {
            Full(full) => FullCache::into_inner(full),
            Swap(swap) => SwapCache::into_inner(swap),
        }
    }

    fn len(&self) -> u64 {
        match self {
            Full(ref full) => full.len(),
            Swap(ref swap) => swap.len(),
        }
    }

    fn cache_size(&self) -> usize {
        match self {
            Full(ref full) => full.cache_size(),
            Swap(ref swap) => swap.cache_size(),
        }
    }

    fn traverse_chunks<R: RangeBounds<u64>, F: FnMut(&[u8]) -> Result<()>>(&self, range: R, f: F) -> Result<()> {
        match self {
            Full(ref full) => full.traverse_chunks(range, f),
            Swap(ref swap) => swap.traverse_chunks(range, f),
        }
    }
}

use super::{Cache, FullCache, SwapCache};

use std::io::{Read, Seek, SeekFrom};
use std::ops::RangeBounds;

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
    type Input = T;

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

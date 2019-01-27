use super::{Cache, FullCache, SwapCache};

use std::io::{Read, Seek};
use std::ops::Range;

pub enum AutoCache<T: Read + Seek> {
    Full(FullCache<T>),
    Swap(SwapCache<T>),
}

use self::AutoCache::Full;
use self::AutoCache::Swap;

use super::{Result, Error};

impl AutoCache<std::fs::File> {
    pub fn from_file(source: std::fs::File, mem_max: usize) -> Result<Self> {
        let len;
        match source.metadata() {
            Ok(meta) => {
                len = meta.len();
            }
            Err(e) => {
                return Err(Error::from_io(e));
            }
        }
        AutoCache::new(source, len, mem_max)
    }
}

impl<T: Read + Seek> AutoCache<T> {
    pub fn new(source: T, len: u64, mem_max: usize) -> Result<Self> {
        if len > mem_max as u64 {
            let page_sz = (mem_max as f64).sqrt() as usize;
            if page_sz == 0 {
                return Err(Error::new_zero_cache("Not enough memory requested for AutoCache"));
            }
            let mut frame_count = page_sz + 1;
            if page_sz * frame_count > mem_max {
                frame_count = page_sz;
            }
            Ok(Swap(SwapCache::new(source, len, page_sz, frame_count)?))
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

    fn traverse_chunks<F: FnMut(&[u8]) -> Result<()>>(&self, range: Range<u64>, f: F) -> Result<()> {
        match self {
            Full(ref full) => full.traverse_chunks(range, f),
            Swap(ref swap) => swap.traverse_chunks(range, f),
        }
    }
}

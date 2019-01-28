use super::Cache;
use std::io::{Read, Seek, SeekFrom};
use std::ops::{RangeBounds, Bound};

use super::{Result, Error};

pub struct FullCache<T: Read + Seek> {
    source: T,
    data: Vec<u8>,
}

impl<T: Read + Seek> FullCache<T> {
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
    type Input = T;

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

    fn traverse_chunks<R: RangeBounds<u64>, F: FnMut(&[u8]) -> Result<()>>(&self, range: R, f: F) -> Result<()> {
        let mut f = f;
        let len = self.data.len() as u64;
        let start = match range.start_bound() {
            Bound::Included(start) => {
                if *start >= len { return Ok(()); } else { *start }
            },
            Bound::Excluded(start) => {
                let start = *start + 1;
                if start > len { return Ok(()); } else { start }
            },
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(end) => {
                if *end >= len { len } else { *end + 1 }
            },
            Bound::Excluded(end) => {
                if *end > len { len } else { *end }
            },
            Bound::Unbounded => len
        };
        f(&self.data[start as usize..end as usize])?;
        Ok(())
    }
}

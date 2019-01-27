use super::Cache;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;

use super::{Result, Error};

pub struct FullCache<T: Read + Seek> {
    source: T,
    data: Vec<u8>,
}

impl FullCache<std::fs::File> {
    fn from_file(source: std::fs::File) -> Result<Self> {
        FullCache::new(source)
    }
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

    fn traverse_chunks<F: FnMut(&[u8]) -> Result<()>>(&self, range: Range<u64>, f: F) -> Result<()> {
        let mut f = f;
        if range.start < self.len() {
            f(&self.data[range.start as usize..range.end as usize])?;
        }
        Ok(())
    }
}

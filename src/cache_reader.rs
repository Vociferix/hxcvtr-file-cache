use super::{Cache, Error};
use std::io::{Read, Seek, SeekFrom};

pub struct CacheReader<C: Cache> {
    pos: u64,
    cache: C,
}

impl<C: Cache> CacheReader<C> {
    pub fn new(cache: C) -> Self {
        Self {
            pos: 0,
            cache,
        }
    }

    pub fn into_inner(self) -> C {
        self.cache
    }

    pub fn cache(&self) -> &C {
        &self.cache
    }

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
                    0
                }
            },
        };
        Ok(self.pos)
    }
}
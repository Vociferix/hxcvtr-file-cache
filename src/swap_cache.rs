use super::Cache;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::sync::Mutex;

use super::{Result, Error};

struct Frame {
    data: Vec<u8>,
    page: u64,
    next: Option<usize>,
    prev: Option<usize>,
}

struct SwapCacheImpl<T: Read + Seek> {
    page_sz: u64,
    source: T,
    frames: Vec<Frame>,
    map: HashMap<u64, usize>,
    front: usize,
    back: usize,
}

impl<T: Read + Seek> SwapCacheImpl<T> {
    fn new(source: T, page_size: usize, frame_count: usize) -> Result<Self> {
        let mut source = source;
        let mut frames: Vec<Frame> = Vec::new();
        let mut map: HashMap<u64, usize> = HashMap::new();
        let last = frame_count - 1;
        match source.seek(SeekFrom::Start(0)) {
            Ok(_) => {}
            Err(e) => {
                return Err(Error::from_io(e));
            }
        }
        frames.reserve_exact(frame_count as usize);
        map.reserve(frame_count as usize);
        if frame_count == 1 {
            map.insert(0, 0);
            let mut data = vec![0; page_size as usize];
            match source.read(&mut data) {
                Ok(_) => {}
                Err(e) => return Err(Error::from_io(e)),
            }
            frames.push(Frame {
                data,
                page: 0,
                next: None,
                prev: None,
            })
        } else {
            for i in 0..frame_count {
                map.insert((i as u64) * (page_size as u64), i);
                let mut data = vec![0; page_size as usize];
                match source.read(&mut data) {
                    Ok(_) => {}
                    Err(e) => return Err(Error::from_io(e)),
                }
                if i == 0 {
                    frames.push(Frame {
                        data,
                        page: 0,
                        next: None,
                        prev: Some(1),
                    });
                } else if i == last {
                    frames.push(Frame {
                        data,
                        page: last as u64,
                        next: Some(last),
                        prev: None,
                    });
                } else {
                    frames.push(Frame {
                        data,
                        page: i as u64,
                        next: Some(i - 1),
                        prev: Some(i + 1),
                    });
                }
            }
        }
        Ok(SwapCacheImpl {
            page_sz: page_size as u64,
            source,
            frames,
            map,
            front: last,
            back: 0,
        })
    }

    fn load_page(&mut self, page: u64) -> Result<usize> {
        match self.map.remove(&self.frames[self.front].page) {
            Some(_) => {},
            None => unreachable!(),
        }

        match self.source.seek(SeekFrom::Start(page * self.page_sz)) {
            Err(e) => {
                return Err(Error::from_io(e));
            }
            _ => {}
        }

        match self.source.read(&mut self.frames[self.front].data) {
            Ok(_) => {}
            Err(e) => return Err(Error::from_io(e)),
        }

        self.frames[self.front].page = page;

        Ok(self.front)
    }

    fn promote_frame(&mut self, fidx: usize) {
        if self.back != self.front {
            match self.frames[fidx].next {
                Some(next) => {
                    match self.frames[fidx].prev {
                        Some(prev) => {
                            self.frames[prev].next = Some(next);
                            self.frames[next].prev = Some(prev);
                        }
                        None => {
                            self.front = next;
                            self.frames[next].prev = None;
                        }
                    }
                    self.frames[self.back].next = Some(fidx);
                    self.frames[fidx].prev = Some(self.back);
                    self.frames[fidx].next = None;
                }
                None => {}
            }
        }
    }

    fn get_chunk(&mut self, pos: u64) -> Result<&[u8]> {
        let fidx: usize;
        let page = pos / self.page_sz;

        let fidx_opt: Option<usize> = match self.map.get(&page) {
            Some(fidx) => Some(*fidx),
            None => None,
        };

        fidx = match fidx_opt {
            Some(fidx) => fidx,
            None => self.load_page(page)?,
        };

        self.promote_frame(fidx);

        Ok(&self.frames[fidx].data[(pos - (page * self.page_sz)) as usize..])
    }
}

pub struct SwapCache<T: Read + Seek> {
    sz: u64,
    swap: Mutex<SwapCacheImpl<T>>,
}

impl SwapCache<std::fs::File> {
    pub fn from_file(
        source: std::fs::File,
        page_size: usize,
        frame_count: usize,
    ) -> Result<Self> {
        let len;
        match source.metadata() {
            Ok(meta) => {
                len = meta.len();
            }
            Err(e) => return Err(Error::from_io(e)),
        }
        SwapCache::new(source, len, page_size, frame_count)
    }
}

impl<T: Read + Seek> SwapCache<T> {
    pub fn new(source: T, len: u64, page_size: usize, frame_count: usize) -> Result<Self> {
        if page_size != 0 && frame_count != 0 {
            Ok(SwapCache {
                sz: len,
                swap: Mutex::new(SwapCacheImpl::new(source, page_size, frame_count)?),
            })
        } else if page_size == 0 {
            Err(Error::new_zero_cache("swap cache given zero pages"))
        } else {
            Err(Error::new_zero_cache("swap cache given zero frames"))
        }
    }
}

impl<T: Read + Seek> Cache for SwapCache<T> {
    type Input = T;

    fn into_inner(self) -> Result<T> {
        match Mutex::into_inner(self.swap) {
            Ok(mut swap) => match swap.source.seek(SeekFrom::Start(0)) {
                Err(e) => Err(Error::from_io(e)),
                _ => Ok(swap.source),
            },
            Err(e) => Err(Error::from_poison(e)),
        }
    }

    fn len(&self) -> u64 {
        self.sz
    }

    fn traverse_chunks<F: FnMut(&[u8]) -> Result<()>>(&self, range: Range<u64>, f: F) -> Result<()> {
        if range.start < self.sz {
            let mut f = f;
            let mut guard = match self.swap.lock() {
                Ok(guard) => guard,
                Err(e) => return Err(Error::from_poison(e)),
            };
            let mut pos = range.start;
            loop {
                let chunk = (*guard).get_chunk(pos)?;
                let new_pos = pos + chunk.len() as u64;
                if new_pos > range.end {
                    return f(&chunk[..(new_pos - pos) as usize]);
                } else {
                    f(chunk)?;
                }
                pos = new_pos;
            }
        }
        Ok(())
    }
}

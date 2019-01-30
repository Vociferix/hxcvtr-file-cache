use super::Cache;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::ops::{RangeBounds, Bound};
use std::sync::Mutex;

use super::{Result, Error};

struct Frame {
    data: Vec<u8>,
    page: u64,
    next: usize,
    prev: usize,
}

const NULL: usize = std::usize::MAX;

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
                next: NULL,
                prev: NULL,
            })
        } else {
            for i in 0..frame_count {
                map.insert(i as u64, i);
                let mut data = vec![0; page_size as usize];
                match source.read(&mut data) {
                    Ok(_) => {}
                    Err(e) => return Err(Error::from_io(e)),
                }
                if i == 0 {
                    frames.push(Frame {
                        data,
                        page: 0,
                        next: NULL,
                        prev: 1,
                    });
                } else if i == last {
                    frames.push(Frame {
                        data,
                        page: i as u64,
                        next: last,
                        prev: NULL,
                    });
                } else {
                    frames.push(Frame {
                        data,
                        page: i as u64,
                        next: i - 1,
                        prev: i + 1,
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

    fn get_frame(&self, fidx: usize) -> &Frame {
        &self.frames[fidx]
    }

    fn get_frame_mut(&mut self, fidx: usize) -> &mut Frame {
        &mut self.frames[fidx]
    }

    fn map_frame<Ret, F: Fn(&Frame) -> Ret>(&self, fidx: usize, f: F) -> Ret {
        f(self.get_frame(fidx))
    }

    fn map_frame_mut<Ret, F: Fn(&mut Frame) -> Ret>(&mut self, fidx: usize, f: F) -> Ret {
        f(self.get_frame_mut(fidx))
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

        self.map.insert(page, self.front);

        Ok(self.front)
    }

    fn promote_frame(&mut self, fidx: usize) {
        if self.back != self.front {
            let (next_idx, prev_idx) = self.map_frame(fidx, |frame| {
                (frame.next, frame.prev)
            });
            if next_idx != NULL {
                if prev_idx != NULL {
                    self.get_frame_mut(prev_idx).next = next_idx;
                    self.get_frame_mut(next_idx).prev = prev_idx;
                } else {
                    self.front = next_idx;
                    self.get_frame_mut(next_idx).prev = NULL;
                }
                self.get_frame_mut(self.back).next = fidx;
                let back_idx = self.back;
                self.map_frame_mut(fidx, |frame| {
                    frame.prev = back_idx;
                    frame.next = NULL;
                });
            }
        }
    }

    fn get_chunk(&mut self, pos: u64) -> Result<&[u8]> {
        let page = pos / self.page_sz;

        let fidx = match self.map.get(&page) {
            Some(fidx) => *fidx,
            None => NULL,
        };

        let fidx = if fidx == NULL {
            self.load_page(page)?
        } else {
            fidx
        };

        self.promote_frame(fidx);

        Ok(&self.get_frame(fidx).data[(pos - (page * self.page_sz)) as usize..])
    }
}

/// A cache that swaps pages in and out of memory using an LRU policy.
///
/// `SwapCache` allocates in-memory frames which store pages from the
/// source that have been swapped in. When a page needs to be swapped
/// in, the least recently accessed page currently swapped in memory
/// will be replaced by the new page. Because interior mutability is
/// required, the primary functionality of `SwapCache` is wrapped with
/// a mutex, which also makes it thread safe.
pub struct SwapCache<T: Read + Seek> {
    sz: u64,
    cache_sz: usize,
    swap: Mutex<SwapCacheImpl<T>>,
}

impl<T: Read + Seek> SwapCache<T> {
    /// Creates a new `SwapCache` containing the passed source, and with pages
    /// of size `page_size` bytes, and `frame_count` frames.
    pub fn new(source: T, page_size: usize, frame_count: usize) -> Result<Self> {
        let mut source = source;
        let len = match source.seek(SeekFrom::End(0)) {
            Ok(len) => len,
            Err(e) => return Err(Error::from_io(e)),
        };
        if page_size != 0 && frame_count != 0 {
            Ok(SwapCache {
                sz: len,
                cache_sz: page_size * frame_count,
                swap: Mutex::new(SwapCacheImpl::new(source, page_size, frame_count)?),
            })
        } else if page_size == 0 {
            Err(Error::new_zero_cache("swap cache configured with zero pages"))
        } else {
            Err(Error::new_zero_cache("swap cache configured with zero frames"))
        }
    }
}

impl<T: Read + Seek> Cache for SwapCache<T> {
    type Source = T;

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

    fn cache_size(&self) -> usize {
        self.cache_sz
    }

    fn traverse_chunks<R: RangeBounds<u64>, F: FnMut(&[u8]) -> Result<()>>(&self, range: R, f: F) -> Result<()> {
        let len = self.sz;
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
        if start < len {
            let mut f = f;
            let mut guard = match self.swap.lock() {
                Ok(guard) => guard,
                Err(e) => return Err(Error::from_poison(e)),
            };
            let mut pos = start;
            loop {
                let chunk = (*guard).get_chunk(pos)?;
                let new_pos = pos + chunk.len() as u64;
                if new_pos > end {
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

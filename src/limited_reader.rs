use std::io::{Read, Seek, SeekFrom};
use std::ops::{Bound, RangeBounds};

pub struct LimitedReader<R, B: RangeBounds<u64>> {
    inner: R,
    range: B,
    pos: u64,
}

impl<R: Read + Seek, B: RangeBounds<u64>> LimitedReader<R, B> {
    pub fn new(mut inner: R, range: B) -> std::io::Result<Self> {
        let start = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };

        inner.seek(SeekFrom::Start(start))?;

        Ok(Self {
            inner,
            pos: start,
            range,
        })
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read + Seek, B: RangeBounds<u64>> Read for LimitedReader<R, B> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let end_bound = match self.range.end_bound() {
            Bound::Excluded(&n) => Some(n),
            Bound::Included(&n) => Some(n + 1),
            Bound::Unbounded => None,
        };

        let to_read = if let Some(end) = end_bound {
            if self.pos >= end {
                return Ok(0);
            }
            let remaining = end - self.pos;
            (buf.len() as u64).min(remaining) as usize
        } else {
            buf.len()
        };

        let n = self.inner.read(&mut buf[..to_read])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Read + Seek, B: RangeBounds<u64>> Seek for LimitedReader<R, B> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let start = match self.range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };

        let new_pos = match pos {
            SeekFrom::Start(offset) => start + offset,
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.pos + offset as u64
                } else {
                    self.pos.saturating_sub((-offset) as u64)
                }
            }
            SeekFrom::End(offset) => {
                let end = match self.range.end_bound() {
                    Bound::Excluded(&end) => end,
                    Bound::Included(&end) => end + 1,
                    Bound::Unbounded => self.inner.seek(SeekFrom::End(0))?,
                };

                if offset >= 0 {
                    end + offset as u64
                } else {
                    end.saturating_sub((-offset) as u64)
                }
            }
        };

        let clamped = match self.range.end_bound() {
            Bound::Excluded(&end) => new_pos.clamp(start, end),
            Bound::Included(&end) => new_pos.clamp(start, end + 1),
            Bound::Unbounded => new_pos.max(start),
        };

        self.inner.seek(SeekFrom::Start(clamped))?;
        self.pos = clamped;
        Ok(clamped - start)
    }
}

use std::io::{Read, Seek, SeekFrom};

pub struct LimitedReader<R> {
    inner: R,
    start: u64,
    end: u64,
    pos: u64,
}

impl<R: Read + Seek> LimitedReader<R> {
    pub fn new(mut inner: R, start: u64, end: u64) -> std::io::Result<Self> {
        inner.seek(SeekFrom::Start(start))?;
        Ok(Self {
            inner,
            start,
            end,
            pos: start,
        })
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read + Seek> Read for LimitedReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.end.saturating_sub(self.pos);
        if remaining == 0 {
            return Ok(0);
        }

        let to_read = (buf.len() as u64).min(remaining) as usize;
        let n = self.inner.read(&mut buf[..to_read])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Read + Seek> Seek for LimitedReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_pos = match pos {
            SeekFrom::Start(offset) => self.start + offset,
            SeekFrom::Current(offset) => {
                if offset >= 0 {
                    self.pos + offset as u64
                } else {
                    self.pos.saturating_sub((-offset) as u64)
                }
            }
            SeekFrom::End(offset) => {
                if offset >= 0 {
                    self.end + offset as u64
                } else {
                    self.end.saturating_sub((-offset) as u64)
                }
            }
        };

        let clamped = new_pos.clamp(self.start, self.end);
        self.inner.seek(SeekFrom::Start(clamped))?;
        self.pos = clamped;
        Ok(clamped - self.start)
    }
}

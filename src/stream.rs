#![allow(dead_code)]
use crate::location::{line_column, Offset};
use std::io;

/// A stream which can be used to convert between offsets and line-column numbers.
#[derive(Debug)]
pub struct Stream<Reader> {
    reader: Reader,
    base: usize, // For future use

    lines: Vec<usize>,
    next_offset: usize,
    current_line: usize,
    buffer: Vec<u8>,
}

impl<R> Stream<R> {
    const BUF_SIZE: usize = 1024;

    #[inline]
    pub fn base(&self) -> usize {
        self.base
    }

    #[inline]
    pub fn line_offset(&self, line: usize) -> Option<Offset> {
        self.lines.get(line).copied().map(Offset::new)
    }
}

impl<R: io::Read> Stream<R> {
    pub fn new(reader: R, buffer_size: usize) -> Self {
        Self {
            reader,
            base: 0,
            lines: vec![0],
            next_offset: 0,
            current_line: 0,
            buffer: vec![0; buffer_size],
        }
    }

    #[inline]
    pub fn from_reader(reader: R) -> Self {
        Self::new(reader, Self::BUF_SIZE)
    }

    #[inline]
    pub fn set_base(&mut self, base: usize) {
        self.base = base;
    }

    #[inline]
    pub fn reset(&mut self) {
        self.set_base(0);
    }

    /// Read length
    #[inline]
    pub fn read_len(&self) -> usize {
        self.next_offset
    }

    /// Get offset from line and column number
    pub fn offset_of(&mut self, line_index: line_column::ZeroBased) -> io::Result<Offset> {
        let (line, col) = line_index.raw();
        loop {
            if let Some(offset) = self.lines.get(line) {
                break Ok(Offset::new(offset + col));
            }

            if self.forward()? == 0 {
                break Err(io_error(format!("Invalid line index: ({}, {})", line, col)));
            }
        }
    }

    /// Get line and column number from offset
    pub fn line_index(&mut self, offset: Offset) -> io::Result<line_column::ZeroBased> {
        let line = self.line_of(offset)?;
        let line_offset = self.lines.get(line).unwrap();
        let col = offset.raw() - line_offset;
        Ok((line, col).into())
    }

    /// Try to get line of offset without reading
    pub fn try_line_of(&self, offset: Offset) -> Option<usize> {
        let offset = offset.raw();
        binary_search_between(&self.lines, offset)
    }

    /// Try to get line index of offset
    pub fn try_line_index(&self, offset: Offset) -> Option<line_column::ZeroBased> {
        let line = self.try_line_of(offset)?;
        let line_offset = self.lines.get(line).unwrap();
        let col = offset.raw() - line_offset;
        Some((line, col).into())
    }

    /// Try to get offset from (line, column)
    pub fn try_offset_of(&mut self, line_index: line_column::ZeroBased) -> Option<Offset> {
        let (line, col) = line_index.raw();
        self.lines.get(line).map(|offset| Offset::new(offset + col))
    }

    /// Get line of offset
    pub fn line_of(&mut self, offset: Offset) -> io::Result<usize> {
        let offset = offset.raw();

        let mut begin = 0;
        loop {
            let n = self.lines.len();
            if let Some(i) = binary_search_between(&self.lines[begin..], offset) {
                break Ok(i);
            }
            if n > 0 {
                begin = n - 1;
            }

            if self.forward()? == 0 {
                if offset < self.next_offset {
                    break Ok(self.current_line);
                }
                break Err(io_error("Invalid offset, exceed EOF"));
            }
        }
    }

    #[inline]
    fn read_bytes(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.reader.read(buf)
    }

    /// Try to get more bytes and update states
    fn forward(&mut self) -> io::Result<usize> {
        let n = self.reader.read(&mut self.buffer[..])?;

        for (offset, b) in self.buffer.iter().take(n).enumerate() {
            if *b == b'\n' {
                self.current_line += 1;
                self.lines.push(self.next_offset + offset + 1); // next line begin
                continue;
            }
        }
        self.next_offset += n;
        Ok(n)
    }

    #[inline]
    pub fn read_forward(&mut self) -> io::Result<(usize, &[u8])> {
        let n = self.reader.read(&mut self.buffer[..])?;
        Ok((n, &self.buffer))
    }

    /// Get line, column of offset while returning read bytes in `buf`
    pub fn read_line_of(&mut self, offset: Offset, buf: &mut Vec<u8>) -> io::Result<usize> {
        let offset = offset.raw();

        let mut begin = 0;
        loop {
            let n = self.lines.len();
            if let Some(i) = binary_search_between(&self.lines[begin..], offset) {
                break Ok(i);
            }
            if n > 0 {
                begin = n - 1;
            }

            let (n_bytes, buffer) = self.read_forward()?;
            if n_bytes == 0 {
                if offset < self.next_offset {
                    break Ok(self.current_line);
                }
                break Err(io_error("Invalid offset, exceed EOF"));
            }
            buf.extend(buffer);
        }
    }

    /// Get line, column index while returning read bytes in `buf`
    pub fn read_line_index(
        &mut self,
        offset: Offset,
        buf: &mut Vec<u8>,
    ) -> io::Result<line_column::ZeroBased> {
        let line = self.read_line_of(offset, buf)?;
        let line_offset = self.lines.get(line).unwrap();
        let col = offset.raw() - line_offset;
        Ok((line, col).into())
    }

    /// Get offset from line-column index while returning read bytes in `buf`
    pub fn read_offset_of(
        &mut self,
        line_index: line_column::ZeroBased,
        buf: &mut Vec<u8>,
    ) -> io::Result<Offset> {
        let (line, col) = line_index.raw();
        loop {
            if let Some(offset) = self.lines.get(line) {
                break Ok(Offset::new(offset + col));
            }

            let (n_bytes, buffer) = self.read_forward()?;
            if n_bytes == 0 {
                break Err(io_error(format!("Invalid line index: ({}, {})", line, col)));
            }
            buf.extend(buffer);
        }
    }

    /// Drain the reader, consume the reader
    pub fn drain(&mut self) -> io::Result<()> {
        loop {
            let n = self.forward()?;
            if n == 0 {
                return Ok(());
            }
        }
    }
}

impl<R: io::Read> From<R> for Stream<R> {
    fn from(value: R) -> Self {
        Stream::from_reader(value)
    }
}

impl<R: io::Read> io::Read for Stream<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_bytes(buf)
    }
}

/// Assuming `xs` is ordered, try to find a interval where `x` lies.  
/// returns the start index of interval
fn binary_search_between(xs: &[usize], x: usize) -> Option<usize> {
    if xs.is_empty() {
        return None;
    }
    if x == xs[0] {
        return Some(0);
    }
    if x < xs[0] {
        return None;
    }

    let mut start = 0;
    let mut end = xs.len() - 1;
    while start < end {
        // base case
        if start == end - 1 && xs[start] <= x && x < xs[end] {
            return Some(start);
        }

        // binary search
        let mid = start + ((end - start) >> 1);
        let y = xs[mid];
        if x == y {
            return Some(mid);
        }

        if x < y {
            end = mid;
            continue;
        }
        // x > y
        if start == mid {
            return None;
        }
        start = mid;
    }

    None
}

#[inline]
fn io_error<S: ToString>(msg: S) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg.to_string())
}

#[cfg(test)]
mod test {
    #![allow(unused_must_use)]
    use std::fs::File;

    use super::*;

    #[test]
    fn test_binary_search() {
        let xs = [2, 4, 6];
        let i = binary_search_between(&xs, 3);
        assert_eq!(i, Some(0));

        let i = binary_search_between(&xs, 1);
        assert_eq!(i, None);
    }

    #[test]
    fn test_stream_str() {
        let reader = "\nThis is s sim\nple test that\n I have to verify stread reader!";
        let mut stream = Stream::from(reader.as_bytes());
        let ans = stream.line_index(Offset::new(20));
        assert!(ans.is_ok());
        assert_eq!(ans.unwrap(), (2, 5).into());
    }

    #[test]
    fn test_stream_file() {
        let file = File::open("/Users/comcx/Workspace/Repo/stream-locate-converter/Cargo.toml")
            .expect("Failed to open file");
        let mut stream = Stream::from(file);
        let ans = stream.line_index(Offset::new(50));
        dbg!(ans);

        let ans = stream.line_index(Offset::new(20));
        dbg!(ans);
    }

    #[test]
    fn test_stream_drain() {
        let file = File::open("/Users/comcx/Workspace/Repo/stream-locate-converter/Cargo.toml")
            .expect("Failed to open file");
        let mut stream = Stream::from(file);
        let ans = stream.drain();
        dbg!(ans);
        dbg!(stream.lines);
    }
}

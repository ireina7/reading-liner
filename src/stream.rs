#![allow(dead_code)]
use crate::{
    index::{self, Index},
    location::{line_column, Offset},
};
use std::io;

/// A stream which can be used to convert between offsets and line-column locations.
#[derive(Debug)]
pub struct Stream<'index, Reader> {
    reader: Reader,

    base: usize, // For future use
    index: &'index mut Index,
    next_offset: usize,
    current_line: usize,
}

impl<'index, R> Stream<'index, R> {
    const DEFAULT_BUF_SIZE: usize = 1024;

    #[inline]
    pub fn base(&self) -> usize {
        self.base
    }

    #[inline]
    pub fn line_offset(&self, line: usize) -> Option<Offset> {
        self.index.query().get_line_offset(line)
    }

    #[inline]
    pub fn query(&self) -> index::Query<'_> {
        self.index.query()
    }

    #[inline]
    pub fn get_index(&self) -> &Index {
        &self.index
    }
}

impl<'index, R: io::Read> Stream<'index, R> {
    pub fn new(reader: R, index: &'index mut Index) -> Self {
        Self {
            reader,
            base: 0,
            index,
            next_offset: 0,
            current_line: 0,
        }
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
    pub fn offset_of(
        &mut self,
        line_index: line_column::ZeroBased,
        buf: &mut [u8],
    ) -> io::Result<Offset> {
        let (line, col) = line_index.raw();
        loop {
            if let Some(offset) = self.query().get_line_offset(line) {
                break Ok(offset + col);
            }

            if self.forward(buf)? == 0 {
                break Err(io_error(format!("Invalid line index: ({}, {})", line, col)));
            }
        }
    }

    /// Get line and column number from offset
    pub fn locate(&mut self, offset: Offset, buf: &mut [u8]) -> io::Result<line_column::ZeroBased> {
        let line = self.locate_line(offset, buf)?;
        let line_offset = self.query().get_line_offset(line).unwrap();
        let col = offset - line_offset;
        Ok((line, col.raw()).into())
    }

    /// Try to get line of offset without reading
    fn try_line_of(&self, offset: Offset) -> Option<usize> {
        self.query().locate_line(offset)
    }

    /// Try to get line index of offset
    pub fn try_line_index(&self, offset: Offset) -> Option<line_column::ZeroBased> {
        let line = self.try_line_of(offset)?;
        let line_offset = self.query().get_line_offset(line).unwrap();
        let col = offset - line_offset;
        Some((line, col.raw()).into())
    }

    /// Try to get offset from (line, column)
    pub fn try_offset_of(&mut self, line_index: line_column::ZeroBased) -> Option<Offset> {
        let (line, col) = line_index.raw();
        self.query()
            .get_line_offset(line)
            .map(|offset| offset + col)
    }

    /// Get line of offset
    pub fn locate_line(&mut self, offset: Offset, buf: &mut [u8]) -> io::Result<usize> {
        let mut begin = 0;
        loop {
            let n = self.index.len();
            if let Some(i) = self.query().from_range_from(begin..).locate_line(offset) {
                break Ok(begin + i);
            }
            if n > 0 {
                begin = n - 1;
            }

            if self.forward(buf)? == 0 {
                if offset.raw() < self.next_offset {
                    break Ok(self.current_line);
                }
                break Err(io_error("Invalid offset, exceed EOF"));
            }
        }
    }

    /// Try to get more bytes and update states
    fn forward(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf)?;

        for (offset, b) in buf.iter().take(n).enumerate() {
            if *b == b'\n' {
                self.current_line += 1;
                self.index
                    .next_line(Offset::new(self.next_offset + offset + 1)); // next line begin
                continue;
            }
        }
        self.next_offset += n;
        Ok(n)
    }

    /// Drain the reader, consume the reader
    pub fn drain(&mut self, buf: &mut [u8]) -> io::Result<()> {
        loop {
            let n = self.forward(buf)?;
            if n == 0 {
                return Ok(());
            }
        }
    }
}

impl<'index, R: io::Read> io::Read for Stream<'index, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.forward(buf)
    }
}

#[inline]
fn io_error<S: ToString>(msg: S) -> io::Error {
    io::Error::new(io::ErrorKind::Other, msg.to_string())
}

#[cfg(test)]
mod test {
    #![allow(unused_must_use)]
    use std::{
        fs::File,
        io::{BufReader, Read},
    };

    use super::*;

    #[test]
    fn test_stream_str() {
        let reader = "\nThis is s sim\nple test that\n I have to verify stream reader!";

        let mut index = Index::new_from_zero();
        // let mut stream = Stream::new(reader.as_bytes(), &mut index);
        // let mut buf = vec![b'\0'; 10];
        // stream.drain(&mut buf);

        let stream = Stream::new(reader.as_bytes(), &mut index);
        let mut reader = BufReader::new(stream);
        let mut buf = String::new();
        let src = reader.read_to_string(&mut buf);
        dbg!(buf);

        let ans = reader.get_ref().query().locate(Offset::new(20));
        assert!(ans.is_some());
        assert_eq!(ans.unwrap(), (2, 5).into());
        // dbg!(stream);
    }

    #[test]
    fn test_stream_file() {
        let file = File::open("/Users/comcx/Workspace/Repo/stream-locate-converter/Cargo.toml")
            .expect("Failed to open file");
        let mut index = Index::new_from_zero();
        let mut stream = Stream::new(file, &mut index);
        let mut buf = vec![b'\0'; 100];
        let ans = stream.locate(Offset::new(50), &mut buf);
        dbg!(ans);

        let ans = stream.locate(Offset::new(20), &mut buf);
        dbg!(ans);
    }

    #[test]
    fn test_stream_drain() {
        let file = File::open("/Users/comcx/Workspace/Repo/stream-locate-converter/Cargo.toml")
            .expect("Failed to open file");
        let mut index = Index::new_from_zero();
        let mut stream = Stream::new(file, &mut index);
        let mut buf = vec![b'\0'; 100];
        let ans = stream.drain(&mut buf);
        dbg!(ans);
        dbg!(stream.index);
    }
}

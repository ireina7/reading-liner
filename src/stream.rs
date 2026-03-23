use crate::{
    index::{self, Index},
    location::{Offset, line_column},
};
use std::io;

/// A stream which can be used to convert between offsets and line-column locations.
#[derive(Debug)]
pub struct Stream<'index, Reader> {
    reader: Reader,

    base: usize, // For future use
    index: &'index mut Index,
    next_offset: Offset,
    current_line: usize,
}

impl<'index, R> Stream<'index, R> {
    pub fn new(reader: R, index: &'index mut Index) -> Self {
        Self {
            reader,
            base: 0,
            index,
            next_offset: 0.into(),
            current_line: 0,
        }
    }

    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    #[inline]
    pub fn base(&self) -> usize {
        self.base
    }

    /// Immutable query to further query offsets and line-column locations
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
    /// Read length
    #[inline]
    pub fn read_len(&self) -> usize {
        self.next_offset.raw()
    }

    /// Try to get more bytes and update states
    fn forward(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = self.reader.read(buf)?;

        for (offset, b) in buf.iter().take(n).enumerate() {
            if *b == b'\n' {
                self.current_line += 1;
                self.index.add_next_line(self.next_offset + offset + 1); // next line begin
                continue;
            }
        }

        // reached EoF, try to add fake ending
        if !buf.is_empty() && n == 0 {
            // TODO
            match self.index.end() {
                Some(end) if end != self.next_offset => {
                    self.index.add_next_line(self.next_offset);
                }
                None => self.index.add_next_line(self.next_offset),
                _ => {}
            }
        }

        self.next_offset += n;
        Ok(n)
    }

    /// Locate the (line, column) position for a given byte `offset`.  
    ///
    /// NOTE: this method may cause extra reading when the offset input cannot find a location.
    ///
    /// This method first resolves the line index via [`locate_line`], then
    /// computes the column by subtracting the starting offset of that line.
    ///
    /// # Parameters
    /// - `offset`: The target byte offset.
    /// - `buf`: A temporary buffer used for incremental reading.
    ///
    /// # Returns
    /// - `Ok(ZeroBased(line, column))` if the offset is within bounds.
    /// - `Err` if the offset exceeds EOF (propagated from [`locate_line`]).
    ///
    /// # Invariants
    /// - The internal index always contains a valid starting offset for every line.
    /// - Therefore, `line_offset(line)` must succeed for any valid `line`.
    ///
    /// # Notes
    /// - Both line and column are zero-based.
    /// - Column is computed in **bytes**, not characters (UTF-8 aware handling is not performed here).
    pub fn locate(&mut self, offset: Offset, buf: &mut [u8]) -> io::Result<line_column::ZeroBased> {
        let line = self.locate_line(offset, buf)?;
        let line_offset = self.query().line_offset(line).unwrap();
        let col = offset - line_offset;
        Ok((line, col.raw()).into())
    }

    /// Locate the line index for a given byte `offset`.
    ///
    /// This method performs an incremental lookup:
    /// it first queries the existing line index, and if the offset
    /// is not covered, it reads more data and extends the index.
    /// This method may cause extra reading when the offset input cannot find a location.
    ///
    /// # Invariants
    /// - The internal index is non-empty and ends with a sentinel EOF offset.
    ///
    /// # Errors
    /// Returns an error if `offset` exceeds EOF.
    pub fn locate_line(&mut self, offset: Offset, buf: &mut [u8]) -> io::Result<usize> {
        let mut begin = 0;
        loop {
            // Invariant: index is non-empty and ends with EOF.
            // Therefore, begin <= query.count() always holds, and range_from(begin..)
            // is guaranteed to be a valid slice (possibly containing only EOF).
            if let Some(i) = self.query().range_from(begin..).locate_line(offset) {
                break Ok(i); // look here the returned `i` is already `begin` based, there's no need to add an extra begin
            }
            begin = self.index.count();

            if self.forward(buf)? == 0 {
                break Err(io_error("Invalid offset, exceed EOF"));
            }
        }
    }

    /// Encode a (line, column) location into a byte `Offset`.
    ///
    /// This method may incrementally extend the internal line index by reading
    /// additional data if the requested line is not yet available.
    ///
    /// # Behavior
    /// - If the line is already indexed, the offset is computed directly.
    /// - Otherwise, more data is read and the index is extended until the line
    ///   becomes available or EOF is reached.
    ///
    /// # Returns
    /// - `Ok(offset)` if the position can be resolved.
    /// - `Err` if the line index exceeds EOF.
    ///
    /// # Notes
    /// - Column is interpreted as a **byte offset** relative to the start of the line.
    /// - This method does **not** validate whether the column lies within the bounds
    ///   of the line.
    pub fn encode(
        &mut self,
        line_index: line_column::ZeroBased,
        buf: &mut [u8],
    ) -> io::Result<Offset> {
        let (line, col) = line_index.raw();
        loop {
            if let Some(offset) = self.query().line_offset(line) {
                break Ok(offset + col);
            }

            if self.forward(buf)? == 0 {
                break Err(io_error(format!("Invalid line index: ({}, {})", line, col)));
            }
        }
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

/// You can use [Stream] as a normal [io::Read] and recording index at the same time.
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
    use std::io::{BufReader, Read};

    use super::*;

    #[test]
    fn test_stream_str() {
        let reader = "\nThis is s sim\nple test that\n I have to verify stream reader!";

        let mut index = Index::new();
        // let mut stream = Stream::new(reader.as_bytes(), &mut index);
        // let mut buf = vec![b'\0'; 10];
        // stream.drain(&mut buf);

        let stream = Stream::new(reader.as_bytes(), &mut index);
        let mut reader = BufReader::new(stream);
        let mut buf = String::new();
        reader.read_to_string(&mut buf).unwrap();

        let ans = reader.get_ref().query().locate(Offset(20));
        assert!(ans.is_some());
        assert_eq!(ans.unwrap(), (2, 5).into());
    }

    // #[test]
    // fn test_stream_file() {
    //     let file = std::fs::File::open("Cargo.toml").expect("Failed to open file");
    //     let mut index = Index::new();
    //     let mut stream = Stream::new(file, &mut index);
    //     let mut buf = vec![b'\0'; 10];
    //     let ans = stream.locate(Offset(50), &mut buf);
    //     dbg!(ans);

    //     let ans = stream.locate(Offset(20), &mut buf);
    //     dbg!(ans);
    // }
}

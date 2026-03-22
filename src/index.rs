use std::ops;

use crate::location::{Offset, line_column};

/// An index built for fast convertion between byte offsets and line-column locations.
///
/// NOTE: the last element of `line_offsets` should be considered as a fake offset.
/// By the word 'fake', we means that if some offset >= the last offset,
/// it should be seen as exceding the index since the last element only marks ending.
#[derive(Debug)]
pub struct Index {
    /// vector of offsets whose element records the beginning offset of source UTF8 string
    line_offsets: Vec<Offset>,
}

impl Index {
    /// An index with the first line starting at offset 0, which is the most common usage.
    ///
    /// The zero is safe here since it just means an ending, which also means empty.
    pub fn new() -> Self {
        Self {
            line_offsets: vec![0.into()],
        }
    }

    /// length of index
    pub fn len(&self) -> usize {
        self.line_offsets.len()
    }

    /// ending offset of the source
    pub fn end(&self) -> Option<Offset> {
        self.line_offsets.last().copied()
    }

    /// into vector of offsets
    pub fn into_offsets(self) -> Vec<Offset> {
        self.line_offsets
    }
}

impl Index {
    /// Get the query and freeze index when querying
    pub fn query(&self) -> Query<'_> {
        Query::from(&self.line_offsets[..])
    }

    pub fn get_line_offset_mut(&mut self, line_no: usize) -> Option<&mut Offset> {
        self.line_offsets.get_mut(line_no)
    }

    /// Add next line offset to index
    pub fn add_next_line(&mut self, offset: Offset) {
        self.line_offsets.push(offset);
    }

    /// Reset the index
    pub fn clear(&mut self) {
        self.line_offsets.clear();
        self.add_next_line(0.into());
    }
}

/// Query line and offset information.
///
/// NOTE: Since the `Query` can be sliced, we carefully store an extra beginning offset to trace slice beginning.
/// One should keep in mind that all line numbers passed into query methods should be numbers **from the original [Index]**.
#[derive(Debug)]
pub struct Query<'index> {
    /// the beginning line number,
    /// used to recover line number since `slice` lacks beginning offset from the original [Index]
    begin: usize,

    slice: &'index [Offset],
}

impl<'index> Query<'index> {
    pub fn new(begin: usize, slice: &'index [Offset]) -> Self {
        Self { begin, slice }
    }

    /// build from raw slice, assuming the beginning is zero
    pub fn from(slice: &'index [Offset]) -> Self {
        Self { begin: 0, slice }
    }

    pub fn range(&self, range: ops::Range<usize>) -> Self {
        Self::new(range.start, &self.slice[range])
    }

    pub fn range_from(&self, range_from: ops::RangeFrom<usize>) -> Self {
        Self::new(range_from.start, &self.slice[range_from])
    }

    pub fn len(&self) -> usize {
        self.slice.len()
    }
}

impl<'index> Query<'index> {
    #[inline]
    pub fn get_line_offset(&self, line_no: usize) -> Option<Offset> {
        if line_no < self.begin {
            return None;
        }
        let line_no = line_no - self.begin;
        self.slice.get(line_no).copied()
    }

    /// Locate line number from byte offset
    #[inline]
    pub fn locate_line(&self, offset: Offset) -> Option<usize> {
        binary_search_between(&self.slice, offset).map(|n| self.begin + n)
    }

    /// Locate line-column numbers from byte offset
    pub fn locate(&self, offset: Offset) -> Option<line_column::ZeroBased> {
        let line = self.locate_line(offset)?;
        let line_offset = self.get_line_offset(line).unwrap();
        let col = offset - line_offset;

        let line = self.begin + line;
        Some((line, col.raw()).into())
    }

    /// Encode byte offset from line-column location
    pub fn encode(&self, location: line_column::ZeroBased) -> Option<Offset> {
        let (mut line, col) = location.raw();
        if line < self.begin {
            return None;
        }
        line -= self.begin;

        if let Some(offset) = self.get_line_offset(line) {
            return Some(offset + col);
        }
        None
    }
}

/// SAFETY: Assuming `xs` is ordered, try to find a interval where `x` lies.  
/// returns the start index of interval
///
/// NOTE: if the input `x` equals the last element of `xs`, this function still returns `None`.
/// This is because the last element is regarded as an fake ending.
fn binary_search_between<A: Ord + Copy>(xs: &[A], x: A) -> Option<usize> {
    if xs.len() <= 1 {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_binary_search() {
        let xs = [2, 4, 6];
        let i = binary_search_between(&xs, 3);
        assert_eq!(i, Some(0));

        let i = binary_search_between(&xs, 4);
        assert_eq!(i, Some(1));

        let i = binary_search_between(&xs, 1);
        assert_eq!(i, None);

        let i = binary_search_between(&xs, 7);
        assert_eq!(i, None);

        let i = binary_search_between(&xs, 6);
        assert_eq!(i, None);
    }
}

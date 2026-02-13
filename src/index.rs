use std::ops;

use crate::location::{line_column, Offset};

/// An index built for fast convertion between byte offsets and line-column locations.
#[derive(Debug)]
pub struct Index {
    line_offsets: Vec<Offset>,
}

impl Index {
    /// empty index
    pub fn new() -> Self {
        Self {
            line_offsets: Vec::new(),
        }
    }

    /// An index with the first line starting at offset 0, which is the most common usage.
    pub fn new_from_zero() -> Self {
        Self {
            line_offsets: vec![0.into()],
        }
    }

    /// length of index
    pub fn len(&self) -> usize {
        self.line_offsets.len()
    }
}

impl Index {
    /// Get the query and freeze index when querying
    pub fn query(&self) -> Query<'_> {
        Query::from_slice(&self.line_offsets[..])
    }

    pub fn get_line_offset_mut(&mut self, line_no: usize) -> Option<&mut Offset> {
        self.line_offsets.get_mut(line_no)
    }

    /// Add next line offset to index
    pub fn add_next_line(&mut self, offset: Offset) {
        self.line_offsets.push(offset);
    }

    pub fn clear(&mut self) {
        self.line_offsets.clear();
    }
}

#[derive(Debug)]
pub struct Query<'index> {
    slice: &'index [Offset],
}

impl<'index> Query<'index> {
    pub fn from_slice(slice: &'index [Offset]) -> Self {
        Self { slice }
    }

    pub fn range(&self, range: ops::Range<usize>) -> Self {
        Self::from_slice(&self.slice[range])
    }

    pub fn range_from(&self, range_from: ops::RangeFrom<usize>) -> Self {
        Self::from_slice(&self.slice[range_from])
    }

    pub fn len(&self) -> usize {
        self.slice.len()
    }
}

impl<'index> Query<'index> {
    #[inline]
    pub fn get_line_offset(&self, line_no: usize) -> Option<Offset> {
        self.slice.get(line_no).copied()
    }

    /// Locate line number from byte offset
    #[inline]
    pub fn locate_line(&self, offset: Offset) -> Option<usize> {
        binary_search_between(&self.slice, offset)
    }

    /// Locate line-column numbers from byte offset
    pub fn locate(&self, offset: Offset) -> Option<line_column::ZeroBased> {
        let line = self.locate_line(offset)?;
        let line_offset = self.get_line_offset(line).unwrap();
        let col = offset - line_offset;

        Some((line, col.raw()).into())
    }

    /// Encode byte offset from line-column location
    pub fn encode(&self, location: line_column::ZeroBased) -> Option<Offset> {
        let (line, col) = location.raw();
        if let Some(offset) = self.get_line_offset(line) {
            return Some(offset + col);
        }
        None
    }
}

/// SAFETY: Assuming `xs` is ordered, try to find a interval where `x` lies.  
/// returns the start index of interval
fn binary_search_between<A: Ord + Copy>(xs: &[A], x: A) -> Option<usize> {
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_binary_search() {
        let xs = [2, 4, 6];
        let i = binary_search_between(&xs, 3);
        assert_eq!(i, Some(0));

        let i = binary_search_between(&xs, 1);
        assert_eq!(i, None);
    }
}

use std::ops;

use crate::location::{Offset, line_column};

/// An index built for fast convertion between byte offsets and line-column locations.
///
/// NOTE: the last element of `line_offsets` should be considered as a fake offset (sentinel):
///
/// `self.line_offsets: [line0, line1, ..., EOF]`
///
/// By the word 'fake', we means that if some offset >= the last offset,
/// it should be seen as exceding the index since the last element only marks ending.
///
/// # Invariants
///
/// - `index` is never empty.
/// - `index[0] == 0`.
/// - `index` is monotonically increasing.
/// - `index` is append-only (no removals or mutations).
/// - A sentinel EOF offset is always present as the last element.
///
/// Therefore:
/// - Valid logical line indices are `0..self.count()`,
///   where `count() = index.len() - 1`.
/// - For any valid line `i`, the byte range is:
///   `[index[i], index[i + 1])`.
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
/// NOTE: Since the `Query` can be sliced, we carefully store an extra beginning offset to trace slice beginning:
///
/// `self.slice: [line0, line1, ..., EOF]`
///
/// One should keep in mind that all line numbers passed into query methods should be numbers **from the original [Index]**
/// and the slice is *non-empty*.
#[derive(Debug)]
pub struct Query<'index> {
    /// the beginning line number,
    /// used to recover line number since `slice` lacks beginning offset from the original [Index]
    begin: usize,

    slice: &'index [Offset],
}

impl<'index> Query<'index> {
    /// This builder is internal since we don't want users to accidentally build a Query with empty slice
    fn new(begin: usize, slice: &'index [Offset]) -> Self {
        Self { begin, slice }
    }

    /// build from raw slice, assuming the beginning is zero
    fn from(slice: &'index [Offset]) -> Self {
        Self { begin: 0, slice }
    }

    /// Returns a zero-copy view over a subrange of lines.
    ///
    /// The input `range` is interpreted over the *logical* line indices,
    /// i.e. `0..self.count()`, where `count() = self.slice.len() - 1`.
    ///
    /// Internally, `self.slice` stores a sentinel EOF offset as the last element:
    /// `[line0, line1, ..., EOF]`.
    ///
    /// To preserve this invariant, the returned slice includes one extra element
    /// at the end (the sentinel), so the actual slice is:
    ///
    /// ```text
    /// slice[range.start .. range.end + 1]
    /// ```
    ///
    /// This ensures that every line `i` in the resulting view satisfies:
    ///
    /// ```text
    /// line i = [slice[i], slice[i+1])
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `range.start > range.end`
    /// - `range.end > self.count()`
    ///
    /// These conditions indicate a violation of the API contract.
    ///
    /// # Performance
    ///
    /// This operation is zero-copy and does not allocate.
    ///
    /// Invariant:
    /// - self.slice.len() >= 1
    /// - last element is EOF
    /// - valid line indices: 0..self.slice.len()-1
    pub fn range(&self, range: ops::Range<usize>) -> Self {
        assert!(range.start <= range.end);
        assert!(range.end <= self.count());

        let range = range.start..range.end + 1;
        Self::new(range.start, &self.slice[range])
    }

    /// Returns a zero-copy view over lines starting from `range_from.start`
    /// to the end.
    ///
    /// The input is interpreted over the *logical* line indices,
    /// i.e. `0..self.count()`.
    ///
    /// Internally, `self.slice` always ends with a sentinel EOF offset:
    /// `[line0, line1, ..., EOF]`.
    ///
    /// Therefore, slicing with `slice[start..]` naturally preserves the sentinel,
    /// and no additional adjustment is needed.
    ///
    /// The resulting view satisfies:
    ///
    /// ```text
    /// line i = [slice[i], slice[i+1])
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if:
    /// - `range_from.start > self.count()`
    ///
    /// # Edge Cases
    ///
    /// - If `range_from.start == self.count()`, the returned slice contains only
    ///   the EOF sentinel. This represents an empty range of lines.
    ///
    /// # Performance
    ///
    /// This operation is zero-copy and does not allocate.
    ///
    /// invariant: self.slice always ends with EOF
    /// so slice[start..] always contains a valid sentinel
    pub fn range_from(&self, range_from: ops::RangeFrom<usize>) -> Self {
        assert!(range_from.start <= self.count());

        Self::new(range_from.start, &self.slice[range_from])
    }

    /// Count the total lines
    pub fn count(&self) -> usize {
        debug_assert!(!self.slice.is_empty());
        self.slice.len() - 1
    }
}

impl Query<'_> {
    /// Given the number of line `line_no`, returns its start offset.
    #[inline]
    pub fn line_offset(&self, line_no: usize) -> Option<Offset> {
        if line_no < self.begin {
            return None;
        }
        let line_no = line_no - self.begin;
        self.slice.get(line_no).copied()
    }

    /// Given the number of line `line_no`, returns its start..end offset span.
    pub fn line_span(&self, line_no: usize) -> Option<ops::Range<Offset>> {
        let start = self.line_offset(line_no)?;
        let end = self.line_offset(line_no + 1)?; // it's safe here since we have a fake ending

        Some(start..end)
    }

    /// The beginning of the whole query range
    #[inline]
    pub fn beginning(&self) -> Option<Offset> {
        self.line_offset(0)
    }

    /// The ending of the whole query range
    #[inline]
    pub fn ending(&self) -> Option<Offset> {
        self.slice.last().copied()
    }

    /// check if contains the given offset
    pub fn contains(&self, offset: Offset) -> bool {
        let Some(begin) = self.beginning() else {
            return false;
        };
        let Some(end) = self.ending() else {
            return false;
        };

        offset >= begin && offset < end
    }

    /// Locate line number from byte offset
    #[inline]
    pub fn locate_line(&self, offset: Offset) -> Option<usize> {
        binary_search_between(&self.slice, offset).map(|n| self.begin + n)
    }

    /// Locate line-column numbers from byte offset
    pub fn locate(&self, offset: Offset) -> Option<line_column::ZeroBased> {
        let line = self.locate_line(offset)?;
        let line_offset = self.line_offset(line).unwrap();
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

        if let Some(offset) = self.line_offset(line) {
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
    use quickcheck_macros::quickcheck;

    fn linear_search_between<A: Ord + Copy>(xs: &[A], x: A) -> Option<usize> {
        if xs.len() <= 1 {
            return None;
        }

        for i in 0..xs.len() - 1 {
            if xs[i] <= x && x < xs[i + 1] {
                return Some(i);
            }
        }
        None
    }

    #[quickcheck]
    fn prop_binary_search_between(mut xs: Vec<i64>, x: i64) -> bool {
        xs.sort();
        xs.dedup();

        if xs.len() < 2 {
            return true;
        }

        let res0 = linear_search_between(&xs, x);
        let res1 = binary_search_between(&xs, x);

        res0 == res1
    }

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

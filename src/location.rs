//! Location types
//!
//! [Offset] is byte based offset.
//!
//! Line-column locations are further divided as. [line_column::ZeroBased] and [line_column::OneBased].

use std::ops;

/// Zero-based offset of bytes, only BYTES
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(pub usize);

impl Offset {
    pub fn new(raw: usize) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> usize {
        self.0
    }

    fn plus(self, that: Self) -> Self {
        Self(self.raw() + that.raw())
    }

    fn minus(self, that: Self) -> Self {
        Self(self.raw() - that.raw())
    }
}

impl From<usize> for Offset {
    fn from(value: usize) -> Self {
        Offset(value)
    }
}

impl From<Offset> for usize {
    fn from(value: Offset) -> Self {
        value.raw()
    }
}

impl Default for Offset {
    fn default() -> Self {
        Self(0)
    }
}

impl ops::Add for Offset {
    type Output = Offset;

    fn add(self, rhs: Offset) -> Self::Output {
        self.plus(rhs)
    }
}

impl ops::Add<usize> for Offset {
    type Output = Offset;

    fn add(self, rhs: usize) -> Self::Output {
        self.plus(Offset(rhs))
    }
}

impl ops::AddAssign for Offset {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.raw();
    }
}

impl ops::AddAssign<usize> for Offset {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl ops::Sub for Offset {
    type Output = Offset;

    fn sub(self, rhs: Self) -> Self::Output {
        self.minus(rhs)
    }
}

impl ops::Sub<usize> for Offset {
    type Output = Offset;

    fn sub(self, rhs: usize) -> Self::Output {
        self.minus(rhs.into())
    }
}

/// For convenience
pub trait OffsetRangeExt {
    fn to_usize(self) -> ops::Range<usize>;
}

impl OffsetRangeExt for ops::Range<Offset> {
    fn to_usize(self) -> ops::Range<usize> {
        self.start.raw()..self.end.raw()
    }
}

pub mod line_column {
    use std::num::NonZeroUsize;

    /// Zero-based (line, column) location
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct ZeroBased {
        pub line: usize,
        pub column: usize,
    }

    impl ZeroBased {
        pub fn new(line: usize, column: usize) -> Self {
            Self { line, column }
        }

        pub fn raw(&self) -> (usize, usize) {
            (self.line, self.column)
        }

        /// Get one-based line and column numbers
        pub fn one_based(&self) -> OneBased {
            unsafe {
                OneBased {
                    line: NonZeroUsize::new_unchecked(self.line + 1),
                    column: NonZeroUsize::new_unchecked(self.column + 1),
                }
            }
        }
    }

    impl From<(usize, usize)> for ZeroBased {
        fn from((line, column): (usize, usize)) -> Self {
            ZeroBased { line, column }
        }
    }

    /// One-based (line, column) location
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct OneBased {
        pub line: NonZeroUsize,
        pub column: NonZeroUsize,
    }

    impl OneBased {
        pub fn new(line: usize, column: usize) -> Option<Self> {
            let line = NonZeroUsize::new(line)?;
            let column = NonZeroUsize::new(column)?;
            Some(Self { line, column })
        }

        pub fn raw(&self) -> (usize, usize) {
            (self.line.get(), self.column.get())
        }

        /// Get zero-based line and column numbers
        pub fn zero_based(&self) -> ZeroBased {
            ZeroBased {
                line: self.line.get() - 1,
                column: self.column.get() - 1,
            }
        }
    }

    impl From<(NonZeroUsize, NonZeroUsize)> for OneBased {
        fn from((line, column): (NonZeroUsize, NonZeroUsize)) -> Self {
            OneBased { line, column }
        }
    }
}

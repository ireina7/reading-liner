/// Zero-based offset of bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Offset(usize);

impl Offset {
    pub fn new(raw: usize) -> Self {
        Self(raw)
    }

    pub fn raw(&self) -> usize {
        self.0
    }
}

impl From<usize> for Offset {
    fn from(value: usize) -> Self {
        Offset::new(value)
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

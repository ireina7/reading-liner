use quickcheck::{Arbitrary, quickcheck};
use reading_liner::location::line_column::ZeroBased;
use reading_liner::{Index, Stream, location::Offset};
use std::io;
use std::io::Read;

quickcheck! {
    /// Built index is valid
    fn check_index(src: Source) -> bool {
        let s = src.build();
        let checker = Checker::from_bytes(s.as_bytes());
        let index = build_index(s.as_bytes());

        checker.index == index.into_offsets()
    }

    /// Querying location is valid
    fn check_location(src: Source) -> bool {
        let s = src.build();
        let checker = Checker::from_bytes(s.as_bytes());
        let index = build_index(s.as_bytes());

        const LOOP: usize = 1000;
        for _ in 0..LOOP {
            let i = rand::random_range(0..s.len() + 10);
            let offset = Offset(i);

            let loc0 = checker.locate(offset);
            let loc1 = index.query().locate(offset);
            if loc0 != loc1 {
                return false;
            }
        }
        true
    }
}

/// Firstly we need to build a cumbersome but restrict index and query checker
#[derive(Debug)]
struct Checker {
    index: Vec<Offset>,
}

impl Checker {
    fn from_bytes(bs: &[u8]) -> Self {
        let mut index = vec![0.into()];

        let mut cnt = Offset(0);
        for b in bs {
            cnt += 1;
            if *b == b'\n' {
                // here we still add an fake beginning if `\n` is the last byte which is useful actually...
                index.push(cnt);
            }
        }

        // if the last byte is not '\n', add an extra fake ending
        if !bs.is_empty() && bs.last() != Some(&b'\n') {
            index.push(cnt);
        }

        Self { index }
    }

    #[inline]
    pub fn get_line_offset(&self, line_no: usize) -> Option<Offset> {
        self.index.get(line_no).copied()
    }

    /// a linear search which can be validated easily
    fn locate_line(&self, offset: Offset) -> Option<usize> {
        let mut before = 0;
        for (i, &end) in self.index.iter().enumerate() {
            // found the interval
            if offset < end {
                return Some(before);
            }
            before = i;
        }
        None
    }

    fn locate(&self, offset: Offset) -> Option<ZeroBased> {
        let line = self.locate_line(offset)?;
        let line_offset = self.get_line_offset(line)?;
        let col = offset - line_offset;

        Some((line, col.raw()).into())
    }
}

#[derive(Debug, Clone)]
struct Source {
    lines: Vec<String>,
}

impl Source {
    fn build(self) -> String {
        let mut s = String::new();
        for line in self.lines {
            s.push_str(&line);
        }
        s.push('\n');
        s
    }
}

impl Arbitrary for Source {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        let cnt_line = usize::arbitrary(g) % 100 + 10; // 10..100
        let mut lines = Vec::with_capacity(10);
        for _ in 0..cnt_line {
            let line = String::arbitrary(g);
            lines.push(line);
        }

        Self { lines }
    }
}

fn build_index(s: &[u8]) -> Index {
    let mut index = Index::new();
    let stream = Stream::new(s, &mut index);
    let mut reader = io::BufReader::new(stream);
    let mut buf = String::new();
    reader.read_to_string(&mut buf).unwrap();

    index
}

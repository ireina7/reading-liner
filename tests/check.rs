use quickcheck::Arbitrary;
use quickcheck_macros::quickcheck;
use reading_liner::location::line_column::ZeroBased;
use reading_liner::{Index, Stream, location::Offset};
use std::io::Read;
use std::io::{self, BufReader};

const LOOP: usize = 1000;

/// Check the built index is valid
#[quickcheck]
fn check_index(src: Source) -> bool {
    let s = src.build();
    let checker = Checker::from_bytes(s.as_bytes());
    let index = build_index(s.as_bytes());

    checker.index == index.into_offsets()
}

/// Check Querying location is valid
#[quickcheck]
fn check_location(src: Source) -> bool {
    let s = src.build();
    let checker = Checker::from_bytes(s.as_bytes());
    let index = build_index(s.as_bytes());

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

/// Check incremental Querying location is valid
#[quickcheck]
fn check_incremental(src: Source) -> bool {
    let s = src.build();
    let checker = Checker::from_bytes(s.as_bytes());
    let mut index = Index::new();
    let mut stream = Stream::new(s.as_bytes(), &mut index);
    let mut buf = vec![b'\0'; 10];

    for _ in 0..LOOP {
        let i = rand::random_range(0..s.len() + 10);
        let offset = Offset(i);

        let loc0 = checker.locate(offset);
        let loc1 = stream.locate(offset, &mut buf).ok();
        if loc0 != loc1 {
            return false;
        }

        if let Some(loc0) = loc0
            && loc0.line > 0
        {
            let i = rand::random_range(0..loc0.line);
            let loc3 = stream.query().range_from(loc0.line - i..).locate(offset);
            let Some(loc3) = loc3 else {
                return false;
            };
            if loc0 != loc3 {
                return false;
            }

            let max = stream.get_index().count();
            if max - loc0.line > 1 {
                let i = rand::random_range(1..max - loc0.line);
                let loc3 = stream.query().range_from(loc0.line + i..).locate(offset);
                if loc3.is_some() {
                    return false;
                }
            }
        }
    }
    true
}

/// Test reading file IO
#[test]
fn test_stream_file() {
    let file = std::fs::File::open("./tests/xiao_yao_you.txt").expect("Failed to open file");
    let mut s = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut s).expect("failed to read file");
    let checker = Checker::from_bytes(s.as_bytes());

    let file = std::fs::File::open("./tests/xiao_yao_you.txt").expect("Failed to open file");
    let mut index = Index::new();
    let mut stream = Stream::new(file, &mut index);
    let mut buf = vec![b'\0'; 512];

    for _ in 0..LOOP {
        let i = rand::random_range(0..s.len() + 10);
        let offset = Offset(i);
        let loc0 = checker.locate(offset);
        let loc1 = stream.locate(offset, &mut buf);
        assert_eq!(loc0, loc1.ok());
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

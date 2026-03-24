# reading-liner

[![Crates.io][crate-badge]][crate-url]
[![Docs.rs][docs-badge]][docs-url]

[crate-url]: https://crates.io/crates/reading-liner
[crate-badge]: https://img.shields.io/crates/v/reading-liner?logo=rust&logoColor=white
[docs-url]: https://docs.rs/reading-liner
[docs-badge]: https://img.shields.io/docsrs/reading-liner?logo=docsdotrs&logoColor=white&label=docs

A Rust crate for streaming construction of line/column indices over text sources.

It enables on-the-fly (one-pass) mapping from byte offsets to (line, column) locations while reading from a stream (e.g. a file), without requiring the entire input to be loaded into memory.

It provides a `Stream` reader which can convert between byte offset and line-column numbers.
Support any type which implements `std::io::Read`.

The whole design is based on an `Index`, 
which is composed of line information to convert between byte offsets and line-column locations.
One perticular usage is to use the `Stream` as a builder of `Index` or 
you can also use it when lazily reading and convert locations at the same time.

This lib should be used at *low-level abstraction*.


## 📍 Features
- Offset → (line, column) mapping
- True streaming support (no full file buffering required)
- Consistent with in-memory indexing
- Single-pass construction
- Designed to integrate with parsers and error reporting tools (e.g. [codespan_reporting](https://github.com/brendanzab/codespan))

## 🚀 Motivation
Most existing approaches follow this pattern:
```rust
let src = std::fs::read_to_string("file.txt")?;
let index = build_index(src.as_bytes());
```
❌ This has several drawbacks:
- Requires loading the entire file into memory
- Not suitable for streaming or large inputs
- Forces a separation between IO and indexing (often multiple passes)

✅ This crate takes a different approach

It builds the index as the data is being read, enabling:
- Parsing directly from a stream
- Accurate location tracking without a second pass
- Reduced memory footprint for large files

## 🎯 Use Cases
- Compilers and interpreters
- Incremental / streaming parsers
- Large file processing
- Custom diagnostic tools
- Language tooling (LSP, linters, etc.)

## 📄 Documentation
The API is documented at [https://docs.rs/reading-liner](https://docs.rs/reading-liner).


## 📦 Examples

### Load and build index
```rust
use reading_liner::{Stream, location::Offset, Index};
use std::io::Read;
use std::{fs, io};

fn example() -> io::Result<()> {
    // build stream
    let file = fs::File::open("foo.rs")?;
    let mut index = Index::new();
    let stream = Stream::new(file, &mut index);

    // wrap BufReader
    let mut reader = io::BufReader::new(stream);
    let mut buf = String::new();
    reader.read_to_string(&mut buf)?;

    // use the built index
    let line_index = index.query().locate(Offset(20)).unwrap();
    dbg!(line_index.one_based());
    Ok(())
}
```

### Build index while loading
```rust
use reading_liner::{Stream, location::Offset, Index};
use std::io::Read;
use std::{fs, io};

fn example(offset: Offset) -> io::Result<()> {
    let file = std::fs::File::open("foo.rs")?;
    let mut index = Index::new();
    let mut stream = Stream::new(file, &mut index);
    let mut buf = vec![b'\0'; 1024];

    let loc = stream.locate(offset, &mut buf); // on-demand loading
    dbg!(loc);
}
```

For more examples, please refer to [tests](./tests/check.rs)


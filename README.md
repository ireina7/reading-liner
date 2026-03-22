# reading-liner
Reading and convert offset and line-column location.  

A Stream reader which can convert between byte offset and line-column numbers.
Support any type which implements `io::Read`.

The whole design is based on an `Index`, 
which is composed of line information to convert between byte offsets and line-column locations.
One perticular usage is to use the `Stream` as a builder of `Index` or 
you can also use it when lazily reading and convert locations at the same time.

This lib should be used at *low-level abstraction*.

## Documentation
The API is documented: [https://docs.rs/reading-liner](https://docs.rs/reading-liner)

## Core methods
### Immutable queries (best practice)
- locate line-column location from byte offset:
    + `Query::locate(&self, Offset) -> Option<line_column::ZeroBased>`
- encode offset from line-column location
    + `Query::encode(&self, line_column::ZeroBased) -> Option<Offset>`

### Mutable queries
The names are the same with immutable ones, 
but be careful since these mutable queries may read bytes implicitly.
- locate line-column location from byte offset:
    + `Stream::locate(&mut self, Offset) -> io::Result<line_column::ZeroBased>`
- encode offset from line-column location
    + `Stream::encode(&mut self, line_column::ZeroBased) -> io::Result<Offset>`

## Example
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


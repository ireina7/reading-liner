# reading-liner
Reading and convert offset and line-column location.  

A Stream reader which can convert between byte offset and line-column numbers.
Support any type which implements `io::Read`.

## Core methods
- `Stream::line_index(&mut self, offset:: location::Offset) -> io::Result<location::line_column::ZeroBased>`
- `Stream::offset_of(&mut self, line_index: location::line_column::ZeroBased) -> io::Result<location::Offset>`

- `Stream::read_line_index(&mut self, offset:: location::Offset, buf: &mut Vec<u8>) -> io::Result<location::line_column::ZeroBased>`
- `Stream::read_offset_of(&mut self, line_index: location::line_column::ZeroBased, buf: &mut Vec<u8>) -> io::Result<location::Offset>`

## Example
```rust
use stream_locate_converter::Stream;
use stream_locate_converter::location;
use std::fs;

fn main() -> io::Result<()> {
    let file = fs::File::open("foo.rs")?;
    let mut stream = Stream::from(file);

    let offset = location::Offset::new(20);
    let line_index = stream.line_index(offset)?;

    let (line, col) = line_index.one_based().raw();
    println!("The offset is on line {line}, column {col}.");
}
```


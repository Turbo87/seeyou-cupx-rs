# Architecture & Design

This document describes the design decisions, key abstractions, and implementation strategies used in the `seeyou-cupx` library.

## Overview

The library provides Rust APIs for reading and writing CUPX files, which are used in aviation/gliding applications to store waypoint data along with associated pictures. The core challenge is that CUPX files contain two concatenated ZIP archives in a single file, requiring careful parsing to separate and access each archive independently.

### Design Goals

- **Zero-copy where possible**: Avoid unnecessary data copying when reading archives
- **Streaming-friendly**: Support both file and in-memory I/O through generic `Read + Seek` traits
- **Type safety**: Leverage Rust's type system to prevent invalid file construction
- **Minimal dependencies**: Keep the dependency tree small and focused
- **Correct error handling**: Distinguish between parsing warnings and fatal errors

## Module Organization

```
seeyou-cupx/
├── src/
│   ├── lib.rs              # Public API surface
│   ├── reader.rs           # CupxFile: Parsing and reading CUPX files
│   ├── writer.rs           # CupxWriter: Creating CUPX files
│   ├── limited_reader.rs   # LimitedReader: Byte range restriction wrapper
│   └── error.rs            # Error and Warning types
```

### Module Responsibilities

- **`reader.rs`**: Contains the `CupxFile` struct and all parsing logic, including the EOCD search algorithm
- **`writer.rs`**: Contains `CupxWriter` builder pattern for constructing CUPX files with pictures
- **`limited_reader.rs`**: Provides `LimitedReader<R, B>`, a critical abstraction for working with concatenated archives
- **`error.rs`**: Defines `Error` (fatal) and `Warning` (non-fatal) types

## Key Abstractions

### LimitedReader

`LimitedReader<R, B>` is a wrapper that restricts a `Read + Seek` implementation to only access a specific byte range. This is essential for parsing CUPX files because:

1. CUPX files contain two ZIP archives concatenated together
2. ZIP parsers expect to read an entire archive from start to end
3. Without byte range limitation, the ZIP parser would read past the first archive into the second

**How it works**:
- Wraps any `R: Read + Seek` with a `RangeBounds<u64>`
- Translates all read/seek operations to stay within the specified range
- Returns EOF when attempting to read past the range boundary
- Makes the underlying reader appear as if only the byte range exists

**Example use**:
```rust
// Read only bytes 0..1000 from a file
let limited = LimitedReader::new(file, 0..1000)?;
let archive = ZipArchive::new(limited)?; // ZIP parser only sees those bytes
```

### Two-Phase Parsing Strategy

The reader uses a two-phase approach:

**Phase 1: Archive Boundary Detection**
1. Search the file backwards for End of Central Directory (EOCD) signatures
2. Find the boundary between the two ZIP archives
3. Determine if pictures archive exists

**Phase 2: Archive Reading**
1. Create `LimitedReader` for the points archive (second ZIP)
2. Parse `POINTS.CUP` file and extract waypoint/task data
3. Create `LimitedReader` for the pics archive (first ZIP) if it exists
4. Keep pics archive accessible for picture reading

This separation ensures the file is scanned only once for boundaries, then accessed on-demand.

## ZIP File Format & EOCD Search

### Key ZIP Concept

ZIP files end with an End of Central Directory (EOCD) record containing the signature `PK\x05\x06` (at offset 0) and a comment length field (at offset 20). The EOCD appears exactly once per ZIP archive. CUPX files contain two concatenated ZIPs, so two EOCD signatures exist.

### Boundary Detection Algorithm

The parser finds the boundary between archives by searching backwards for EOCD signatures:

1. **Chunked backward search**: Read 64KB chunks from file end, searching for `PK\x05\x06` using `memchr::memmem`
2. **Track positions**: Record the last two EOCD positions found
3. **Calculate boundary**: `second_eocd_offset + 22 + comment_length` (read comment length from EOCD bytes 20-21)

**Archive ranges**:
- Two EOCDs found: Pics `[0..boundary)`, Points `[boundary..end]`
- One EOCD found: No pics (warning), Points `[0..end]`
- Zero EOCDs: Error

Chunked search limits memory to 64KB regardless of file size.

## Reading Flow

```
User calls CupxFile::from_path()
    ↓
Open file as Read + Seek
    ↓
Search backwards for EOCD signatures (chunked)
    ↓
Calculate boundary between archives
    ↓
Create LimitedReader for points archive (from boundary to EOF)
    ↓
Parse POINTS.CUP using ZipArchive + seeyou-cup parser
    ↓
Create LimitedReader for pics archive (from 0 to boundary) if exists
    ↓
Return CupxFile with pics archive accessible
    ↓
User calls read_picture() or picture_names()
    ↓
Access pics archive on-demand via LimitedReader
```

## Writing Flow

```
User creates CupxWriter::new(&cup_file)
    ↓
User adds pictures via add_picture()
    ↓
Pictures stored as HashMap<filename, PictureSource>
    ↓
User calls write() or write_to_path()
    ↓
Validate all filenames (no empty, no path separators)
    ↓
Write pics archive:
    ├── Create ZipWriter
    ├── For each picture: add to ZIP as "pics/{filename}"
    └── Finish pics ZIP
    ↓
Write points archive:
    ├── Create in-memory ZipWriter
    ├── Add POINTS.CUP from CupFile
    ├── Finish points ZIP to buffer
    └── Append buffer to output
    ↓
Result: Valid CUPX file (pics.zip + points.zip concatenated)
```

### Writer Design Notes

**In-memory points buffer**: The points archive is built entirely in memory before writing. This is acceptable because:
- CUP files are typically small (text-based waypoint data)
- Building in memory simplifies the API (no need for two-pass writing)
- Memory usage is predictable and bounded

**Pictures from paths vs bytes**: `PictureSource` enum allows both:
- `PictureSource::Path`: Read from filesystem during write (avoids loading into memory)
- `PictureSource::Bytes`: Already in memory (useful for generated/modified images)

**Duplicate handling**: Using `HashMap` means adding a picture with the same filename twice replaces the first. This matches intuitive builder pattern behavior.

## Generic Design Patterns

### Generic over Read + Seek

Both `CupxFile<R>` and `LimitedReader<R, B>` are generic over the reader type:

```rust
pub struct CupxFile<R> {
    cup_file: CupFile,
    pics_archive: Option<ZipArchive<LimitedReader<R, Range<u64>>>>,
}
```

**Benefits**:
- Works with `File`, `Cursor<Vec<u8>>`, `BufReader`, or any custom reader
- Enables testing with in-memory data
- Allows future async implementations without API breakage

**Convenience methods**: `CupxFile<File>` gets special methods like `from_path()` to reduce boilerplate for common cases.

### Builder Pattern for Writing

`CupxWriter` uses the builder pattern with method chaining:

```rust
CupxWriter::new(&cup_file)
    .add_picture("a.jpg", path_a)
    .add_picture("b.jpg", path_b)
    .write_to_path("output.cupx")?;
```

This provides:
- Fluent, readable API
- Flexibility in picture sources
- Compile-time enforcement of required data (CupFile must be provided)

## Error Handling Philosophy

The library distinguishes between **errors** (fatal) and **warnings** (non-fatal):

### Errors (`Error` enum)
- I/O failures
- Malformed ZIP archives
- Invalid CUPX structure (missing EOCD signatures)
- Invalid filenames in writer
- CUP parsing errors

All operations return `Result<T, Error>` for propagation.

### Warnings (`Warning` enum)
- No pictures archive found (still valid CUPX)
- CUP parse warnings (logged but recoverable)

Warnings are collected and returned alongside the result: `Result<(CupxFile, Vec<Warning>), Error>`.

**Rationale**: Many CUPX files in the wild have minor issues but are still usable. Warnings allow users to:
- Log issues without failing
- Decide whether to treat warnings as errors in their context
- Provide better user feedback than silent success or hard failure

## Dependencies

The library has minimal runtime dependencies:

- **`zip`**: ZIP archive reading/writing (with only `deflate` feature enabled)
- **`seeyou-cup`**: CUP file format parsing/writing
- **`thiserror`**: Ergonomic error type derivation
- **`memchr`**: Fast EOCD signature search using SIMD when available

Dev dependencies include `criterion` (benchmarking) and `insta` (snapshot testing).

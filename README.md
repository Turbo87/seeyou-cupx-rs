# seeyou-cupx

A Rust library for parsing and writing SeeYou CUPX files, commonly used in aviation and gliding for waypoints with attached pictures. 

CUPX files consist of two concatenated ZIP archives: a "pics" archive containing images and a "points" archive containing a `POINTS.CUP` file with waypoint and task data. For more details, see the [official CUPX file format specification](https://downloads.naviter.com/docs/SeeYou_CUPX_file_format.pdf).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
seeyou-cupx = "0.1.0"
```

## Usage

### Reading CUPX files

```rust,no_run
use seeyou_cupx::CupxFile;
use std::io::Read;

let (mut cupx, warnings) = CupxFile::from_path("waypoints.cupx")?;

// Access waypoint data
for waypoint in cupx.waypoints() {
    println!("{}: {}, {}", waypoint.name, waypoint.latitude, waypoint.longitude);
}

// Access pictures
for pic_name in cupx.picture_names() {
    println!("Picture: {}", pic_name);
}

// Read a specific picture
let mut reader = cupx.read_picture("airport.jpg")?;
let mut buffer = Vec::new();
reader.read_to_end(&mut buffer)?;

# Ok::<(), seeyou_cupx::Error>(())
```

### Writing CUPX files

```rust,no_run
use seeyou_cupx::cup::CupFile;
use seeyou_cupx::CupxWriter;
use std::path::Path;

CupxWriter::new(CupFile::default())
    .add_picture("airport.jpg", Path::new("images/airport.jpg"))
    .add_picture("runway.jpg", Path::new("images/runway.jpg"))
    .write_to_path("output.cupx")?;

# Ok::<(), seeyou_cupx::Error>(())
```

### Encoding Support

By default, the library automatically detects the text encoding of CUP files. If you know the encoding beforehand:

```rust,no_run
use seeyou_cupx::cup::Encoding;
use seeyou_cupx::CupxFile;

let (cupx, warnings) = CupxFile::from_path_with_encoding("waypoints.cupx", Encoding::Utf8)?;

# Ok::<(), seeyou_cupx::Error>(())
```

## Dependencies

This library uses [seeyou-cup](https://github.com/Turbo87/seeyou-cup-rs) for parsing and writing the underlying CUP file format.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dually licensed as above, without any additional terms or conditions.

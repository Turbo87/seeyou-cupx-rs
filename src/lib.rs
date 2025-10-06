#![doc = include_str!("../README.md")]

mod error;
mod limited_reader;
mod reader;
mod writer;

pub use error::{Error, Warning};
pub use reader::CupxFile;
pub use writer::{CupxWriter, PictureSource};

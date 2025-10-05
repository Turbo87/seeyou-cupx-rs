use seeyou_cup::{CupEncoding, CupFile, Task, Waypoint};
use std::fs::File;
use std::io::{Read, Seek};
use std::path::Path;

pub struct CupxFile {
    cup_file: CupFile,
}

impl CupxFile {
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<(Self, Vec<Warning>), Error> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }
    pub fn from_path_with_encoding<P: AsRef<Path>>(
        path: P,
        encoding: CupEncoding,
    ) -> Result<(Self, Vec<Warning>), Error> {
        let file = File::open(path)?;
        Self::from_reader_with_encoding(file, encoding)
    }

    pub fn from_reader<R: Read + Seek>(reader: R) -> Result<(Self, Vec<Warning>), Error> {
        todo!()
    }
    pub fn from_reader_with_encoding<R: Read + Seek>(
        reader: R,
        encoding: CupEncoding,
    ) -> Result<(Self, Vec<Warning>), Error> {
        todo!()
    }

    pub fn cup_file(&self) -> &CupFile {
        &self.cup_file
    }

    pub fn waypoints(&self) -> &[Waypoint] {
        &self.cup_file().waypoints
    }

    pub fn tasks(&self) -> &[Task] {
        &self.cup_file().tasks
    }

    // /// Get reader for image by filename (without "pics/" prefix)
    // /// Returns error if image doesn't exist
    // /// Only one image can be read at a time (requires &mut self)
    // pub fn image(&mut self, filename: &str) -> Result<impl Read + '_, Error> {
    //     todo!()
    // }
    //
    // /// Iterator over all available image filenames (without "pics/" prefix)
    // pub fn image_names(&self) -> impl Iterator<Item = String> {
    //     todo!()
    // }
}

#[derive(Debug, Clone)]
pub struct Warning {
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    Cup(#[from] seeyou_cup::Error),
}

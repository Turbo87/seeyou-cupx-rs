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

    pub fn from_reader<R: Read + Seek>(mut reader: R) -> Result<(Self, Vec<Warning>), Error> {
        const EOCD_SIGNATURE: &[u8] = b"PK\x05\x06";
        const EOCD_MIN_SIZE: u64 = 22;
        const MAX_COMMENT_SIZE: u64 = 65535;

        // Get file size
        reader.seek(std::io::SeekFrom::Start(0))?;
        let file_size = reader.seek(std::io::SeekFrom::End(0))?;

        // Find both EOCD signatures by searching backwards
        let search_size = (EOCD_MIN_SIZE + MAX_COMMENT_SIZE).min(file_size);
        let search_start = file_size - search_size;

        reader.seek(std::io::SeekFrom::Start(search_start))?;
        let mut buffer = vec![0u8; search_size as usize];
        reader.read_exact(&mut buffer)?;

        // Find all EOCD signatures
        let mut eocd_offsets = Vec::new();
        for i in 0..buffer.len().saturating_sub(EOCD_SIGNATURE.len()) {
            if &buffer[i..i + EOCD_SIGNATURE.len()] == EOCD_SIGNATURE {
                eocd_offsets.push(search_start + i as u64);
            }
        }

        if eocd_offsets.len() < 2 {
            return Err(Error::InvalidCupx);
        }

        // The last EOCD is for the second ZIP, second-to-last is for the first ZIP
        let first_eocd_offset = eocd_offsets[eocd_offsets.len() - 2];
        let second_eocd_offset = eocd_offsets[eocd_offsets.len() - 1];

        dbg!(first_eocd_offset);
        dbg!(second_eocd_offset);

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
    #[error("Invalid CUPX file: could not find two ZIP archives")]
    InvalidCupx,
}

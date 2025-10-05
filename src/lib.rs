use limited_reader::LimitedReader;
use seeyou_cup::{CupEncoding, CupFile, Task, Waypoint};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

mod limited_reader;

pub struct CupxFile<R> {
    cup_file: CupFile,
    pics_archive: zip::ZipArchive<LimitedReader<R>>,
}

impl CupxFile<File> {
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
}

impl<R: Read + Seek> CupxFile<R> {
    pub fn from_reader(reader: R) -> Result<(Self, Vec<Warning>), Error> {
        Self::from_reader_inner(reader, None)
    }

    pub fn from_reader_with_encoding(
        reader: R,
        encoding: CupEncoding,
    ) -> Result<(Self, Vec<Warning>), Error> {
        Self::from_reader_inner(reader, Some(encoding))
    }

    fn from_reader_inner(
        mut reader: R,
        encoding: Option<CupEncoding>,
    ) -> Result<(Self, Vec<Warning>), Error> {
        const EOCD_SIGNATURE: &[u8] = b"PK\x05\x06";
        const EOCD_MIN_SIZE: u64 = 22;
        const MAX_COMMENT_SIZE: u64 = 65535;

        // Get file size
        reader.seek(SeekFrom::Start(0))?;
        let file_size = reader.seek(SeekFrom::End(0))?;

        // Find both EOCD signatures by searching backwards
        let search_size = (EOCD_MIN_SIZE + MAX_COMMENT_SIZE).min(file_size);
        let search_start = file_size - search_size;

        reader.seek(SeekFrom::Start(search_start))?;
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
        let _second_eocd_offset = eocd_offsets[eocd_offsets.len() - 1];

        // Calculate the boundary: first EOCD offset + EOCD record length
        // Read comment length from first EOCD to get full record size
        reader.seek(SeekFrom::Start(first_eocd_offset + 20))?;
        let mut comment_len_buf = [0u8; 2];
        reader.read_exact(&mut comment_len_buf)?;
        let comment_len = u16::from_le_bytes(comment_len_buf) as u64;

        let boundary = first_eocd_offset + EOCD_MIN_SIZE + comment_len;

        // First, read the points archive to get the CUP file
        let points_reader = LimitedReader::new(reader, boundary, file_size)?;
        let mut points_archive = zip::ZipArchive::new(points_reader)?;

        let cup_file = points_archive.by_name("POINTS.CUP")?;
        let (cup_file, _) = match encoding {
            Some(encoding) => CupFile::from_reader_with_encoding(cup_file, encoding)?,
            None => CupFile::from_reader(cup_file)?,
        };

        // Now convert points_archive back to the underlying reader and create pics_archive
        let limited_reader = points_archive.into_inner();
        let reader = limited_reader.into_inner();
        let pics_reader = LimitedReader::new(reader, 0, boundary)?;
        let pics_archive = zip::ZipArchive::new(pics_reader)?;

        Ok((
            Self {
                cup_file,
                pics_archive,
            },
            Vec::new(),
        ))
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

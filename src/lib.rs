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
        Self::from_reader_inner(reader, None)
    }

    pub fn from_reader_with_encoding<R: Read + Seek>(
        reader: R,
        encoding: CupEncoding,
    ) -> Result<(Self, Vec<Warning>), Error> {
        Self::from_reader_inner(reader, Some(encoding))
    }

    fn from_reader_inner<R: Read + Seek>(
        mut reader: R,
        encoding: Option<CupEncoding>,
    ) -> Result<(Self, Vec<Warning>), Error> {
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
        let _second_eocd_offset = eocd_offsets[eocd_offsets.len() - 1];

        // Calculate the boundary: first EOCD offset + EOCD record length
        // Read comment length from first EOCD to get full record size
        reader.seek(std::io::SeekFrom::Start(first_eocd_offset + 20))?;
        let mut comment_len_buf = [0u8; 2];
        reader.read_exact(&mut comment_len_buf)?;
        let comment_len = u16::from_le_bytes(comment_len_buf) as u64;

        let boundary = first_eocd_offset + EOCD_MIN_SIZE + comment_len;
        let second_zip_size = file_size - boundary;

        // Create a limited reader for the second ZIP archive
        reader.seek(std::io::SeekFrom::Start(boundary))?;
        let mut buf = vec![0u8; second_zip_size as usize];
        reader.read_exact(&mut buf)?;

        let limited_reader = std::io::Cursor::new(buf);
        let mut points_archive = zip::ZipArchive::new(limited_reader)?;

        let cup_file = points_archive.by_name("POINTS.CUP")?;
        let (cup_file, _) = match encoding {
            Some(encoding) => CupFile::from_reader_with_encoding(cup_file, encoding)?,
            None => CupFile::from_reader(cup_file)?,
        };

        Ok((Self { cup_file }, Vec::new()))
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

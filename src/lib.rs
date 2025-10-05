use limited_reader::LimitedReader;
use seeyou_cup::{CupEncoding, CupFile, Task, Waypoint};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::Path;

mod limited_reader;

pub struct CupxFile<R> {
    cup_file: CupFile,
    pics_archive: Option<zip::ZipArchive<LimitedReader<R, Range<u64>>>>,
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

        // Find the second-to-last EOCD signature using fast pattern matching
        let mut prev = None;
        let mut current = None;

        for offset in memchr::memmem::find_iter(&buffer, EOCD_SIGNATURE) {
            prev = current;
            current = Some(search_start + offset as u64);
        }

        let mut warnings = Vec::new();

        // Determine points archive range and whether pics exist
        let pics_boundary = if let Some(first_eocd_offset) = prev {
            // Two ZIP archives found (normal case with pictures)
            // Calculate the boundary: first EOCD offset + EOCD record length
            // Read comment length from first EOCD to get full record size
            reader.seek(SeekFrom::Start(first_eocd_offset + 20))?;
            let mut comment_len_buf = [0u8; 2];
            reader.read_exact(&mut comment_len_buf)?;
            let comment_len = u16::from_le_bytes(comment_len_buf) as u64;

            let boundary = first_eocd_offset + EOCD_MIN_SIZE + comment_len;
            Some(boundary)
        } else if current.is_some() {
            // Only one ZIP archive found (no pictures)
            warnings.push(Warning::NoPicturesArchive);
            None
        } else {
            return Err(Error::InvalidCupx);
        };

        // Read the points archive to get the CUP file
        let points_start = pics_boundary.unwrap_or(0);
        let points_reader = LimitedReader::new(reader, points_start..)?;
        let mut points_archive = zip::ZipArchive::new(points_reader)?;

        let cup_file = points_archive.by_name("POINTS.CUP")?;
        let (cup_file, cup_warnings) = match encoding {
            Some(encoding) => CupFile::from_reader_with_encoding(cup_file, encoding)?,
            None => CupFile::from_reader(cup_file)?,
        };
        warnings.extend(cup_warnings.into_iter().map(|issue| Warning::CupParseIssue {
            message: issue.message().to_string(),
            line: issue.line(),
        }));

        // Create pics archive if present
        let pics_archive = if let Some(boundary) = pics_boundary {
            let limited_reader = points_archive.into_inner();
            let reader = limited_reader.into_inner();
            let pics_reader = LimitedReader::new(reader, 0..boundary)?;
            Some(zip::ZipArchive::new(pics_reader)?)
        } else {
            None
        };

        let cupx_file = Self {
            cup_file,
            pics_archive,
        };

        Ok((cupx_file, warnings))
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

    /// Get reader for image by filename (without "pics/" prefix)
    /// Returns error if image doesn't exist
    /// Only one image can be read at a time (requires &mut self)
    pub fn read_picture(&mut self, filename: &str) -> Result<impl Read + '_, Error> {
        let pics_archive = self
            .pics_archive
            .as_mut()
            .ok_or(zip::result::ZipError::FileNotFound)?;

        // Try to find the file with case-insensitive prefix matching
        let target_filename = filename.to_lowercase();
        let actual_path = pics_archive
            .file_names()
            .find(|name| {
                name.len() >= 5
                    && name.is_char_boundary(5)
                    && name[..5].eq_ignore_ascii_case("pics/")
                    && name[5..].to_lowercase() == target_filename
            })
            .ok_or(zip::result::ZipError::FileNotFound)?
            .to_string();

        let file = pics_archive.by_name(&actual_path)?;
        Ok(file)
    }

    /// Iterator over all available image filenames (without "pics/" prefix)
    pub fn picture_names(&self) -> impl Iterator<Item = String> + '_ {
        self.pics_archive
            .as_ref()
            .into_iter()
            .flat_map(|archive| archive.file_names())
            .filter_map(|name| {
                // Handle case-insensitive "pics/" prefix
                if name.len() >= 5
                    && name.is_char_boundary(5)
                    && name[..5].eq_ignore_ascii_case("pics/")
                {
                    Some(name[5..].to_string())
                } else {
                    None
                }
            })
    }
}

#[derive(Debug, Clone)]
pub enum Warning {
    NoPicturesArchive,
    CupParseIssue { message: String, line: Option<u64> },
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

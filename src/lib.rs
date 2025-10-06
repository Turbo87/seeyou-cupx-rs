#![doc = include_str!("../README.md")]

use limited_reader::LimitedReader;
use seeyou_cup::{CupEncoding, CupFile, Task, Waypoint};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use std::ops::Range;
use std::path::{Path, PathBuf};

mod limited_reader;

/// A parsed CUPX file containing waypoint data and optional pictures.
///
/// CUPX files consist of two concatenated ZIP archives. The first contains pictures
/// in a `pics/` directory, and the second contains a `POINTS.CUP` file with waypoint
/// and task data.
///
/// The generic parameter `R` is the underlying reader type, which must implement
/// [`Read`] and [`Seek`].
///
/// # Examples
///
/// ```no_run
/// use seeyou_cupx::CupxFile;
///
/// let (cupx, warnings) = CupxFile::from_path("waypoints.cupx")?;
/// println!("Loaded {} waypoints", cupx.waypoints().len());
/// # Ok::<(), seeyou_cupx::Error>(())
/// ```
pub struct CupxFile<R> {
    cup_file: CupFile,
    pics_archive: Option<zip::ZipArchive<LimitedReader<R, Range<u64>>>>,
}

impl CupxFile<File> {
    /// Opens and parses a CUPX file from the given path.
    ///
    /// The text encoding of the CUP file is detected automatically.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use seeyou_cupx::CupxFile;
    ///
    /// let (cupx, warnings) = CupxFile::from_path("waypoints.cupx")?;
    /// println!("Loaded {} waypoints", cupx.waypoints().len());
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, is not a valid CUPX file,
    /// or contains invalid CUP data.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<(Self, Vec<Warning>), Error> {
        let file = File::open(path)?;
        Self::from_reader(file)
    }

    /// Opens and parses a CUPX file from the given path with a specific encoding.
    ///
    /// Use this when you know the encoding of the CUP file and want to avoid
    /// automatic detection.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, is not a valid CUPX file,
    /// or contains invalid CUP data.
    pub fn from_path_with_encoding<P: AsRef<Path>>(
        path: P,
        encoding: CupEncoding,
    ) -> Result<(Self, Vec<Warning>), Error> {
        let file = File::open(path)?;
        Self::from_reader_with_encoding(file, encoding)
    }
}

impl<R: Read + Seek> CupxFile<R> {
    /// Parses a CUPX file from a reader.
    ///
    /// The text encoding of the CUP file is detected automatically.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader does not contain a valid CUPX file or
    /// if the CUP data is invalid.
    pub fn from_reader(reader: R) -> Result<(Self, Vec<Warning>), Error> {
        Self::from_reader_inner(reader, None)
    }

    /// Parses a CUPX file from a reader with a specific encoding.
    ///
    /// Use this when you know the encoding of the CUP file and want to avoid
    /// automatic detection.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader does not contain a valid CUPX file or
    /// if the CUP data is invalid.
    pub fn from_reader_with_encoding(
        reader: R,
        encoding: CupEncoding,
    ) -> Result<(Self, Vec<Warning>), Error> {
        Self::from_reader_inner(reader, Some(encoding))
    }

    /// Parses a CUPX file by locating the two ZIP archives within it.
    ///
    /// CUPX files contain two concatenated ZIP archives. This method finds both by
    /// searching for End of Central Directory (EOCD) signatures. The EOCD of the first
    /// archive marks the boundary between the two archives. If only one EOCD is found,
    /// the file contains no pictures.
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
        warnings.extend(
            cup_warnings
                .into_iter()
                .map(|issue| Warning::CupParseIssue {
                    message: issue.message().to_string(),
                    line: issue.line(),
                }),
        );

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

    /// Returns a reference to the parsed CUP file data.
    ///
    /// The [`CupFile`] contains all waypoints and tasks from the CUPX file.
    pub fn cup_file(&self) -> &CupFile {
        &self.cup_file
    }

    /// Returns a slice of all waypoints in the file.
    pub fn waypoints(&self) -> &[Waypoint] {
        &self.cup_file().waypoints
    }

    /// Returns a slice of all tasks in the file.
    pub fn tasks(&self) -> &[Task] {
        &self.cup_file().tasks
    }

    /// Returns a reader for the picture with the given filename.
    ///
    /// The filename should not include the `pics/` prefix. Matching is case-insensitive.
    ///
    /// Only one picture can be read at a time, as this method requires `&mut self`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use seeyou_cupx::CupxFile;
    /// use std::io::Read;
    ///
    /// let (mut cupx, _) = CupxFile::from_path("waypoints.cupx")?;
    /// let mut reader = cupx.read_picture("airport.jpg")?;
    ///
    /// let mut buffer = Vec::new();
    /// reader.read_to_end(&mut buffer)?;
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the picture doesn't exist or if the CUPX file
    /// doesn't contain a pictures archive.
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

    /// Returns an iterator over all picture filenames in the CUPX file.
    ///
    /// Filenames do not include the `pics/` prefix. If the CUPX file doesn't
    /// contain a pictures archive, the iterator will be empty.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use seeyou_cupx::CupxFile;
    ///
    /// let (cupx, _) = CupxFile::from_path("waypoints.cupx")?;
    ///
    /// for name in cupx.picture_names() {
    ///     println!("Picture: {}", name);
    /// }
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
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

/// A builder for creating CUPX files with waypoint data and pictures.
///
/// `CupxWriter` allows you to construct a CUPX file by providing waypoint/task data
/// via a [`CupFile`] and optionally adding pictures that will be included in the
/// output file.
///
/// # Examples
///
/// ```no_run
/// use seeyou_cupx::CupxWriter;
/// use seeyou_cup::CupFile;
/// # use std::path::Path;
///
/// CupxWriter::new(CupFile::default())
///     .add_picture("photo.jpg", Path::new("images/photo.jpg"))
///     .write_to_path("output.cupx")?;
/// # Ok::<(), seeyou_cupx::Error>(())
/// ```
pub struct CupxWriter {
    cup_file: CupFile,
    pictures: HashMap<String, PictureSource>,
}

/// Source of picture data for inclusion in a CUPX file.
///
/// Pictures can be provided either as in-memory byte vectors or as file paths
/// that will be read when the CUPX file is written.
pub enum PictureSource {
    /// Picture data provided as a byte vector in memory.
    Bytes(Vec<u8>),
    /// Picture data will be read from a file at the given path.
    Path(PathBuf),
}

impl From<Vec<u8>> for PictureSource {
    fn from(bytes: Vec<u8>) -> Self {
        PictureSource::Bytes(bytes)
    }
}

impl From<PathBuf> for PictureSource {
    fn from(path: PathBuf) -> Self {
        PictureSource::Path(path)
    }
}

impl From<&Path> for PictureSource {
    fn from(path: &Path) -> Self {
        PictureSource::Path(path.to_path_buf())
    }
}

impl CupxWriter {
    /// Creates a new CUPX writer with the given waypoint/task data.
    ///
    /// Pictures can be added using [`add_picture`](Self::add_picture).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use seeyou_cupx::CupxWriter;
    /// use seeyou_cup::CupFile;
    ///
    /// let cup_file = CupFile::default();
    /// let writer = CupxWriter::new(cup_file);
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    pub fn new(cup_file: CupFile) -> Self {
        Self {
            cup_file,
            pictures: HashMap::new(),
        }
    }

    /// Adds a picture to the CUPX file.
    ///
    /// The `filename` is the name the picture will have in the archive (without
    /// the `pics/` prefix). The `source` can be either a file path or byte data.
    ///
    /// Returns a mutable reference to `self` for method chaining.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use seeyou_cupx::CupxWriter;
    /// use seeyou_cup::CupFile;
    /// # use std::path::Path;
    ///
    /// CupxWriter::new(CupFile::default())
    ///     .add_picture("photo1.jpg", Path::new("images/photo1.jpg"))
    ///     .add_picture("photo2.jpg", vec![0u8; 100])
    ///     .write_to_path("output.cupx")?;
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    pub fn add_picture(
        &mut self,
        filename: impl Into<String>,
        source: impl Into<PictureSource>,
    ) -> &mut Self {
        self.pictures.insert(filename.into(), source.into());
        self
    }

    /// Writes the CUPX file to the given writer.
    ///
    /// The writer must implement both [`Write`] and [`Seek`].
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Any picture filename is invalid (empty or contains path separators)
    /// - A picture file cannot be read
    /// - Writing to the output fails
    pub fn write<W: Write + Seek>(&self, writer: W) -> Result<(), Error> {
        for filename in self.pictures.keys() {
            if filename.is_empty() || filename.contains('/') || filename.contains('\\') {
                return Err(Error::InvalidFilename(filename.clone()));
            }
        }

        let options = zip::write::FileOptions::<()>::default()
            .compression_method(zip::CompressionMethod::Deflated);

        let mut pics_zip = zip::ZipWriter::new(writer);

        for (filename, source) in &self.pictures {
            let zip_filename = format!("pics/{}", filename);
            pics_zip.start_file(&zip_filename, options)?;

            match source {
                PictureSource::Bytes(data) => {
                    pics_zip.write_all(data)?;
                }
                PictureSource::Path(path) => {
                    let mut file = File::open(path)?;
                    std::io::copy(&mut file, &mut pics_zip)?;
                }
            }
        }

        let mut writer = pics_zip.finish()?;

        let mut points_buffer = Vec::new();
        let mut points_zip = zip::ZipWriter::new(Cursor::new(&mut points_buffer));
        points_zip.start_file("POINTS.CUP", options)?;
        self.cup_file.to_writer(&mut points_zip)?;
        points_zip.finish()?;
        writer.write_all(&points_buffer)?;

        Ok(())
    }

    /// Writes the CUPX file to a byte vector.
    ///
    /// This is a convenience method that creates an in-memory buffer and
    /// writes the CUPX file to it.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use seeyou_cupx::CupxWriter;
    /// use seeyou_cup::CupFile;
    ///
    /// let bytes = CupxWriter::new(CupFile::default()).write_to_vec()?;
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if any picture filename is invalid or if a picture
    /// file cannot be read.
    pub fn write_to_vec(&self) -> Result<Vec<u8>, Error> {
        let mut buffer = Vec::new();
        self.write(Cursor::new(&mut buffer))?;
        Ok(buffer)
    }

    /// Writes the CUPX file to the given path.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use seeyou_cupx::CupxWriter;
    /// use seeyou_cup::CupFile;
    ///
    /// CupxWriter::new(CupFile::default()).write_to_path("output.cupx")?;
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The file cannot be created
    /// - Any picture filename is invalid
    /// - A picture file cannot be read
    /// - Writing to the output fails
    pub fn write_to_path(&self, path: impl AsRef<Path>) -> Result<(), Error> {
        let file = File::create(path)?;
        self.write(file)
    }
}

/// Non-fatal warnings that may occur when parsing a CUPX file.
///
/// Warnings indicate issues that don't prevent the file from being read,
/// but may indicate missing data or parsing concerns.
#[derive(Debug, Clone)]
pub enum Warning {
    /// The CUPX file does not contain a pictures archive.
    NoPicturesArchive,
    /// An issue occurred while parsing the CUP file data.
    ///
    /// The `message` describes the issue, and `line` indicates the line number
    /// in the CUP file where it occurred, if available.
    CupParseIssue { message: String, line: Option<u64> },
}

/// Errors that can occur when reading or writing CUPX files.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// An I/O error occurred.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// An error occurred while reading or writing a ZIP archive.
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    /// An error occurred while parsing the CUP file data.
    #[error(transparent)]
    Cup(#[from] seeyou_cup::Error),
    /// The file is not a valid CUPX file.
    ///
    /// This typically means the required ZIP archive structure could not be found.
    #[error("Invalid CUPX file: could not find two ZIP archives")]
    InvalidCupx,
    /// A picture filename is invalid.
    ///
    /// Picture filenames must not be empty and must not contain path separators
    /// (`/` or `\`).
    #[error("Invalid picture filename: {0}")]
    InvalidFilename(String),
}

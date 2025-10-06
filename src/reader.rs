use crate::error::{Error, Warning};
use crate::limited_reader::LimitedReader;
use seeyou_cup::{CupFile, Encoding, Task, Waypoint};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::ops::Range;
use std::path::Path;

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
        encoding: Encoding,
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
        encoding: Encoding,
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
        encoding: Option<Encoding>,
    ) -> Result<(Self, Vec<Warning>), Error> {
        const EOCD_SIGNATURE: &[u8] = b"PK\x05\x06";
        const EOCD_MIN_SIZE: u64 = 22;
        const CHUNK_SIZE: u64 = 65536; // 64KB chunks for incremental search

        // Get file size
        reader.seek(SeekFrom::Start(0))?;
        let file_size = reader.seek(SeekFrom::End(0))?;

        // Find both EOCD signatures by searching backwards incrementally
        let mut last_eocd: Option<u64> = None;
        let mut second_last_eocd: Option<u64> = None;
        let mut search_end = file_size;

        // Search backwards in chunks until we find 2 EOCDs or reach the beginning
        while second_last_eocd.is_none() && search_end > 0 {
            let chunk_size = CHUNK_SIZE.min(search_end);
            let chunk_start = search_end - chunk_size;

            reader.seek(SeekFrom::Start(chunk_start))?;
            let mut chunk_buffer = vec![0u8; chunk_size as usize];
            reader.read_exact(&mut chunk_buffer)?;

            // Find the last two EOCDs in this chunk
            // Since we iterate forward, the last ones we see are the rightmost
            let mut chunk_last: Option<u64> = None;
            let mut chunk_second_last: Option<u64> = None;

            for offset in memchr::memmem::find_iter(&chunk_buffer, EOCD_SIGNATURE) {
                chunk_second_last = chunk_last;
                chunk_last = Some(chunk_start + offset as u64);
            }

            // Update global tracking: first chunk provides both, subsequent chunks provide second-to-last
            if last_eocd.is_none() {
                last_eocd = chunk_last;
                second_last_eocd = chunk_second_last;
            } else if second_last_eocd.is_none() && chunk_last.is_some() {
                second_last_eocd = chunk_last;
            }

            search_end = chunk_start;
        }

        let mut warnings = Vec::new();

        // Determine points archive range and whether pics exist
        let pics_boundary = if let Some(first_eocd_offset) = second_last_eocd {
            // Two ZIP archives found (normal case with pictures)
            // Calculate the boundary: first EOCD offset + EOCD record length
            // Read comment length from first EOCD to get full record size
            reader.seek(SeekFrom::Start(first_eocd_offset + 20))?;
            let mut comment_len_buf = [0u8; 2];
            reader.read_exact(&mut comment_len_buf)?;
            let comment_len = u16::from_le_bytes(comment_len_buf) as u64;

            let boundary = first_eocd_offset + EOCD_MIN_SIZE + comment_len;
            Some(boundary)
        } else if last_eocd.is_some() {
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

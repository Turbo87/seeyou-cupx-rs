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

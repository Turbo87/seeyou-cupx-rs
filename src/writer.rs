use crate::error::Error;
use seeyou_cup::CupFile;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Cursor, Seek, Write};
use std::path::Path;

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
/// # let cup_file = CupFile::default();
/// CupxWriter::new(&cup_file)
///     .add_picture("photo.jpg", Path::new("images/photo.jpg"))
///     .write_to_path("output.cupx")?;
/// # Ok::<(), seeyou_cupx::Error>(())
/// ```
pub struct CupxWriter<'a> {
    cup_file: &'a CupFile,
    pictures: HashMap<&'a str, PictureSource<'a>>,
}

/// Source of picture data for inclusion in a CUPX file.
///
/// Pictures can be provided either as in-memory byte slices or as file paths
/// that will be read when the CUPX file is written.
pub enum PictureSource<'a> {
    /// Picture data provided as a borrowed byte slice.
    Bytes(&'a [u8]),
    /// Picture data will be read from a file at the given path.
    Path(&'a Path),
}

impl<'a> From<&'a [u8]> for PictureSource<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        PictureSource::Bytes(bytes)
    }
}

impl<'a> From<&'a Path> for PictureSource<'a> {
    fn from(path: &'a Path) -> Self {
        PictureSource::Path(path)
    }
}

impl<'a> CupxWriter<'a> {
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
    /// let writer = CupxWriter::new(&cup_file);
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    pub fn new(cup_file: &'a CupFile) -> Self {
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
    /// # let cup_file = CupFile::default();
    /// # let image_data = vec![0u8; 100];
    /// CupxWriter::new(&cup_file)
    ///     .add_picture("photo1.jpg", Path::new("images/photo1.jpg"))
    ///     .add_picture("photo2.jpg", &image_data[..])
    ///     .write_to_path("output.cupx")?;
    /// # Ok::<(), seeyou_cupx::Error>(())
    /// ```
    pub fn add_picture(
        &mut self,
        filename: &'a str,
        source: impl Into<PictureSource<'a>>,
    ) -> &mut Self {
        self.pictures.insert(filename, source.into());
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
                return Err(Error::InvalidFilename(filename.to_string()));
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
    /// let cup_file = CupFile::default();
    /// let bytes = CupxWriter::new(&cup_file).write_to_vec()?;
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
    /// let cup_file = CupFile::default();
    /// CupxWriter::new(&cup_file).write_to_path("output.cupx")?;
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

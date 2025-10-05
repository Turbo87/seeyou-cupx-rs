use insta::{assert_binary_snapshot, assert_compact_debug_snapshot};
use seeyou_cup::CupFile;
use seeyou_cupx::{CupxFile, CupxWriter};
use std::io::{Cursor, Read};
use std::path::PathBuf;

#[test]
fn test_write_empty() {
    let buffer = CupxWriter::new(CupFile::default()).write_to_vec().unwrap();

    let (result, _) = CupxFile::from_reader(Cursor::new(&buffer)).unwrap();
    assert_eq!(result.waypoints().len(), 0);
    assert_eq!(result.tasks().len(), 0);
    assert_eq!(result.picture_names().count(), 0);
}

#[test]
fn test_write_with_bytes_picture() {
    let picture_data = b"fake image data".to_vec();

    let buffer = CupxWriter::new(CupFile::default())
        .add_picture("test.jpg", picture_data.clone())
        .write_to_vec()
        .unwrap();

    let (mut result, _) = CupxFile::from_reader(Cursor::new(&buffer)).unwrap();
    let names: Vec<_> = result.picture_names().collect();
    assert_eq!(names, vec!["test.jpg"]);

    let mut read_data = Vec::new();
    result
        .read_picture("test.jpg")
        .unwrap()
        .read_to_end(&mut read_data)
        .unwrap();
    assert_eq!(read_data, picture_data);
}

#[test]
fn test_write_with_path_picture() {
    let buffer = CupxWriter::new(CupFile::default())
        .add_picture("2_1034.jpg", PathBuf::from("tests/fixtures/2_1034.jpg"))
        .write_to_vec()
        .unwrap();

    let (mut result, _) = CupxFile::from_reader(Cursor::new(&buffer)).unwrap();
    let names: Vec<_> = result.picture_names().collect();
    assert_eq!(names, vec!["2_1034.jpg"]);

    let mut read_data = Vec::new();
    result
        .read_picture("2_1034.jpg")
        .unwrap()
        .read_to_end(&mut read_data)
        .unwrap();
    assert_binary_snapshot!("2_1034.jpg", read_data);
}

#[test]
fn test_write_duplicate_filename_replaces() {
    let first_data = b"first".to_vec();
    let second_data = b"second".to_vec();

    let buffer = CupxWriter::new(CupFile::default())
        .add_picture("test.jpg", first_data)
        .add_picture("test.jpg", second_data.clone())
        .write_to_vec()
        .unwrap();

    let (mut result, _) = CupxFile::from_reader(Cursor::new(&buffer)).unwrap();
    let names: Vec<_> = result.picture_names().collect();
    assert_eq!(names, vec!["test.jpg"]);

    let mut read_data = Vec::new();
    result
        .read_picture("test.jpg")
        .unwrap()
        .read_to_end(&mut read_data)
        .unwrap();
    assert_eq!(read_data, second_data);
}

#[test]
fn test_write_multiple_pictures() {
    let buffer = CupxWriter::new(CupFile::default())
        .add_picture("a.jpg", b"data a".to_vec())
        .add_picture("b.jpg", b"data b".to_vec())
        .add_picture("c.jpg", b"data c".to_vec())
        .write_to_vec()
        .unwrap();

    let (result, _) = CupxFile::from_reader(Cursor::new(&buffer)).unwrap();
    let mut names: Vec<_> = result.picture_names().collect();
    names.sort();
    assert_eq!(names, vec!["a.jpg", "b.jpg", "c.jpg"]);
}

#[test]
fn test_write_invalid_filename_empty() {
    let result = CupxWriter::new(CupFile::default())
        .add_picture("", b"data".to_vec())
        .write_to_vec();

    assert_compact_debug_snapshot!(result, @r#"Err(InvalidFilename(""))"#);
}

#[test]
fn test_write_invalid_filename_with_slash() {
    let result = CupxWriter::new(CupFile::default())
        .add_picture("path/to/file.jpg", b"data".to_vec())
        .write_to_vec();

    assert_compact_debug_snapshot!(result, @r#"Err(InvalidFilename("path/to/file.jpg"))"#);
}

#[test]
fn test_write_invalid_filename_with_backslash() {
    let result = CupxWriter::new(CupFile::default())
        .add_picture("path\\to\\file.jpg", b"data".to_vec())
        .write_to_vec();

    assert_compact_debug_snapshot!(result, @r#"Err(InvalidFilename("path\\to\\file.jpg"))"#);
}

#[test]
fn test_write_nonexistent_path() {
    let result = CupxWriter::new(CupFile::default())
        .add_picture("test.jpg", PathBuf::from("nonexistent/file.jpg"))
        .write_to_vec();

    assert_compact_debug_snapshot!(result, @r#"Err(Io(Os { code: 2, kind: NotFound, message: "No such file or directory" }))"#);
}

#[test]
fn test_write_to_path() {
    let temp_path = std::env::temp_dir().join("test_cupx_writer.cupx");

    CupxWriter::new(CupFile::default())
        .add_picture("test.jpg", b"test data".to_vec())
        .write_to_path(&temp_path)
        .unwrap();

    let (result, _) = CupxFile::from_path(&temp_path).unwrap();
    assert_eq!(result.waypoints().len(), 0);
    let names: Vec<_> = result.picture_names().collect();
    assert_eq!(names, vec!["test.jpg"]);

    std::fs::remove_file(&temp_path).unwrap();
}

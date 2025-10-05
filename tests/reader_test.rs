use insta::assert_compact_debug_snapshot;
use seeyou_cupx::CupxFile;
use std::io::Read;

#[test]
fn test_westalpen() {
    let (mut cupx, warnings) = CupxFile::from_path("tests/fixtures/westalpen_de.cupx").unwrap();
    assert_eq!(cupx.waypoints().len(), 126);
    assert_eq!(cupx.tasks().len(), 0);
    assert_eq!(warnings.len(), 0);

    let mut image_files = cupx.picture_names().collect::<Vec<_>>();
    image_files.sort();
    insta::assert_debug_snapshot!(image_files);

    let mut reader = cupx.read_picture("2_1034.jpg").unwrap();
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer).unwrap();
    insta::assert_binary_snapshot!("2_1034.jpg", buffer);
}

#[test]
fn test_ec25_no_pictures_zip() {
    let (cupx, warnings) = CupxFile::from_path("tests/fixtures/EC25_no_pictures_zip.cupx").unwrap();
    assert_eq!(cupx.waypoints().len(), 221);
    assert_eq!(cupx.tasks().len(), 0);
    assert_compact_debug_snapshot!(warnings, @"[NoPicturesArchive]");
    assert_eq!(cupx.picture_names().count(), 0);
}

use seeyou_cupx::CupxFile;

#[test]
fn test_westalpen() {
    let (cupx, warnings) = CupxFile::from_path("tests/fixtures/westalpen_de.cupx").unwrap();
    assert_eq!(cupx.waypoints().len(), 126);
    assert_eq!(cupx.tasks().len(), 0);
    assert_eq!(warnings.len(), 0);
}

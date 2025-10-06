use seeyou_cupx::CupxFile;
use std::io::{Cursor, Write};
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

#[test]
fn test_three_zip_archives() {
    // Create the first ZIP (extra.zip) - should be ignored
    let mut extra_zip = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut extra_zip));
        zip.start_file("extra/data.txt", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"extra data").unwrap();
        zip.finish().unwrap();
    }

    // Create the second ZIP (pics.zip)
    let mut pics_zip = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut pics_zip));
        zip.start_file("pics/test.jpg", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"fake image data").unwrap();
        zip.finish().unwrap();
    }

    // Create the third ZIP (points.zip) with POINTS.CUP
    let mut points_zip = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut points_zip));
        zip.start_file("POINTS.CUP", SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"name,code,country,lat,lon,elev,style,rwdir,rwlen,freq,desc\n")
            .unwrap();
        zip.finish().unwrap();
    }

    // Concatenate: extra.zip + pics.zip + points.zip
    let mut cupx_data = Vec::new();
    cupx_data.extend_from_slice(&extra_zip);
    cupx_data.extend_from_slice(&pics_zip);
    cupx_data.extend_from_slice(&points_zip);

    // Try to parse the three-ZIP CUPX file
    let (cupx, warnings) = CupxFile::from_reader(Cursor::new(&cupx_data)).unwrap();

    // Current behavior: successfully parses using the last two ZIPs,
    // silently ignoring the first ZIP without any warning
    assert_eq!(warnings.len(), 0);
    assert_eq!(cupx.waypoints().len(), 0);

    // Successfully reads picture from the second ZIP (pics.zip)
    let pictures: Vec<_> = cupx.picture_names().collect();
    assert_eq!(pictures, vec!["test.jpg"]);

    // The first ZIP (extra.zip) is completely ignored without warning
}

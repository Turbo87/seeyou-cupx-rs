use seeyou_cupx::cup::{CupFile, Elevation, Waypoint, WaypointStyle};
use seeyou_cupx::{CupxFile, CupxWriter};
use std::io::Cursor;

/// Test that CUPX files with large CUP data (>65KB compressed) can be parsed correctly.
///
/// This tests the case where the first ZIP archive's EOCD falls outside the initial
/// 65KB search buffer used in `from_reader_inner()`.
#[test]
fn test_large_cup_file() {
    // Create a CUP file with many waypoints containing varied text to avoid good compression
    let mut cup_file = CupFile::default();

    // Generate enough waypoints to create a compressed size > 65KB
    // We need varied descriptions to prevent good compression ratios
    for i in 0..2000 {
        cup_file.waypoints.push(create_varied_waypoint(i));
    }

    // Write the CUPX file with a small picture to ensure we have two ZIP archives
    let cupx_buffer = CupxWriter::new(cup_file)
        .add_picture("test.jpg", b"small test image data".to_vec())
        .write_to_vec()
        .unwrap();

    // Verify the points archive is large enough to push the pics EOCD outside the buffer
    // The search starts from the end, so if points archive is > 65KB, pics EOCD will be missed
    assert!(
        cupx_buffer.len() > 65557,
        "Test data not large enough to trigger bug"
    );

    // This should succeed even though the first EOCD is outside the initial search buffer
    let (cupx, warnings) = CupxFile::from_reader(Cursor::new(&cupx_buffer)).unwrap();

    // Verify we got the correct data
    assert_eq!(warnings.len(), 0);
    assert_eq!(cupx.waypoints().len(), 2000);
    assert_eq!(cupx.picture_names().count(), 1);
    assert_eq!(warnings.len(), 0);
}

/// Create a waypoint with varied data that doesn't compress well
fn create_varied_waypoint(index: usize) -> Waypoint {
    // Keep coordinates in valid ranges: latitude [-90, 90], longitude [-180, 180]
    let lat = -89.0 + ((index % 178) as f64) + (index as f64 * 0.001).fract();
    let lon = -179.0 + ((index % 358) as f64) + (index as f64 * 0.002).fract();

    // Generate varied description text using multiple patterns
    let description = format!(
        "Waypoint number {:05} located at coordinates with unique identifier {}. \
         Additional information: {:08x} {:08x} {:08x} {:08x}. \
         Notes: This is a test waypoint with index {} for testing large file parsing. \
         The description contains varied text to prevent good compression ratios. \
         Random-looking data: {} {} {} {}",
        index,
        index * 12345,
        index.wrapping_mul(2654435761),
        index.wrapping_mul(2654435761).wrapping_add(1),
        index.wrapping_mul(2654435761).wrapping_add(2),
        index.wrapping_mul(2654435761).wrapping_add(3),
        index,
        index.wrapping_mul(48271) % 100000,
        index.wrapping_mul(69621) % 100000,
        index.wrapping_mul(40014) % 100000,
        index.wrapping_mul(40692) % 100000,
    );

    Waypoint {
        name: format!("Waypoint{:05}", index),
        code: format!("WP{:04}", index % 10000),
        country: format!(
            "{}{}",
            (b'A' + (index % 26) as u8) as char,
            (b'A' + ((index / 26) % 26) as u8) as char,
        ),
        latitude: lat,
        longitude: lon,
        elevation: Elevation::Meters((index % 3000) as f64),
        style: WaypointStyle::Waypoint,
        runway_direction: None,
        runway_length: None,
        runway_width: None,
        frequency: String::new(),
        description,
        userdata: String::new(),
        pictures: vec![],
    }
}

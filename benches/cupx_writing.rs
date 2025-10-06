use criterion::{Criterion, criterion_group, criterion_main};
use seeyou_cup::CupFile;
use seeyou_cupx::CupxWriter;
use std::io::Cursor;
use std::path::Path;

fn bench_write_empty(c: &mut Criterion) {
    c.bench_function("CupxWriter::write (empty)", |b| {
        let mut buffer = Vec::with_capacity(10_000);
        b.iter(|| {
            buffer.clear();
            let cup_file = CupFile::default();
            let writer = CupxWriter::new(&cup_file);
            writer.write(Cursor::new(&mut buffer)).unwrap();
        })
    });
}

fn bench_write_with_single_picture(c: &mut Criterion) {
    let picture_data = vec![0u8; 34858]; // Same size as 2_1034.jpg

    c.bench_function("CupxWriter::write (1 picture)", |b| {
        let mut buffer = Vec::with_capacity(50_000);
        b.iter(|| {
            buffer.clear();
            let cup_file = CupFile::default();
            let mut writer = CupxWriter::new(&cup_file);
            writer.add_picture("test.jpg", picture_data.as_slice());
            writer.write(Cursor::new(&mut buffer)).unwrap();
        })
    });
}

fn bench_write_with_picture_from_path(c: &mut Criterion) {
    c.bench_function("CupxWriter::write (picture from path)", |b| {
        let mut buffer = Vec::with_capacity(50_000);
        b.iter(|| {
            buffer.clear();
            let cup_file = CupFile::default();
            let mut writer = CupxWriter::new(&cup_file);
            writer.add_picture("2_1034.jpg", Path::new("tests/fixtures/2_1034.jpg"));
            writer.write(Cursor::new(&mut buffer)).unwrap();
        })
    });
}

fn bench_write_with_multiple_pictures(c: &mut Criterion) {
    let picture_data_small = vec![0u8; 10000];
    let picture_data_medium = vec![0u8; 34858];
    let picture_data_large = vec![0u8; 100000];

    c.bench_function("CupxWriter::write (3 pictures)", |b| {
        let mut buffer = Vec::with_capacity(200_000);
        b.iter(|| {
            buffer.clear();
            let cup_file = CupFile::default();
            let mut writer = CupxWriter::new(&cup_file);
            writer.add_picture("small.jpg", picture_data_small.as_slice());
            writer.add_picture("medium.jpg", picture_data_medium.as_slice());
            writer.add_picture("large.jpg", picture_data_large.as_slice());
            writer.write(Cursor::new(&mut buffer)).unwrap();
        })
    });
}

criterion_group!(
    benches,
    bench_write_empty,
    bench_write_with_single_picture,
    bench_write_with_picture_from_path,
    bench_write_with_multiple_pictures
);
criterion_main!(benches);

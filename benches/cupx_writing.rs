use criterion::{criterion_group, criterion_main, Criterion};
use seeyou_cup::CupFile;
use seeyou_cupx::CupxWriter;
use std::hint::black_box;
use std::io::Cursor;
use std::path::PathBuf;

fn bench_write_empty(c: &mut Criterion) {
    c.bench_function("CupxWriter::write (empty)", |b| {
        let mut buffer = Vec::with_capacity(10_000);
        b.iter(|| {
            buffer.clear();
            let writer = CupxWriter::new(CupFile::default());
            black_box(writer.write(Cursor::new(&mut buffer)).unwrap());
            black_box(buffer.len())
        })
    });
}

fn bench_write_with_single_picture(c: &mut Criterion) {
    let picture_data = vec![0u8; 34858]; // Same size as 2_1034.jpg

    c.bench_function("CupxWriter::write (1 picture)", |b| {
        let mut buffer = Vec::with_capacity(50_000);
        b.iter(|| {
            buffer.clear();
            let mut writer = CupxWriter::new(CupFile::default());
            writer.add_picture("test.jpg", picture_data.clone());
            black_box(writer.write(Cursor::new(&mut buffer)).unwrap());
            black_box(buffer.len())
        })
    });
}

fn bench_write_with_picture_from_path(c: &mut Criterion) {
    c.bench_function("CupxWriter::write (picture from path)", |b| {
        let mut buffer = Vec::with_capacity(50_000);
        b.iter(|| {
            buffer.clear();
            let mut writer = CupxWriter::new(CupFile::default());
            writer.add_picture("2_1034.jpg", PathBuf::from("tests/fixtures/2_1034.jpg"));
            black_box(writer.write(Cursor::new(&mut buffer)).unwrap());
            black_box(buffer.len())
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
            let mut writer = CupxWriter::new(CupFile::default());
            writer.add_picture("small.jpg", picture_data_small.clone());
            writer.add_picture("medium.jpg", picture_data_medium.clone());
            writer.add_picture("large.jpg", picture_data_large.clone());
            black_box(writer.write(Cursor::new(&mut buffer)).unwrap());
            black_box(buffer.len())
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

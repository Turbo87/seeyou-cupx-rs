use criterion::{criterion_group, criterion_main, Criterion};
use seeyou_cupx::CupxFile;

fn bench_from_path(c: &mut Criterion) {
    c.bench_function("CupxFile::from_path", |b| {
        b.iter(|| CupxFile::from_path("tests/fixtures/westalpen_de.cupx").unwrap())
    });
}

criterion_group!(benches, bench_from_path);
criterion_main!(benches);

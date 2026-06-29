use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use image::{ImageBuffer, Rgb, RgbImage};
use lightframe_dedup::{compute_phash, dhash, hamming_distance};

fn gradient_image() -> RgbImage {
    ImageBuffer::from_fn(256, 256, |x, y| {
        let v = ((x + y) % 256) as u8;
        Rgb([v, v.saturating_add(10), v.saturating_sub(10)])
    })
}

fn bench_dhash(c: &mut Criterion) {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("bench.png");
    gradient_image().save(&path).expect("save bench image");

    c.bench_function("compute_dhash", |b| {
        b.iter(|| black_box(dhash(&path).expect("dhash")));
    });
}

fn bench_phash(c: &mut Criterion) {
    let img = image::DynamicImage::ImageRgb8(gradient_image());

    c.bench_function("compute_phash", |b| {
        b.iter(|| black_box(compute_phash(&img)));
    });
}

fn bench_hamming_distance(c: &mut Criterion) {
    let hashes: Vec<u64> = (0..10_000u64)
        .map(|i| i.wrapping_mul(0x9E37_79B9))
        .collect();
    let target = hashes[5000];

    let mut group = c.benchmark_group("hamming_distance_at_scale");
    group.bench_with_input(
        BenchmarkId::new("compare_10k", 10_000),
        &hashes,
        |b, hashes| {
            b.iter(|| {
                let mut total = 0u32;
                for &hash in hashes.iter() {
                    total = total.wrapping_add(hamming_distance(target, hash));
                }
                black_box(total)
            });
        },
    );
    group.finish();
}

criterion_group!(benches, bench_dhash, bench_phash, bench_hamming_distance);
criterion_main!(benches);

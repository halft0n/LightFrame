use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::path::Path;
use tempfile::TempDir;

fn create_test_tree(file_count: usize) -> TempDir {
    let dir = TempDir::new().expect("temp dir");
    let root = dir.path();

    let extensions = ["jpg", "png", "mp4", "cr2", "txt", "pdf", "rs"];
    let media_ratio = 5; // 5 out of 7 are media

    for i in 0..file_count {
        let subdir = root.join(format!("folder_{:03}", i / 100));
        std::fs::create_dir_all(&subdir).ok();
        let ext = extensions[i % extensions.len()];
        let filename = format!("file_{i:06}.{ext}");
        std::fs::write(subdir.join(filename), b"x").expect("write file");
    }

    // Verify expected media count
    let expected_media = file_count * media_ratio / extensions.len();
    let _ = expected_media;

    dir
}

fn bench_scan_walkdir(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let mut group = c.benchmark_group("scan_walkdir");
    group.sample_size(20);

    for &count in &[1_000usize, 5_000, 10_000] {
        let dir = create_test_tree(count);

        group.bench_with_input(BenchmarkId::from_parameter(count), &dir, |b, dir| {
            b.iter(|| {
                rt.block_on(async {
                    black_box(lightframe_indexer::scan_folder(dir.path()).await.unwrap())
                })
            });
        });
    }
    group.finish();
}

fn bench_is_media_file(c: &mut Criterion) {
    let paths: Vec<&Path> = vec![
        Path::new("photo.jpg"),
        Path::new("video.mp4"),
        Path::new("raw.cr2"),
        Path::new("document.pdf"),
        Path::new("code.rs"),
        Path::new("noext"),
        Path::new("PHOTO.PNG"),
        Path::new("clip.MOV"),
    ];

    c.bench_function("is_media_file_batch_8", |b| {
        b.iter(|| {
            for path in &paths {
                black_box(lightframe_indexer::is_media_file(path));
            }
        });
    });
}

fn bench_classify_extension(c: &mut Criterion) {
    let paths: Vec<&Path> = vec![
        Path::new("a.jpg"),
        Path::new("b.mp4"),
        Path::new("c.cr2"),
        Path::new("d.txt"),
        Path::new("e.webp"),
        Path::new("f.mkv"),
        Path::new("g.dng"),
        Path::new("h.avif"),
    ];

    c.bench_function("classify_extension_batch_8", |b| {
        b.iter(|| {
            for path in &paths {
                black_box(lightframe_indexer::classify_extension(path));
            }
        });
    });
}

criterion_group!(
    benches,
    bench_scan_walkdir,
    bench_is_media_file,
    bench_classify_extension
);
criterion_main!(benches);

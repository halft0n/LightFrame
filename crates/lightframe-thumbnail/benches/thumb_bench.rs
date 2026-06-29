use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use image::{ImageBuffer, ImageFormat, Rgb, RgbImage};
use lightframe_core::media::{DecodedImage, ThumbnailSize};

fn source_image() -> RgbImage {
    ImageBuffer::from_fn(4032, 3024, |x, y| {
        let v = ((x + y) % 256) as u8;
        Rgb([v, v.saturating_add(10), v.saturating_sub(10)])
    })
}

fn decoded_from_rgb(img: &RgbImage) -> DecodedImage {
    let dynamic = image::DynamicImage::ImageRgb8(img.clone());
    let rgba = dynamic.to_rgba8();
    let (width, height) = rgba.dimensions();
    DecodedImage {
        rgba: rgba.into_raw(),
        width,
        height,
    }
}

fn resize_thumbnail(decoded: &DecodedImage, size: ThumbnailSize) -> image::DynamicImage {
    let img = decoded.to_dynamic_image();
    let pixels = size.pixels();
    img.thumbnail(pixels, pixels)
}

fn encode_webp(thumb: &image::DynamicImage) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, ImageFormat::WebP)
        .expect("webp encode");
    buf.into_inner()
}

fn encode_jpeg(thumb: &image::DynamicImage) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    thumb
        .write_to(&mut buf, ImageFormat::Jpeg)
        .expect("jpeg encode");
    buf.into_inner()
}

fn bench_thumbnail_sizes(c: &mut Criterion) {
    let img = source_image();
    let decoded = decoded_from_rgb(&img);

    let mut group = c.benchmark_group("thumbnail_resize_from_decoded");
    for size in [
        ThumbnailSize::Micro,
        ThumbnailSize::Small,
        ThumbnailSize::Large,
    ] {
        let label = match size {
            ThumbnailSize::Micro => "micro_64",
            ThumbnailSize::Small => "small_256",
            ThumbnailSize::Large => "large_1024",
        };
        group.bench_with_input(BenchmarkId::from_parameter(label), &size, |b, &size| {
            b.iter(|| black_box(resize_thumbnail(&decoded, size)));
        });
    }
    group.finish();
}

fn bench_encode_formats(c: &mut Criterion) {
    let img = source_image();
    let decoded = decoded_from_rgb(&img);
    let thumb = resize_thumbnail(&decoded, ThumbnailSize::Small);

    let mut group = c.benchmark_group("thumbnail_encode_small");
    group.bench_function("webp", |b| {
        b.iter(|| black_box(encode_webp(&thumb)));
    });
    group.bench_function("jpeg", |b| {
        b.iter(|| black_box(encode_jpeg(&thumb)));
    });
    group.finish();
}

fn bench_full_pipeline_no_io(c: &mut Criterion) {
    let img = source_image();
    let decoded = decoded_from_rgb(&img);

    let mut group = c.benchmark_group("decode_to_webp_no_disk");
    for size in [
        ThumbnailSize::Micro,
        ThumbnailSize::Small,
        ThumbnailSize::Large,
    ] {
        let label = match size {
            ThumbnailSize::Micro => "micro",
            ThumbnailSize::Small => "small",
            ThumbnailSize::Large => "large",
        };
        group.bench_with_input(BenchmarkId::from_parameter(label), &size, |b, &size| {
            b.iter(|| {
                let thumb = resize_thumbnail(&decoded, size);
                black_box(encode_webp(&thumb))
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_thumbnail_sizes,
    bench_encode_formats,
    bench_full_pipeline_no_io
);
criterion_main!(benches);

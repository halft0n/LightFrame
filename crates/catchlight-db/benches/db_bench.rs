use catchlight_core::media::{MediaFile, MediaType};
use catchlight_db::Database;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use std::path::Path;

fn open_db() -> Database {
    Database::open(Path::new(":memory:")).expect("open db")
}

fn add_folder(db: &Database) -> i64 {
    db.add_watched_folder("/bench/photos")
        .expect("add folder")
        .id
}

fn make_media(
    path: &str,
    filename: &str,
    dhash: Option<u64>,
    created_at: Option<NaiveDateTime>,
) -> MediaFile {
    MediaFile {
        id: 0,
        path: path.to_string(),
        filename: filename.to_string(),
        media_type: MediaType::Photo,
        size_bytes: 2048,
        width: Some(1920),
        height: Some(1080),
        created_at,
        modified_at: created_at.unwrap_or_default(),
        blake3_hash: Some(format!("hash_{filename}")),
        dhash,
        phash: dhash,
        latitude: None,
        longitude: None,
    }
}

fn seed_media(db: &Database, folder_id: i64, count: usize) {
    for i in 0..count {
        let day = (i % 28) as u32 + 1;
        let month = ((i / 28) % 12) as u32 + 1;
        let year = 2020 + (i / (28 * 12)) as i32;
        let date = NaiveDate::from_ymd_opt(year, month, day)
            .unwrap_or_else(|| NaiveDate::from_ymd_opt(2024, 1, 1).unwrap());
        let created_at = date.and_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap());
        let path = format!("/bench/photos/photo_{i:06}.jpg");
        let filename = format!("vacation_sunset_{i}.jpg");
        let dhash = Some(i as u64 % 64);
        let media = make_media(&path, &filename, dhash, Some(created_at));
        db.upsert_media(folder_id, &media).expect("upsert media");
    }
}

fn bench_upsert_media(c: &mut Criterion) {
    c.bench_function("upsert_media_1000", |b| {
        b.iter(|| {
            let db = open_db();
            let folder_id = add_folder(&db);
            for i in 0..1000 {
                let path = format!("/bench/photos/photo_{i:04}.jpg");
                let filename = format!("photo_{i:04}.jpg");
                let media = make_media(&path, &filename, Some(i as u64), None);
                black_box(db.upsert_media(folder_id, &media).unwrap());
            }
        });
    });
}

fn bench_search_fts(c: &mut Criterion) {
    let db = open_db();
    let folder_id = add_folder(&db);
    seed_media(&db, folder_id, 10_000);

    c.bench_function("search_media_fts_10k", |b| {
        b.iter(|| black_box(db.search_media("vacation", 50, 0).unwrap()));
    });
}

fn bench_dedup_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_perceptual_duplicates");
    for size in [100usize, 500, 1000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let db = open_db();
            let folder_id = add_folder(&db);
            seed_media(&db, folder_id, size);
            b.iter(|| black_box(db.find_perceptual_duplicates(10).unwrap()));
        });
    }
    group.finish();
}

fn bench_pagination(c: &mut Criterion) {
    let db = open_db();
    let folder_id = add_folder(&db);
    seed_media(&db, folder_id, 10_000);

    let mut group = c.benchmark_group("get_media_page");
    group.bench_function("keyset_page_50", |b| {
        b.iter(|| black_box(db.get_media_page(50, None).unwrap()));
    });
    group.bench_function("offset_page_50_at_5000", |b| {
        b.iter(|| black_box(db.get_all_media(50, 5000).unwrap()));
    });
    group.finish();
}

fn bench_timeline_groups(c: &mut Criterion) {
    let db = open_db();
    let folder_id = add_folder(&db);
    seed_media(&db, folder_id, 5_000);

    c.bench_function("get_timeline_groups_5000", |b| {
        b.iter(|| black_box(db.get_timeline_groups(200, 0).unwrap()));
    });
}

criterion_group!(
    benches,
    bench_upsert_media,
    bench_search_fts,
    bench_dedup_scan,
    bench_pagination,
    bench_timeline_groups
);
criterion_main!(benches);

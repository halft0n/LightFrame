use lightframe_dedup::{hamming_distance, is_perceptually_similar};
use std::io::Write;

#[test]
fn hamming_distance_identical() {
    assert_eq!(hamming_distance(0, 0), 0);
    assert_eq!(hamming_distance(u64::MAX, u64::MAX), 0);
    assert_eq!(hamming_distance(0xDEADBEEF, 0xDEADBEEF), 0);
}

#[test]
fn hamming_distance_one_bit() {
    assert_eq!(hamming_distance(0b0000, 0b0001), 1);
    assert_eq!(hamming_distance(0b1000, 0b0000), 1);
}

#[test]
fn hamming_distance_all_different() {
    assert_eq!(hamming_distance(0u64, u64::MAX), 64);
}

#[test]
fn hamming_distance_symmetric() {
    let a = 0x12345678u64;
    let b = 0x87654321u64;
    assert_eq!(hamming_distance(a, b), hamming_distance(b, a));
}

#[test]
fn perceptual_similarity_identical() {
    assert!(is_perceptually_similar(0xABCD, 0xABCD, 0));
}

#[test]
fn perceptual_similarity_within_threshold() {
    assert!(is_perceptually_similar(0b0000, 0b0011, 5));
    assert!(is_perceptually_similar(0b0000, 0b0011, 2));
}

#[test]
fn perceptual_similarity_beyond_threshold() {
    assert!(!is_perceptually_similar(0b0000, 0b1111, 3));
}

#[test]
fn blake3_hash_deterministic() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.bin");
    {
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"hello lightframe").unwrap();
    }

    let h1 = lightframe_dedup::file_hash(&path).unwrap();
    let h2 = lightframe_dedup::file_hash(&path).unwrap();
    assert_eq!(h1, h2);
    assert!(!h1.is_empty());
    assert_eq!(h1.len(), 64, "BLAKE3 hex hash should be 64 chars");
}

#[test]
fn blake3_hash_different_content() {
    let dir = tempfile::tempdir().unwrap();

    let p1 = dir.path().join("a.bin");
    let p2 = dir.path().join("b.bin");
    std::fs::write(&p1, b"content A").unwrap();
    std::fs::write(&p2, b"content B").unwrap();

    let h1 = lightframe_dedup::file_hash(&p1).unwrap();
    let h2 = lightframe_dedup::file_hash(&p2).unwrap();
    assert_ne!(h1, h2);
}

#[test]
fn blake3_hash_nonexistent_file() {
    let result = lightframe_dedup::file_hash(std::path::Path::new("/nonexistent/file.bin"));
    assert!(result.is_err());
}

#[test]
fn hamming_distance_all_zero_hashes() {
    assert_eq!(hamming_distance(0, 0), 0);
    assert_eq!(hamming_distance(0, 1), 1);
    assert_eq!(hamming_distance(0, 0xFFFF), 16);
}

#[test]
fn hamming_distance_all_ones_hashes() {
    let ones = u64::MAX;
    assert_eq!(hamming_distance(ones, ones), 0);
    assert_eq!(hamming_distance(ones, 0), 64);
    assert_eq!(hamming_distance(ones, ones >> 1), 1);
}

#[test]
fn perceptual_similarity_at_max_distance_boundary() {
    assert!(is_perceptually_similar(0, u64::MAX, 64));
    assert!(!is_perceptually_similar(0, u64::MAX, 63));
}

#[test]
fn single_hash_self_comparison_is_identical() {
    let hash = 0x1234_5678_9ABC_DEF0u64;
    assert_eq!(hamming_distance(hash, hash), 0);
    assert!(is_perceptually_similar(hash, hash, 0));
    assert!(is_perceptually_similar(hash, hash, 64));
}

#[test]
fn single_element_hamming_is_zero() {
    let only = 0xAAAA_BBBB_CCCC_DDDDu64;
    assert_eq!(hamming_distance(only, only), 0);
}

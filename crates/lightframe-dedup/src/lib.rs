pub mod exact;
pub mod perceptual;

use lightframe_core::Result;
use std::path::Path;

pub fn file_hash(path: &Path) -> Result<String> {
    exact::blake3_hash(path)
}

pub fn dhash(path: &Path) -> Result<u64> {
    perceptual::compute_dhash(path)
}

pub use perceptual::compute_phash;
pub use perceptual::dhash_from_decoded;
pub use perceptual::phash_from_decoded;

pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

pub fn is_perceptually_similar(a: u64, b: u64, threshold: u32) -> bool {
    hamming_distance(a, b) <= threshold
}

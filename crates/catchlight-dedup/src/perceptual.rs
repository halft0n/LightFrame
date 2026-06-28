use catchlight_core::Result;
use image::imageops::FilterType;
use std::path::Path;

pub fn compute_dhash(path: &Path) -> Result<u64> {
    let img = image::open(path)
        .map_err(|e| catchlight_core::Error::Other(e.to_string()))?;

    let gray = img.resize_exact(9, 8, FilterType::Lanczos3).to_luma8();
    let mut hash: u64 = 0;

    for y in 0..8 {
        for x in 0..8 {
            let left = gray.get_pixel(x, y)[0];
            let right = gray.get_pixel(x + 1, y)[0];
            if left > right {
                hash |= 1 << (y * 8 + x);
            }
        }
    }

    Ok(hash)
}

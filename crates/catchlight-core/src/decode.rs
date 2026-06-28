use crate::Result;
use crate::media::DecodedImage;
use std::path::Path;

pub fn decode_image(path: &Path) -> Result<DecodedImage> {
    let img = image::open(path).map_err(|e| crate::Error::Other(format!("decode failed: {e}")))?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Ok(DecodedImage {
        rgba: rgba.into_raw(),
        width,
        height,
    })
}

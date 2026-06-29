use crate::models::{clip_model_path, model_exists};
use image::RgbImage;
use image::imageops::FilterType;
use lightframe_core::Result;
use ndarray::Array4;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

const INPUT_SIZE: u32 = 224;
const MEAN: [f32; 3] = [0.481_454_66, 0.457_827_5, 0.408_210_73];
#[allow(clippy::excessive_precision)]
const STD: [f32; 3] = [0.268_629_54, 0.261_302_58, 0.275_777_11];

pub struct ClipEncoder {
    session: Session,
}

impl ClipEncoder {
    pub fn new(model_path: &Path) -> Result<Self> {
        if !model_exists(model_path) {
            return Err(lightframe_core::Error::Ai(format!(
                "CLIP model not found at {}",
                model_path.display()
            )));
        }

        let session = Session::builder()
            .map_err(|e| lightframe_core::Error::Ai(format!("CLIP session builder: {e}")))?
            .commit_from_file(model_path)
            .map_err(|e| {
                lightframe_core::Error::Ai(format!(
                    "failed to load CLIP model at {}: {e}",
                    model_path.display()
                ))
            })?;

        Ok(Self { session })
    }

    pub fn try_default() -> Result<Self> {
        Self::new(&clip_model_path())
    }

    pub fn encode_image(&mut self, path: &Path) -> Result<Vec<f32>> {
        let tensor = preprocess_image(path)?;
        let outputs = self
            .session
            .run(ort::inputs![TensorRef::from_array_view(&tensor).map_err(
                |e| lightframe_core::Error::Ai(format!("CLIP tensor: {e}"))
            )?])
            .map_err(|e| lightframe_core::Error::Ai(format!("CLIP inference failed: {e}")))?;

        let output = outputs
            .values()
            .next()
            .ok_or_else(|| lightframe_core::Error::Ai("CLIP model returned no outputs".into()))?;

        let array = output
            .try_extract_array::<f32>()
            .map_err(|e| lightframe_core::Error::Ai(format!("CLIP output extraction: {e}")))?;

        let embedding: Vec<f32> = array.iter().copied().collect();
        if embedding.is_empty() {
            return Err(lightframe_core::Error::Ai(
                "CLIP model returned empty embedding".into(),
            ));
        }

        Ok(embedding)
    }
}

fn preprocess_image(path: &Path) -> Result<Array4<f32>> {
    let img = image::open(path)
        .map_err(|e| {
            lightframe_core::Error::Ai(format!("failed to open image {}: {e}", path.display()))
        })?
        .resize_exact(INPUT_SIZE, INPUT_SIZE, FilterType::Lanczos3)
        .to_rgb8();

    let mut array = Array4::<f32>::zeros((1, 3, INPUT_SIZE as usize, INPUT_SIZE as usize));

    for y in 0..INPUT_SIZE {
        for x in 0..INPUT_SIZE {
            let pixel = img.get_pixel(x, y);
            for c in 0..3 {
                let value = pixel[c] as f32 / 255.0;
                array[[0, c, y as usize, x as usize]] = (value - MEAN[c]) / STD[c];
            }
        }
    }

    Ok(array)
}

#[allow(dead_code)]
fn resize_and_center_crop(img: &RgbImage, size: u32) -> RgbImage {
    let w = img.width();
    let h = img.height();
    let scale = (size as f32 / w as f32).max(size as f32 / h as f32);
    let new_w = (w as f32 * scale).round() as u32;
    let new_h = (h as f32 * scale).round() as u32;
    let resized = image::imageops::resize(img, new_w, new_h, FilterType::Triangle);

    let x = (new_w.saturating_sub(size)) / 2;
    let y = (new_h.saturating_sub(size)) / 2;
    image::imageops::crop_imm(&resized, x, y, size, size).to_image()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_fails_when_model_missing() {
        let result = ClipEncoder::new(Path::new("/nonexistent/clip.onnx"));
        assert!(result.is_err());
    }
}

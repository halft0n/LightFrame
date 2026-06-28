pub mod exif;

use catchlight_core::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PhotoMetadata {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub date_taken: Option<chrono::NaiveDateTime>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub focal_length: Option<f64>,
    pub aperture: Option<f64>,
    pub iso: Option<u32>,
    pub shutter_speed: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub orientation: Option<u16>,
}

pub fn extract(path: &Path) -> Result<PhotoMetadata> {
    exif::extract_exif(path)
}

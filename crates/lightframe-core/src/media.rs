use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Photo,
    Video,
    Screenshot,
    LivePhoto,
    Raw,
    Unknown,
}

impl MediaType {
    /// Whether this media type represents a camera RAW file.
    pub fn is_raw(self) -> bool {
        matches!(self, Self::Raw)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaFile {
    pub id: i64,
    pub path: String,
    pub filename: String,
    pub media_type: MediaType,
    pub size_bytes: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub created_at: Option<chrono::NaiveDateTime>,
    pub modified_at: chrono::NaiveDateTime,
    pub blake3_hash: Option<String>,
    pub dhash: Option<u64>,
    pub phash: Option<u64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ThumbnailSize {
    Micro,
    Small,
    Large,
}

impl ThumbnailSize {
    pub fn pixels(self) -> u32 {
        match self {
            Self::Micro => 64,
            Self::Small => 256,
            Self::Large => 1024,
        }
    }
}

/// A fully decoded image in RGBA format, ready for processing.
pub struct DecodedImage {
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

impl DecodedImage {
    fn dimension_error(&self) -> crate::Error {
        crate::Error::Decode(format!(
            "RGBA buffer length {} does not match {}x{}x4={}",
            self.rgba.len(),
            self.width,
            self.height,
            (self.width as usize) * (self.height as usize) * 4,
        ))
    }

    pub fn to_dynamic_image(&self) -> Result<image::DynamicImage, crate::Error> {
        let img =
            image::RgbaImage::from_raw(self.width, self.height, self.rgba.clone())
                .ok_or_else(|| self.dimension_error())?;
        Ok(image::DynamicImage::ImageRgba8(img))
    }

    /// Move ownership of the RGBA buffer into a DynamicImage, avoiding a clone.
    pub fn into_dynamic_image(self) -> Result<image::DynamicImage, crate::Error> {
        let (w, h) = (self.width, self.height);
        let img = image::RgbaImage::from_raw(w, h, self.rgba)
            .ok_or_else(|| {
                crate::Error::Decode(format!(
                    "RGBA buffer does not match {w}x{h}x4={}",
                    (w as usize) * (h as usize) * 4,
                ))
            })?;
        Ok(image::DynamicImage::ImageRgba8(img))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_size_pixels() {
        assert_eq!(ThumbnailSize::Micro.pixels(), 64);
        assert_eq!(ThumbnailSize::Small.pixels(), 256);
        assert_eq!(ThumbnailSize::Large.pixels(), 1024);
    }

    #[test]
    fn media_type_is_raw() {
        assert!(MediaType::Raw.is_raw());
        assert!(!MediaType::Photo.is_raw());
        assert!(!MediaType::Video.is_raw());
        assert!(!MediaType::Screenshot.is_raw());
    }

    #[test]
    fn media_type_equality() {
        assert_eq!(MediaType::Photo, MediaType::Photo);
        assert_ne!(MediaType::Photo, MediaType::Video);
        assert_ne!(MediaType::Screenshot, MediaType::Unknown);
    }

    #[test]
    fn media_type_serde_roundtrip() {
        let original = MediaType::Screenshot;
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: MediaType = serde_json::from_str(&json).unwrap();
        assert_eq!(original, deserialized);
    }

    #[test]
    fn thumbnail_size_serde_roundtrip() {
        for size in [
            ThumbnailSize::Micro,
            ThumbnailSize::Small,
            ThumbnailSize::Large,
        ] {
            let json = serde_json::to_string(&size).unwrap();
            let back: ThumbnailSize = serde_json::from_str(&json).unwrap();
            assert_eq!(size, back);
        }
    }

    #[test]
    fn decoded_image_to_dynamic_image_valid_rgba() {
        let width = 4u32;
        let height = 3u32;
        let rgba = vec![0u8; (width * height * 4) as usize];
        let decoded = DecodedImage {
            rgba,
            width,
            height,
        };
        let img = decoded.to_dynamic_image().expect("valid buffer");
        assert_eq!(img.width(), width);
        assert_eq!(img.height(), height);
    }

    #[test]
    fn decoded_image_to_dynamic_image_short_buffer_fails() {
        let decoded = DecodedImage {
            rgba: vec![0u8; 10],
            width: 4,
            height: 3,
        };
        let err = decoded.to_dynamic_image().unwrap_err();
        assert!(matches!(err, crate::Error::Decode(_)));
        assert!(err.to_string().contains("RGBA buffer length"));
    }

    #[test]
    fn decoded_image_to_dynamic_image_zero_dimensions_valid() {
        let decoded = DecodedImage {
            rgba: vec![],
            width: 0,
            height: 0,
        };
        let img = decoded.to_dynamic_image().expect("0x0 with empty buffer");
        assert_eq!(img.width(), 0);
        assert_eq!(img.height(), 0);
    }

    #[test]
    fn media_file_serialize() {
        let file = MediaFile {
            id: 1,
            path: "/photos/test.jpg".to_string(),
            filename: "test.jpg".to_string(),
            media_type: MediaType::Photo,
            size_bytes: 1024,
            width: Some(1920),
            height: Some(1080),
            created_at: None,
            modified_at: chrono::NaiveDateTime::default(),
            blake3_hash: Some("abc123".to_string()),
            dhash: Some(0xFF00FF00),
            phash: Some(0x1234567890ABCDEF),
            latitude: Some(39.9042),
            longitude: Some(116.4074),
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("test.jpg"));
        assert!(json.contains("Photo"));
    }
}

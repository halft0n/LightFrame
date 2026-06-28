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
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
        for size in [ThumbnailSize::Micro, ThumbnailSize::Small, ThumbnailSize::Large] {
            let json = serde_json::to_string(&size).unwrap();
            let back: ThumbnailSize = serde_json::from_str(&json).unwrap();
            assert_eq!(size, back);
        }
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
            latitude: Some(39.9042),
            longitude: Some(116.4074),
        };

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("test.jpg"));
        assert!(json.contains("Photo"));
    }
}

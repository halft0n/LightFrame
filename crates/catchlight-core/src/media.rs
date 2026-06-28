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

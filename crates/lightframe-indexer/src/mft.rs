use crate::Result;
use std::path::PathBuf;

pub struct MftScanner {
    volume: char,
}

pub struct MftEntry {
    pub path: PathBuf,
    pub size: u64,
    pub created: Option<std::time::SystemTime>,
    pub modified: Option<std::time::SystemTime>,
    pub is_directory: bool,
}

impl MftScanner {
    pub fn new(volume: char) -> Result<Self> {
        Ok(Self { volume })
    }

    pub fn scan_media_files(&self, extensions: &[&str]) -> Result<Vec<MftEntry>> {
        tracing::info!(
            volume = %self.volume,
            extensions = extensions.len(),
            "MFT scan requested"
        );
        Ok(Vec::new())
    }
}

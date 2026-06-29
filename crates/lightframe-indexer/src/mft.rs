#[cfg(target_os = "windows")]
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

pub struct UsnJournal {
    volume: char,
}

impl UsnJournal {
    pub fn new(volume: char) -> Result<Self> {
        Ok(Self { volume })
    }

    pub fn poll_changes(&self) -> Result<Vec<UsnChange>> {
        let _ = self.volume;
        Ok(Vec::new())
    }
}

pub struct UsnChange {
    pub path: PathBuf,
    pub reason: UsnReason,
}

pub enum UsnReason {
    Created,
    Deleted,
    Renamed,
    Modified,
}

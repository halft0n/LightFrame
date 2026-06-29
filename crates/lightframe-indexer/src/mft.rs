#[cfg(target_os = "windows")]
#[allow(dead_code)]
use crate::Result;
use std::path::PathBuf;

#[allow(dead_code)]
pub struct MftScanner {
    volume: char,
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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

#[allow(dead_code)]
pub struct UsnChange {
    pub path: PathBuf,
    pub reason: UsnReason,
}

#[allow(dead_code)]
pub enum UsnReason {
    Created,
    Deleted,
    Renamed,
    Modified,
}

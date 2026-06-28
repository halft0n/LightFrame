#[cfg(target_os = "windows")]
pub mod mft {
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

        /// Scan MFT for all media files on the volume.
        /// Returns paths of files matching media extensions.
        pub fn scan_media_files(&self, extensions: &[&str]) -> Result<Vec<MftEntry>> {
            // Use Windows API: CreateFile on \\.\C:
            // Then FSCTL_ENUM_USN_DATA to enumerate
            // For now, return empty vec as placeholder
            // Real implementation needs windows-sys or winapi crate
            tracing::info!(
                volume = self.volume,
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

        /// Watch for file changes via USN Journal.
        pub fn poll_changes(&self) -> Result<Vec<UsnChange>> {
            // FSCTL_READ_USN_JOURNAL
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
}

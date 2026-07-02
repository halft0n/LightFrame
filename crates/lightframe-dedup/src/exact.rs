use lightframe_core::Result;
use std::fs::File;
use std::io::Read;
use std::path::Path;

const MMAP_THRESHOLD: u64 = 4 * 1024 * 1024;

pub fn blake3_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;

    if metadata.len() >= MMAP_THRESHOLD {
        // Large files: mmap + rayon multi-threaded hashing
        // SAFETY: file is open and we only read from the mapping
        let mmap = unsafe { memmap2::Mmap::map(&file) }?;
        let hash = blake3::Hasher::new().update_rayon(&mmap).finalize();
        return Ok(hash.to_hex().to_string());
    }

    let mut hasher = blake3::Hasher::new();
    let mut buf = vec![0u8; 128 * 1024];
    let mut reader = std::io::BufReader::new(file);
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().to_hex().to_string())
}

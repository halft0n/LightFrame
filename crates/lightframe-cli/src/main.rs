use clap::{Parser, Subcommand};
use lightframe_core::config;
use lightframe_db::Database;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Parser)]
#[command(name = "lightframe-cli", about = "LightFrame photo management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to database file (defaults to standard location)
    #[arg(long, global = true)]
    db: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show library statistics
    Status,
    /// Scan a directory and add media to the library
    Scan {
        /// Directory path to scan
        path: PathBuf,
    },
    /// Export an edited photo
    Export {
        /// Media ID to export
        media_id: i64,
        /// Output directory
        output_dir: PathBuf,
    },
    /// List and optionally remove duplicate files
    Dedup {
        /// Actually delete duplicates (keep first in each group)
        #[arg(long)]
        delete: bool,
    },
}

fn open_db(cli_db: Option<&PathBuf>) -> Result<Database, String> {
    let path = match cli_db {
        Some(p) => p.clone(),
        None => config::db_path(),
    };
    if !path.exists() {
        return Err(format!(
            "database not found at {}. Run the LightFrame app first to create a library.",
            path.display()
        ));
    }
    Database::open(&path).map_err(|e| format!("failed to open database: {e}"))
}

fn cmd_status(db: &Database) -> Result<(), String> {
    let folders = db.list_watched_folders().map_err(|e| e.to_string())?;
    let media_count = db.get_media_count().map_err(|e| e.to_string())?;
    let persons = db.list_persons().map_err(|e| e.to_string())?;
    let albums = db.list_albums().map_err(|e| e.to_string())?;
    let duplicates = db.list_duplicate_groups().map_err(|e| e.to_string())?;
    let favorites_count = db.get_favorites_count().map_err(|e| e.to_string())?;

    println!("=== LightFrame Library Status ===");
    println!("Watched folders:   {}", folders.len());
    println!("Total media:       {}", media_count);
    println!("Favorites:         {}", favorites_count);
    println!("Persons:           {}", persons.len());
    println!("Albums:            {}", albums.len());
    println!("Duplicate groups:  {}", duplicates.len());

    if !folders.is_empty() {
        println!("\nFolders:");
        for f in &folders {
            println!(
                "  {} ({} files, last scan: {})",
                f.path,
                f.media_count,
                f.last_scan.as_deref().unwrap_or("never")
            );
        }
    }

    Ok(())
}

fn cmd_scan(db: &Database, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("directory not found: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("not a directory: {}", path.display()));
    }

    let canonical = path
        .canonicalize()
        .map_err(|e| format!("failed to canonicalize path: {e}"))?;
    let path_str = canonical
        .to_str()
        .ok_or_else(|| "path is not valid UTF-8".to_string())?;

    let folder = db.add_watched_folder(path_str).map_err(|e| e.to_string())?;
    println!("Added watched folder: {} (id={})", path_str, folder.id);
    println!("Existing media in folder: {} files", folder.media_count);
    println!("\nNote: Full scan with thumbnail generation requires the GUI app.");
    println!("Use `lightframe-cli status` to check library stats.");

    Ok(())
}

fn cmd_dedup(db: &Database, delete: bool) -> Result<(), String> {
    let groups = db.list_duplicate_groups().map_err(|e| e.to_string())?;

    if groups.is_empty() {
        println!("No duplicate groups found.");
        println!("Run dedup detection in the GUI app first.");
        return Ok(());
    }

    println!("Found {} duplicate group(s):\n", groups.len());

    for group in &groups {
        println!(
            "Group #{} ({}): {} files",
            group.id,
            group.match_type,
            group.members.len()
        );
        for member in &group.members {
            println!("  [{}] {}", member.media_id, member.path);
        }
        println!();
    }

    if delete {
        println!("--delete flag: removing duplicates (keeping first in each group)...");
        let mut removed = 0;
        for group in &groups {
            for member in group.members.iter().skip(1) {
                let file_path = std::path::Path::new(&member.path);
                if file_path.exists() {
                    match std::fs::remove_file(file_path) {
                        Ok(()) => {
                            // Mark as deleted in DB
                            if let Err(e) = db.soft_delete_by_path(&member.path) {
                                eprintln!("  Warning: removed file but DB update failed: {e}");
                            }
                            println!("  Removed: {}", member.path);
                            removed += 1;
                        }
                        Err(e) => {
                            eprintln!("  Error removing {}: {}", member.path, e);
                        }
                    }
                }
            }
        }
        // Clear the duplicate groups since we've resolved them
        if removed > 0
            && let Err(e) = db.clear_duplicate_groups()
        {
            eprintln!("  Warning: could not clear duplicate groups: {e}");
        }
        println!("\nRemoved {} duplicate file(s).", removed);
    }

    Ok(())
}

fn cmd_export(db: &Database, media_id: i64, output_dir: &Path) -> Result<(), String> {
    if !output_dir.exists() {
        return Err(format!(
            "output directory not found: {}",
            output_dir.display()
        ));
    }
    if !output_dir.is_dir() {
        return Err(format!("not a directory: {}", output_dir.display()));
    }

    let media = db
        .get_media_by_id(media_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("media with id {} not found", media_id))?;

    let source = std::path::Path::new(&media.path);
    if !source.exists() {
        return Err(format!("source file not found: {}", media.path));
    }

    let dest = output_dir.join(&media.filename);
    std::fs::copy(source, &dest).map_err(|e| format!("failed to copy file: {e}"))?;

    println!("Exported: {} -> {}", media.path, dest.display());

    if let Ok(Some(params)) = db.get_edit_params(media_id) {
        println!("Note: this file has edit parameters that were not applied:");
        println!("  {}", params);
        println!("Use the GUI app to export with edits applied.");
    }

    Ok(())
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::Status => {
            let db = match open_db(cli.db.as_ref()) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            };
            cmd_status(&db)
        }
        Commands::Scan { path } => {
            let db = match open_db(cli.db.as_ref()) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            };
            cmd_scan(&db, path)
        }
        Commands::Export {
            media_id,
            output_dir,
        } => {
            let db = match open_db(cli.db.as_ref()) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            };
            cmd_export(&db, *media_id, output_dir)
        }
        Commands::Dedup { delete } => {
            let db = match open_db(cli.db.as_ref()) {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("Error: {e}");
                    process::exit(1);
                }
            };
            cmd_dedup(&db, *delete)
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lightframe_db::Database;
    use std::path::Path;

    fn test_db() -> (Database, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        (db, dir)
    }

    #[test]
    fn test_cmd_status_empty_db() {
        let (db, _dir) = test_db();
        let result = cmd_status(&db);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_status_with_data() {
        let (db, _dir) = test_db();
        db.add_watched_folder("/test/photos").unwrap();
        let result = cmd_status(&db);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_scan_nonexistent_dir() {
        let (db, _dir) = test_db();
        let fake_path = PathBuf::from("/nonexistent/path/that/does/not/exist");
        let result = cmd_scan(&db, &fake_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_cmd_scan_not_a_directory() {
        let (db, dir) = test_db();
        let file_path = dir.path().join("file.txt");
        std::fs::write(&file_path, "test").unwrap();
        let result = cmd_scan(&db, &file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn test_cmd_scan_valid_directory() {
        let (db, _dir) = test_db();
        let scan_dir = tempfile::tempdir().unwrap();
        let result = cmd_scan(&db, &scan_dir.path().to_path_buf());
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_dedup_empty() {
        let (db, _dir) = test_db();
        let result = cmd_dedup(&db, false);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_export_nonexistent_media() {
        let (db, _dir) = test_db();
        let output = tempfile::tempdir().unwrap();
        let result = cmd_export(&db, 99999, &output.path().to_path_buf());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_cmd_export_nonexistent_output_dir() {
        let (db, _dir) = test_db();
        let fake_dir = PathBuf::from("/nonexistent/output");
        let result = cmd_export(&db, 1, &fake_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_open_db_nonexistent_path() {
        let result = open_db(Some(&PathBuf::from("/nonexistent/db.sqlite")));
        assert!(result.is_err());
    }
}

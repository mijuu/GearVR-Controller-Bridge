use tokio::fs;
use std::path::Path;
use anyhow::Result;

/// Asynchronously ensures that a directory exists, creating it if it does not.
/// This function is idempotent.
pub async fn ensure_directory_exists<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        if let Err(e) = fs::create_dir_all(path).await {
            eprintln!("Failed to create directory at {:?}: {}", path, e);
            return Err(e.into());
        }
        eprintln!("Created directory at: {:?}", path);
    }
    Ok(())
}

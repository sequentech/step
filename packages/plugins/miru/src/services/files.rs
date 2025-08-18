use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::result::Result;

use sequent_core::plugins::{get_plugin_shared_dir, Plugins};

// In a real application, you might want a more robust error type.
type AppResult<T> = Result<T, String>;

/// A simple struct to represent a temporary file and ensure it's removed on drop.
///
/// Note: This is a manual implementation of a temporary file helper.
/// The `tempfile` crate would be preferred if it were available and
/// compatible with the WASI environment.
pub struct TempFileGuard {
    path: PathBuf,
}

impl TempFileGuard {
    pub fn new(path: PathBuf) -> Self {
        TempFileGuard { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_file(&self.path) {
            // Log the error but don't panic, as this happens on cleanup.
            eprintln!(
                "Failed to remove temporary file: {:?}, error: {}",
                self.path, e
            );
        }
    }
}

/// Generates a unique filename using a prefix and a suffix.
fn generate_unique_filename(prefix: &str, suffix: &str) -> String {
    // A simple, non-cryptographic way to generate a unique ID.
    // In a production environment, you might use a more robust method.
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{}{}", prefix, timestamp, suffix)
}

/// Creates a new temporary file in the current working directory.
pub fn create_temp_file(prefix: &str, suffix: &str) -> AppResult<(TempFileGuard, String)> {
    let filename = generate_unique_filename(prefix, suffix);
    let base_path = get_plugin_shared_dir(&Plugins::MIRU);
    let path = PathBuf::from(base_path).join(filename.clone());

    File::create(&path).map_err(|e| format!("Error creating temp file at {:?}: {}", path, e))?;

    Ok((TempFileGuard::new(path), filename))
}

/// Writes data into a named temporary file.
pub fn write_into_named_temp_file(
    data: &[u8],
    prefix: &str,
    suffix: &str,
) -> AppResult<(TempFileGuard, String, String, u64)> {
    let (temp_file_guard, file_name) = create_temp_file(prefix, suffix)?;
    let temp_path = temp_file_guard.path();

    {
        let mut file = OpenOptions::new()
            .write(true)
            .open(&temp_path)
            .map_err(|e| format!("Couldn't open file for writing at {:?}: {}", temp_path, e))?;

        file.write_all(data)
            .map_err(|e| format!("Error writing into named temp file: {}", e))?;
    }

    let file_size = get_file_size(&temp_path)?;
    let temp_path_string = temp_path.to_string_lossy().to_string();

    Ok((temp_file_guard, file_name, temp_path_string, file_size))
}

/// Obtains the size of a file.
fn get_file_size(path: &Path) -> AppResult<u64> {
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Error obtaining file metadata for {:?}: {}", path, e))?;
    Ok(metadata.len())
}

// Example usage
// pub fn run_example() {
//     let data = vec![1, 2, 3, 4, 5];
//     let prefix = "my-file";
//     let suffix = ".dat";

//     match write_into_named_temp_file(&data, prefix, suffix) {
//         Ok((_guard, path, size)) => {
//             println!("Successfully wrote to temporary file: {}", path);
//             println!("File size: {} bytes", size);
//             // The `_guard` will drop here, and the file will be removed.
//         }
//         Err(e) => {
//             eprintln!("An error occurred: {}", e);
//         }
//     }
// }

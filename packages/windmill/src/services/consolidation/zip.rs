use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use tracing::instrument;
use walkdir::WalkDir;
use zip::write::{FileOptions, SimpleFileOptions};

#[instrument(skip_all, err)]
pub fn compress_folder_to_zip(src_dir: &Path, dst_file: &Path) -> Result<()> {
    let path = src_dir.clone();
    let file = File::create(dst_file)
        .with_context(|| format!("Failed to create destination file: {:?}", dst_file))?;
    let mut zip = zip::ZipWriter::new(file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for entry in WalkDir::new(path) {
        let entry = entry.with_context(|| format!("Failed to access entry in path: {:?}", path))?;
        let entry_path = entry.path();
        let name = entry_path
            .strip_prefix(Path::new(path))
            .with_context(|| format!("Failed to strip prefix from path: {:?}", entry_path))?;

        if entry_path.is_file() {
            zip.start_file_from_path(name, options)
                .with_context(|| format!("Failed to add file to zip: {:?}", entry_path))?;
            let mut f = File::open(entry_path)
                .with_context(|| format!("Failed to open file: {:?}", entry_path))?;
            io::copy(&mut f, &mut zip)
                .with_context(|| format!("Failed to write file to zip: {:?}", entry_path))?;
        } else if name.as_os_str().len() != 0 {
            zip.add_directory_from_path(name, options)
                .with_context(|| format!("Failed to add directory to zip: {:?}", entry_path))?;
        }
    }
    zip.finish()
        .with_context(|| "Failed to finish writing zip file")?;
    Ok(())
}

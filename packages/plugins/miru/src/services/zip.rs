// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use std::fs;
use std::fs::File;
use std::io;
use std::path::Path;
use tracing::{info, instrument};
use walkdir::WalkDir;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;

#[instrument(skip_all, err)]
pub fn compress_folder_to_zip(src_dir: &Path, dst_file: &Path) -> Result<(), String> {
    let path = src_dir.clone();
    let file = File::create(dst_file)
        .map_err(|e| format!("Failed to create destination file: {:?}", e))?;
    let mut zip = zip::ZipWriter::new(file);

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for entry in WalkDir::new(path) {
        let entry = entry.map_err(|e| format!("Failed to access entry in path: {:?}", e))?;
        let entry_path = entry.path();
        let name = entry_path
            .strip_prefix(Path::new(path))
            .map_err(|e| format!("Failed to strip prefix from path: {:?}", e))?;

        info!("Adding entry to zip :{}", name.display());

        if entry_path.is_file() {
            zip.start_file_from_path(name, options)
                .map_err(|e| format!("Failed to add file to zip: {:?}", e))?;
            let mut f =
                File::open(entry_path).map_err(|e| format!("Failed to open file: {:?}", e))?;
            io::copy(&mut f, &mut zip)
                .map_err(|e| format!("Failed to write file to zip: {:?}", e))?;
        } else if name.as_os_str().len() != 0 {
            zip.add_directory_from_path(name, options)
                .map_err(|e| format!("Failed to add directory to zip: {:?}", e))?;
        }
    }
    zip.finish()
        .map_err(|e| format!("Failed to finish writing zip file: {:?}", e))?;
    Ok(())
}

#[instrument(skip_all, err)]
pub fn unzip_file(src_file: &Path, dst_dir: &Path) -> Result<(), String> {
    let file =
        File::open(src_file).map_err(|e| format!("Failed to open source zip file: {:?}", e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {:?}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Failed to access file in archive at index: {}", i))?;
        let out_path = dst_dir.join(file.sanitized_name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&out_path)
                .map_err(|e| format!("Failed to create directory: {:?}", e))?;
        } else {
            if let Some(p) = out_path.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create directory: {:?}", e))?;
                }
            }
            let mut outfile = File::create(&out_path)
                .map_err(|e| format!("Failed to create output file: {:?}", e))?;
            io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to write file: {:?}", e))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Some(mode) = file.unix_mode() {
                fs::set_permissions(&out_path, fs::Permissions::from_mode(mode)).map_err(|e| {
                    println!("Failed to set permissions for: {:?}: {:?}", out_path, e);
                    e.to_string()
                })?;
            }
        }
    }
    Ok(())
}

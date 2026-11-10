// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! `trustee destroy`: once the election is over, the trustee's keys file and
//! its session stores are overwritten with zeros, flushed to the disk and
//! removed.
//!
//! Nothing is touched unless the keys file reads as a trustee keys file, the
//! store directory holds regular files only and every one of these files can
//! be overwritten, so a wrong path removes nothing. The keys file goes first,
//! as the one that holds the secrets.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};

use crate::identity;

/// The size of the block of zeros a file is overwritten with, one write at a
/// time.
const ZERO_BLOCK: usize = 64 * 1024;

/// What an error before the first removal says.
const NOTHING_REMOVED: &str = "nothing was removed";

/// Destroy the keys file `keys_file` and the session stores in
/// `store_directory`, reporting each removal to `out`.
pub(crate) fn run(
    keys_file: &Path,
    store_directory: &Path,
    out: &mut impl Write,
) -> Result<()> {
    // Every check comes before the first removal.
    ensure_overwritable(keys_file).context(NOTHING_REMOVED)?;
    let public_keys = identity::load(keys_file)
        .context(NOTHING_REMOVED)?
        .public_keys()?;
    let stores = store_files(store_directory).context(NOTHING_REMOVED)?;

    let size = overwrite_and_remove(keys_file)?;
    writeln!(
        out,
        "destroyed the keys file {} ({size} bytes) of",
        keys_file.display()
    )?;
    identity::print_public_keys(out, &public_keys)?;
    for store in &stores {
        let size = overwrite_and_remove(store)?;
        writeln!(
            out,
            "destroyed the session store {} ({size} bytes)",
            store.display()
        )?;
    }
    match fs::remove_dir(store_directory) {
        Ok(()) => {
            sync_parent_directory(store_directory)?;
            writeln!(
                out,
                "removed the session store directory {}",
                store_directory.display()
            )?;
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        // The directory a volume is mounted on stays, empty.
        Err(err) if err.kind() == io::ErrorKind::ResourceBusy => {
            writeln!(
                out,
                "kept the empty session store directory {}: a volume is \
                 mounted on it",
                store_directory.display()
            )?;
        }
        Err(err) => {
            return Err(err).with_context(|| {
                format!("removing {}", store_directory.display())
            })
        }
    }
    Ok(())
}

/// The session stores' files in `directory`: every entry, each of which must
/// be a regular file that can be overwritten. None when the directory does
/// not exist.
fn store_files(directory: &Path) -> Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(Vec::new())
        }
        Err(err) => {
            return Err(err)
                .with_context(|| format!("reading {}", directory.display()))
        }
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry
            .with_context(|| format!("reading {}", directory.display()))?;
        let path = entry.path();
        // `DirEntry::file_type` does not follow symbolic links.
        if !entry
            .file_type()
            .with_context(|| format!("reading {}", path.display()))?
            .is_file()
        {
            bail!("{} is not a session store file", path.display());
        }
        ensure_overwritable(&path)?;
        files.push(path);
    }
    files.sort();
    Ok(files)
}

fn ensure_regular_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("reading {}", path.display()))?;
    if !metadata.file_type().is_file() {
        bail!("{} is not a regular file", path.display());
    }
    Ok(())
}

/// A regular file this process may write to. Opening it changes nothing.
fn ensure_overwritable(path: &Path) -> Result<()> {
    ensure_regular_file(path)?;
    OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("opening {} for writing", path.display()))?;
    Ok(())
}

/// Overwrite the regular file `path` with zeros, flush it to the disk, remove
/// it and flush the removal. The number of bytes overwritten.
fn overwrite_and_remove(path: &Path) -> Result<u64> {
    ensure_regular_file(path)?;
    let mut file = OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let length = file
        .metadata()
        .with_context(|| format!("reading {}", path.display()))?
        .len();
    let zeros = vec![0u8; ZERO_BLOCK];
    let mut written: u64 = 0;
    while written < length {
        let block = usize::try_from(length - written)
            .map_or(ZERO_BLOCK, |left| left.min(ZERO_BLOCK));
        file.write_all(&zeros[..block])
            .with_context(|| format!("overwriting {}", path.display()))?;
        written += block as u64;
    }
    // The zeros reach the disk before the file is removed: the dirty pages of
    // a removed file need never be written.
    file.sync_all()
        .with_context(|| format!("flushing {}", path.display()))?;
    drop(file);
    fs::remove_file(path)
        .with_context(|| format!("removing {}", path.display()))?;
    sync_parent_directory(path)?;
    Ok(length)
}

/// Flush the directory that holds `path`, so that a removal in it is on the
/// disk.
fn sync_parent_directory(path: &Path) -> Result<()> {
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        Some(_) | None => Path::new("."),
    };
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .with_context(|| format!("flushing {}", parent.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STORE: &str = "board-1.sqlite";
    const JOURNAL: &str = "board-1.sqlite-journal";

    /// A keys file as `trustee generate` writes it, and a store directory
    /// with a store larger than one block and its journal.
    fn trustee_files(dir: &Path) -> (PathBuf, PathBuf) {
        let keys_file = dir.join("trustee.toml");
        identity::generate(Some(keys_file.as_path())).unwrap();
        let stores = dir.join("message_store");
        fs::create_dir(&stores).unwrap();
        fs::write(stores.join(STORE), vec![0x5a; 2 * ZERO_BLOCK + 7]).unwrap();
        fs::write(stores.join(JOURNAL), b"journal").unwrap();
        (keys_file, stores)
    }

    #[test]
    fn the_keys_file_and_every_session_store_are_removed() {
        let dir = tempfile::tempdir().unwrap();
        let (keys_file, stores) = trustee_files(dir.path());
        let public_keys =
            identity::load(&keys_file).unwrap().public_keys().unwrap();
        let keys_file_size = fs::metadata(&keys_file).unwrap().len();

        let mut out = Vec::new();
        run(&keys_file, &stores, &mut out).unwrap();

        assert!(!keys_file.exists());
        assert!(!stores.exists());
        let report = String::from_utf8(out).unwrap();
        let expected = format!(
            "destroyed the keys file {} ({keys_file_size} bytes) of\n\
             public_key: {}\n\
             share_encryption_public_key: {}\n\
             destroyed the session store {} ({} bytes)\n\
             destroyed the session store {} (7 bytes)\n\
             removed the session store directory {}\n",
            keys_file.display(),
            public_keys.signing_public_key,
            public_keys.share_encryption_public_key,
            stores.join(STORE).display(),
            2 * ZERO_BLOCK + 7,
            stores.join(JOURNAL).display(),
            stores.display(),
        );
        assert_eq!(report, expected);
    }

    /// The overwrite itself, seen through a second link to the same file.
    #[test]
    fn a_file_is_overwritten_with_zeros_before_it_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(STORE);
        let witness = dir.path().join("witness");
        let length = 2 * ZERO_BLOCK + 3;
        fs::write(&path, vec![0x5a; length]).unwrap();
        fs::hard_link(&path, &witness).unwrap();

        assert_eq!(overwrite_and_remove(&path).unwrap(), length as u64);

        assert!(!path.exists());
        assert_eq!(fs::read(&witness).unwrap(), vec![0u8; length]);
    }

    #[test]
    fn without_session_stores_the_keys_file_is_still_destroyed() {
        let dir = tempfile::tempdir().unwrap();
        let keys_file = dir.path().join("trustee.toml");
        identity::generate(Some(keys_file.as_path())).unwrap();

        let mut out = Vec::new();
        run(&keys_file, &dir.path().join("message_store"), &mut out).unwrap();

        assert!(!keys_file.exists());
        let report = String::from_utf8(out).unwrap();
        assert_eq!(report.lines().count(), 3, "{report}");
    }

    #[test]
    fn a_file_that_is_not_a_keys_file_removes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (keys_file, stores) = trustee_files(dir.path());
        let other = dir.path().join("notes.toml");
        fs::write(&other, "title = \"not a keys file\"\n").unwrap();

        let refused = run(&other, &stores, &mut Vec::new()).unwrap_err();

        let error = format!("{refused:#}");
        assert!(error.contains(NOTHING_REMOVED), "{error}");
        assert!(other.exists());
        assert!(keys_file.exists());
        assert!(stores.join(STORE).exists());
    }

    #[test]
    fn a_missing_keys_file_removes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (_, stores) = trustee_files(dir.path());
        let missing = dir.path().join("missing.toml");

        let refused = run(&missing, &stores, &mut Vec::new()).unwrap_err();

        let error = format!("{refused:#}");
        assert!(error.contains(NOTHING_REMOVED), "{error}");
        assert!(error.contains(&missing.display().to_string()), "{error}");
        assert!(stores.join(STORE).exists());
    }

    #[test]
    fn a_store_directory_holding_anything_but_files_removes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (keys_file, stores) = trustee_files(dir.path());
        fs::create_dir(stores.join("nested")).unwrap();

        let refused = run(&keys_file, &stores, &mut Vec::new()).unwrap_err();

        let error = format!("{refused:#}");
        assert!(error.contains(NOTHING_REMOVED), "{error}");
        assert!(error.contains("nested"), "{error}");
        assert!(keys_file.exists());
        assert!(stores.join(STORE).exists());
    }

    #[test]
    fn a_keys_file_that_cannot_be_written_removes_nothing() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let (keys_file, stores) = trustee_files(dir.path());
        fs::set_permissions(&keys_file, fs::Permissions::from_mode(0o400))
            .unwrap();
        if OpenOptions::new().write(true).open(&keys_file).is_ok() {
            // A process that ignores file permissions, such as root's.
            return;
        }

        let refused = run(&keys_file, &stores, &mut Vec::new()).unwrap_err();

        let error = format!("{refused:#}");
        assert!(error.contains(NOTHING_REMOVED), "{error}");
        assert!(error.contains("for writing"), "{error}");
        assert!(keys_file.exists());
        assert!(stores.join(STORE).exists());
    }

    #[test]
    fn a_link_in_place_of_the_keys_file_removes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let (keys_file, stores) = trustee_files(dir.path());
        let link = dir.path().join("link.toml");
        std::os::unix::fs::symlink(&keys_file, &link).unwrap();

        let refused = run(&link, &stores, &mut Vec::new()).unwrap_err();

        let error = format!("{refused:#}");
        assert!(error.contains(NOTHING_REMOVED), "{error}");
        assert!(error.contains("not a regular file"), "{error}");
        assert!(keys_file.exists());
        assert!(stores.join(STORE).exists());
    }
}

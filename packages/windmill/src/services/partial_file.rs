// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Output files that appear under their final name only once they are complete.

use anyhow::{Context, Result};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Suffix of a file that is still being written.
pub const PARTIAL_SUFFIX: &str = ".partial";

/// A file written as `<output>.partial` and renamed to `<output>` by
/// [`PartialFile::commit`]. Dropping it without committing removes the partial
/// file and leaves `<output>` untouched.
pub struct PartialFile {
    file: Option<File>,
    partial: PathBuf,
    output: PathBuf,
}

impl PartialFile {
    pub fn create(output: impl Into<PathBuf>) -> Result<Self> {
        let output = output.into();
        let partial = Self::partial_path(&output);
        let file = File::create(&partial)
            .with_context(|| format!("Failed to create {}", partial.display()))?;
        Ok(Self {
            file: Some(file),
            partial,
            output,
        })
    }

    pub fn partial_path(output: &Path) -> PathBuf {
        let mut name = output.as_os_str().to_owned();
        name.push(PARTIAL_SUFFIX);
        PathBuf::from(name)
    }

    pub fn path(&self) -> &Path {
        &self.partial
    }

    /// Flush the file to disk and move it to its final name, replacing any
    /// existing file there.
    pub fn commit(mut self) -> Result<PathBuf> {
        let mut file = self
            .file
            .take()
            .context("Partial file was already committed")?;
        file.flush()
            .and_then(|()| file.sync_all())
            .with_context(|| format!("Failed to write {}", self.partial.display()))?;
        drop(file);
        std::fs::rename(&self.partial, &self.output).with_context(|| {
            format!(
                "Failed to move {} to {}",
                self.partial.display(),
                self.output.display()
            )
        })?;
        Ok(self.output.clone())
    }

    fn file(&mut self) -> std::io::Result<&mut File> {
        self.file
            .as_mut()
            .ok_or_else(|| std::io::Error::other("Partial file was already committed"))
    }
}

impl Write for PartialFile {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file()?.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file()?.flush()
    }
}

impl Drop for PartialFile {
    fn drop(&mut self) {
        if self.file.take().is_some() {
            let _ = std::fs::remove_file(&self.partial);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_output_appears_only_on_commit() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("votes.csv");
        let mut file = PartialFile::create(&output).unwrap();
        file.write_all(b"a,b\n").unwrap();

        assert!(!output.exists());
        assert!(directory.path().join("votes.csv.partial").exists());

        assert_eq!(file.commit().unwrap(), output);
        assert_eq!(std::fs::read(&output).unwrap(), b"a,b\n");
        assert!(!directory.path().join("votes.csv.partial").exists());
    }

    #[test]
    fn dropping_without_commit_removes_the_partial_file_and_keeps_the_old_output() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("votes.csv");
        std::fs::write(&output, b"previous export").unwrap();

        {
            let mut file = PartialFile::create(&output).unwrap();
            file.write_all(b"half of a new export").unwrap();
        }

        assert_eq!(std::fs::read(&output).unwrap(), b"previous export");
        assert!(!PartialFile::partial_path(&output).exists());
    }

    #[test]
    fn a_commit_replaces_the_old_output() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("votes.csv");
        std::fs::write(&output, b"previous export").unwrap();

        let mut file = PartialFile::create(&output).unwrap();
        file.write_all(b"new export").unwrap();
        file.commit().unwrap();

        assert_eq!(std::fs::read(&output).unwrap(), b"new export");
    }

    #[test]
    fn a_missing_directory_is_reported() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("missing").join("votes.csv");

        let error = match PartialFile::create(&output) {
            Ok(_) => panic!("creating a file in a missing directory must fail"),
            Err(error) => format!("{error:#}"),
        };

        assert!(error.contains("votes.csv.partial"), "{error}");
    }
}

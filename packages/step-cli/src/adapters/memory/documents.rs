// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::documents::{Downloader, Uploader};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The id every successful upload returns.
pub const UPLOADED_DOCUMENT_ID: &str = "uploaded-document";

#[derive(Clone, Debug, PartialEq)]
pub struct Upload {
    pub file_path: PathBuf,
    pub is_local: bool,
    pub election_event_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Download {
    pub election_event_id: String,
    pub document_id: String,
    pub output_path: PathBuf,
}

/// Records uploads and downloads without storing any file.
#[derive(Debug, Default)]
pub struct MemoryDocuments {
    failure: Option<String>,
    uploads: Mutex<Vec<Upload>>,
    downloads: Mutex<Vec<Download>>,
}

impl MemoryDocuments {
    pub fn uploads(&self) -> Vec<Upload> {
        self.uploads.lock().expect("uploads lock").clone()
    }

    fn outcome(&self) -> Result<(), Box<dyn Error>> {
        match &self.failure {
            Some(message) => Err(Box::from(message.as_str())),
            None => Ok(()),
        }
    }
}

impl Uploader for MemoryDocuments {
    fn upload(
        &self,
        file_path: &Path,
        is_local: bool,
        election_event_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        self.uploads.lock().expect("uploads lock").push(Upload {
            file_path: file_path.to_path_buf(),
            is_local,
            election_event_id: election_event_id.to_string(),
        });
        self.outcome()?;
        Ok(UPLOADED_DOCUMENT_ID.to_string())
    }
}

impl Downloader for MemoryDocuments {
    fn download(
        &self,
        election_event_id: &str,
        document_id: &str,
        output_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        self.downloads
            .lock()
            .expect("downloads lock")
            .push(Download {
                election_event_id: election_event_id.to_string(),
                document_id: document_id.to_string(),
                output_path: output_path.to_path_buf(),
            });
        self.outcome()
    }
}

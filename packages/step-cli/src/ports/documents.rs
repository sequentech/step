// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::error::Error;
use std::path::Path;

/// Stores a local file as a document of an election event.
pub trait Uploader {
    /// Returns the id of the new document.
    fn upload(
        &self,
        file_path: &Path,
        is_local: bool,
        election_event_id: &str,
    ) -> Result<String, Box<dyn Error>>;
}

/// Saves a document of an election event to a local file.
pub trait Downloader {
    fn download(
        &self,
        election_event_id: &str,
        document_id: &str,
        output_path: &Path,
    ) -> Result<(), Box<dyn Error>>;
}

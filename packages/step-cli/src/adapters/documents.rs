// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::documents::{Downloader, Uploader};
use crate::utils::tally::download_document::{download_file, fetch_document};
use crate::utils::upload_file::GetUploadUrl;
use std::error::Error;
use std::path::Path;

/// Documents behind Hasura's presigned storage URLs.
#[derive(Clone, Copy, Debug, Default)]
pub struct HasuraDocuments;

impl Uploader for HasuraDocuments {
    fn upload(
        &self,
        file_path: &Path,
        is_local: bool,
        election_event_id: &str,
    ) -> Result<String, Box<dyn Error>> {
        GetUploadUrl::upload_for_election_event(
            file_path.to_string_lossy().to_string(),
            is_local,
            Some(election_event_id.to_string()),
        )
    }
}

impl Downloader for HasuraDocuments {
    fn download(
        &self,
        election_event_id: &str,
        document_id: &str,
        output_path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        let document = fetch_document(election_event_id, document_id)?;
        download_file(&document.url, &output_path.to_string_lossy())
    }
}

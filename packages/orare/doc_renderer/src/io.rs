use serde::{Deserialize, Serialize};

use sequent_core::services::pdf::PrintToPdfOptions;

#[derive(Debug, Deserialize)]
pub struct Input {
    pub html: String,
    #[serde(default)]
    pub pdf_options: Option<PrintToPdfOptions>,
    #[serde(default)]
    pub bucket: Option<String>,
    #[serde(default)]
    pub bucket_path: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Output {
    pub pdf_base64: String,
}

use serde::{Deserialize, Serialize};

use crate::openwhisk::PrintToPdfOptions;

#[derive(Debug, Deserialize)]
pub struct Input {
    pub html: Option<String>,
    #[serde(default)]
    pub pdf_options: Option<PrintToPdfOptions>,
}

#[derive(Debug, Serialize)]
pub struct Output {
    pub pdf_base64: String,
}

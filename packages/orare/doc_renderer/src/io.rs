use serde::{Deserialize, Serialize};

use crate::openwhisk::PrintToPdfOptions;

#[derive(Deserialize)]
pub struct Input {
    pub html: Option<String>,
    #[serde(default)]
    pub pdf_options: Option<PrintToPdfOptions>,
}

#[derive(Serialize)]
pub struct Output {
    pub pdf_base64: String,
}

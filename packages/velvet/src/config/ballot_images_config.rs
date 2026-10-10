// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use sequent_core::{
    signatures::ecies_encrypt::EciesKeyPair,
    types::templates::{PrintToPdfOptionsLocal, ReportOptions},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::str::FromStr;
use strum_macros::{Display, EnumString};
use tracing::instrument;

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeConfigBallotImages {
    pub template: String,
    pub system_template: String,
    pub extra_data: Value,
    pub enable_pdfs: bool,
    pub pdf_options: Option<PrintToPdfOptionsLocal>,
    pub report_options: Option<ReportOptions>,
    pub execution_annotations: Option<HashMap<String, String>>,
    pub acm_key: Option<EciesKeyPair>,
}

pub const DEFAULT_MCBALLOT_TITLE: &str = "Ballot images";

pub const BALLOT_IMAGE_SIGNATURE_POLICY_ANNOTATION: &str = "ballot-images:signature-policy";

/// What the signature on each multi-contest ballot image page covers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Display, EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum BallotImageSignaturePolicy {
    /// `event:precinct:serial:election:page`, the original format.
    #[default]
    IdentifiersOnly,
    /// A versioned, length-prefixed payload with the page identifiers, the
    /// contest id and a digest of the selected candidates.
    IdentifiersAndChoices,
}

impl BallotImageSignaturePolicy {
    /// Reads the policy from the election event annotations, defaulting to
    /// `IdentifiersOnly` when the annotation is absent.
    pub fn from_annotations(
        annotations: &HashMap<String, String>,
    ) -> Result<Self, strum::ParseError> {
        annotations
            .get(BALLOT_IMAGE_SIGNATURE_POLICY_ANNOTATION)
            .map(|value| Self::from_str(value))
            .transpose()
            .map(Option::unwrap_or_default)
    }
}

impl PipeConfigBallotImages {
    #[instrument(skip_all, name = "PipeConfigBallotImages::new")]
    pub fn new() -> Self {
        Self::default()
    }

    #[instrument(skip_all, name = "PipeConfigBallotImages::mcballot")]
    pub fn mcballot() -> Self {
        let html: &str = include_str!("../resources/ballot_images_user.hbs");
        let system_html = include_str!("../resources/ballot_images_system.hbs");

        Self {
            template: html.to_string(),
            system_template: system_html.to_string(),
            extra_data: json!({
                "title": DEFAULT_MCBALLOT_TITLE,
                "file_logo": "http://minio:9000/public/public-assets/sequent-logo.svg",
                "file_qrcode_lib": "http://minio:9000/public/public-assets/qrcode.min.js"
            }),
            enable_pdfs: true,
            pdf_options: None,
            report_options: None,
            execution_annotations: None,
            acm_key: None,
        }
    }
}

impl Default for PipeConfigBallotImages {
    #[instrument(skip_all, name = "PipeConfigBallotImages::default")]
    fn default() -> Self {
        let html: &str = include_str!("../resources/ballot_images_user.hbs");
        let system_html = include_str!("../resources/ballot_images_system.hbs");

        Self {
            template: html.to_string(),
            system_template: system_html.to_string(),
            extra_data: json!("{}"),
            enable_pdfs: true,
            pdf_options: None,
            report_options: None,
            execution_annotations: None,
            acm_key: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_policy_defaults_to_identifiers_only() {
        let policy = BallotImageSignaturePolicy::from_annotations(&HashMap::new());

        assert_eq!(policy, Ok(BallotImageSignaturePolicy::IdentifiersOnly));
    }

    #[test]
    fn signature_policy_reads_annotation() {
        let annotations = HashMap::from([(
            BALLOT_IMAGE_SIGNATURE_POLICY_ANNOTATION.to_string(),
            "IDENTIFIERS_AND_CHOICES".to_string(),
        )]);

        assert_eq!(
            BallotImageSignaturePolicy::from_annotations(&annotations),
            Ok(BallotImageSignaturePolicy::IdentifiersAndChoices)
        );
    }

    #[test]
    fn signature_policy_rejects_unknown_value() {
        let annotations = HashMap::from([(
            BALLOT_IMAGE_SIGNATURE_POLICY_ANNOTATION.to_string(),
            "identifiers-and-choices".to_string(),
        )]);

        assert!(BallotImageSignaturePolicy::from_annotations(&annotations).is_err());
    }
}

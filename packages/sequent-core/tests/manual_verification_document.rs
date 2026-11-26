// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

#![cfg(feature = "default_features")]

use sequent_core::types::hasura::core::Document;
use serde_json::{json, Value};

fn document(annotations: Option<Value>) -> Document {
    // Neither the filename nor a public-facing legacy label establishes the
    // protected document's purpose. Only the exact typed annotation does.
    serde_json::from_value(json!({
        "id": "manual-verification",
        "name": "signed-manual-verification.pdf",
        "annotations": annotations,
    }))
    .unwrap()
}

#[test]
fn manual_verification_requires_the_exact_document_purpose() {
    assert!(
        document(Some(json!({"purpose": "signed-manual-verification"})))
            .is_signed_manual_verification()
    );

    for annotations in [
        None,
        Some(json!({})),
        Some(json!({"purpose": null})),
        Some(json!({"purpose": ["signed-manual-verification"]})),
        Some(json!({"purpose": "Signed-Manual-Verification"})),
        Some(json!({"purpose": "ballot-receipt"})),
        Some(json!("signed-manual-verification")),
    ] {
        assert!(!document(annotations).is_signed_manual_verification());
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

/// Loads the election-event template as opaque JSON.
///
/// Deliberately not `sequent_core::election_config::ImportElectionEventSchema`:
/// that type requires a `reports` field with no serde default, which
/// `packages/step-cli/data/test-election-template.json` — the fixture this
/// tool is meant to reuse — predates and doesn't have. `step-cli
/// import-election` itself never deserializes the template locally either;
/// it uploads the file as raw bytes and lets the server validate it. This
/// loader only checks the file is well-formed JSON, so a broken template
/// fails fast instead of surfacing as a confusing import error later.
pub fn load_election_event_template(path: &Path) -> Result<Value> {
    let contents = std::fs::read_to_string(path).with_context(|| {
        format!(
            "failed to read election event template at {}",
            path.display()
        )
    })?;
    parse_template_str(&contents).with_context(|| {
        format!(
            "failed to parse election event template at {}",
            path.display()
        )
    })
}

fn parse_template_str(contents: &str) -> Result<Value> {
    Ok(serde_json::from_str(contents)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
        "tenant_id": "9384db41-1b21-4b93-a6aa-edfc007136d8",
        "election_event": {
            "id": "11111111-1111-5111-8111-111111111111"
        }
    }"#;

    #[test]
    fn a_well_formed_template_parses() {
        let parsed = parse_template_str(VALID).unwrap();
        assert_eq!(parsed["tenant_id"], "9384db41-1b21-4b93-a6aa-edfc007136d8");
    }

    #[test]
    fn the_real_fixture_parses_as_opaque_json() {
        // Confirms this loader doesn't reject the fixture the way strict
        // deserialization into ImportElectionEventSchema does (it's missing
        // `reports`, which that type requires) — see the module doc comment.
        let contents =
            std::fs::read_to_string("../step-cli/data/test-election-template.json").unwrap();
        assert!(parse_template_str(&contents).is_ok());
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(parse_template_str("{ not json").is_err());
    }

    #[test]
    fn load_election_event_template_reports_a_missing_file_clearly() {
        let err = load_election_event_template(Path::new("/nonexistent/path/election-event.json"))
            .unwrap_err();
        assert!(
            err.to_string().contains("election event template"),
            "unexpected error: {err}"
        );
    }
}

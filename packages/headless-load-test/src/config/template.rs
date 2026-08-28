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
    fn the_bundled_template_has_a_working_voting_portal_client() {
        // `data/election-event-template.json` exists specifically because
        // `step-cli`'s fixture (above) doesn't set this up: its
        // `voting-portal` client has no
        // `authorized-elections-oidc-usermodel-attribute-mapper`, so a
        // voter provisioned against it can never get an
        // `authorized-election-ids` claim and Phase 2 casting 401s. This
        // guards against that regressing silently if the bundled template
        // is ever swapped out.
        let contents = std::fs::read_to_string("data/election-event-template.json").unwrap();
        let template = parse_template_str(&contents).unwrap();

        let clients = template["keycloak_event_realm"]["clients"]
            .as_array()
            .expect("keycloak_event_realm.clients should be an array");
        let voting_portal = clients
            .iter()
            .find(|client| client["clientId"] == "voting-portal")
            .expect("template should have a voting-portal client");
        let mapper_types: Vec<&str> = voting_portal["protocolMappers"]
            .as_array()
            .expect("voting-portal should have protocolMappers")
            .iter()
            .filter_map(|mapper| mapper["protocolMapper"].as_str())
            .collect();
        assert!(
            mapper_types.contains(&"authorized-elections-oidc-usermodel-attribute-mapper"),
            "voting-portal should carry the authorized-elections mapper, got {mapper_types:?}"
        );
    }

    #[test]
    fn the_bundled_template_gives_voters_a_top_level_auth_time_claim() {
        // Without this mapper a password-grant voter token has no
        // top-level `auth_time` claim (unlike a browser/authorization-code
        // login, which gets one from Keycloak automatically). Harvest's
        // `check_status` (packages/windmill/src/services/insert_cast_vote.rs)
        // requires `auth_time` for the ONLINE channel and rejects the cast
        // with `CheckStatusFailed("auth_time is not a valid integer")`
        // otherwise — every ONLINE cast in a headless-load-test run would
        // fail. Guards against that regressing silently if the bundled
        // template is ever swapped out.
        let contents = std::fs::read_to_string("data/election-event-template.json").unwrap();
        let template = parse_template_str(&contents).unwrap();

        let clients = template["keycloak_event_realm"]["clients"]
            .as_array()
            .expect("keycloak_event_realm.clients should be an array");
        for client_id in ["voting-portal", "onsite-voting-portal"] {
            let client = clients
                .iter()
                .find(|client| client["clientId"] == client_id)
                .unwrap_or_else(|| panic!("template should have a {client_id} client"));
            let has_auth_time_mapper = client["protocolMappers"]
                .as_array()
                .unwrap_or_else(|| panic!("{client_id} should have protocolMappers"))
                .iter()
                .any(|mapper| {
                    mapper["protocolMapper"] == "oidc-usersessionmodel-note-mapper"
                        && mapper["config"]["user.session.note"] == "AUTH_TIME"
                        && mapper["config"]["claim.name"] == "auth_time"
                });
            assert!(
                has_auth_time_mapper,
                "{client_id} should carry a top-level auth_time claim mapper"
            );
        }
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

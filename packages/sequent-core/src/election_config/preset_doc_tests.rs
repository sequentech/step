// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The shipped presets, proved identical whether they come from Rust or JSON.
//!
//! This is the whole safety net for turning four functions into a file. Nobody
//! can read a two-hundred-line Keycloak realm patch and be sure a transcription
//! is faithful, so nobody is asked to: the file is *generated* from the functions
//! and then pinned, and every assertion below compares the two paths over the
//! same input rather than comparing either to a human's expectations.
//!
//! Regenerate after an intentional change to a preset:
//!
//! ```text
//! SEQUENT_WRITE_DEFAULT_PROFILE=1 cargo test --features … default_profile
//! ```

use serde_json::{json, Map, Value};

use super::*;
use crate::election_config::presets::{self, PresetInput, PRESETS};

/// Every preset as a document, derived from the Rust ones.
///
/// The patch template comes out of `build()` itself: each parameter is fed in as
/// its own placeholder, so whatever the function interpolates lands in the
/// template already spelled `{{that_parameter}}`. There is nothing to transcribe
/// and so nothing to transcribe wrongly.
fn derived() -> Vec<AuthPresetDoc> {
    PRESETS
        .iter()
        .map(|preset| {
            let input = PresetInput::new(
                preset
                    .consumes()
                    .iter()
                    .filter(|key| **key != presets::PARAM_AUTH_TYPE)
                    .map(|key| {
                        (
                            (*key).to_string(),
                            Value::String(format!("{{{{{key}}}}}")),
                        )
                    })
                    .collect(),
            );
            let built = preset.build(&input);

            AuthPresetDoc {
                name: preset.name.to_string(),
                summary: preset.summary.to_string(),
                uses_otp: preset.uses_otp,
                requires: preset
                    .requires
                    .iter()
                    .map(|need| NeedsDoc {
                        kind: need.kind.to_string(),
                        name: need.name.to_string(),
                        why: need.why.to_string(),
                    })
                    .collect(),
                required_parameters: preset
                    .required_parameters
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect(),
                optional_parameters: preset
                    .optional_parameters
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect(),
                profile_attributes: preset
                    .profile_attributes
                    .iter()
                    .map(|key| (*key).to_string())
                    .collect(),
                parameter_defaults: fallbacks(preset.name),
                patch: PatchDoc {
                    patch: built.patch,
                    bind_authenticator_config: built
                        .bind_authenticator_config
                        .map(|(authenticator, config_alias)| BindDoc {
                            authenticator,
                            config_alias,
                        }),
                    user_profile: built.user_profile,
                },
            }
        })
        .collect()
}

/// The `input.text(KEY, fallback)` fallbacks, which are not introspectable.
///
/// The one thing here that *is* transcribed, and the reason
/// `an_absent_parameter_falls_back_the_way_the_function_did` exists: it builds
/// both paths with no parameters at all, so a wrong fallback here is a failing
/// test rather than a realm with `6` where the client asked for `8`.
fn fallbacks(name: &str) -> Map<String, Value> {
    match name {
        "saml_sso_idp_initiated" => match json!({
            presets::PARAM_SAML_IDP_ALIAS: "client-saml-idp",
            presets::PARAM_SAML_METADATA_URL: "",
            presets::PARAM_SAML_PRINCIPAL_ATTRIBUTE: "username",
        }) {
            Value::Object(map) => map,
            _ => Map::new(),
        },
        "otp_email_or_sms" | "voter_link_plus_dob" => match json!({
            presets::PARAM_OTP_LENGTH: "6",
            presets::PARAM_OTP_TTL_SECONDS: "900",
            presets::PARAM_OTP_SENDER_ID: "Sequent",
        }) {
            Value::Object(map) => map,
            _ => Map::new(),
        },
        _ => Map::new(),
    }
}

/// A document's parameters, the way a workbook would supply them.
fn filled() -> PresetInput {
    PresetInput::new(vec![
        (
            presets::PARAM_SAML_IDP_ALIAS.to_string(),
            Value::String("union-idp".to_string()),
        ),
        (
            presets::PARAM_SAML_METADATA_URL.to_string(),
            Value::String("https://idp.example.org/metadata".to_string()),
        ),
        (
            presets::PARAM_SAML_PRINCIPAL_ATTRIBUTE.to_string(),
            Value::String("memberId".to_string()),
        ),
        (
            presets::PARAM_OTP_SENDER_ID.to_string(),
            Value::String("Local 1000".to_string()),
        ),
        // A number, not a string. A spreadsheet cell is typed, a form field is
        // not, and Keycloak's config maps are stringly typed either way.
        (presets::PARAM_OTP_LENGTH.to_string(), json!(8)),
        (presets::PARAM_OTP_TTL_SECONDS.to_string(), json!(600)),
    ])
}

#[test]
fn the_document_builds_what_the_function_built() {
    // The assertion the whole change rests on: for a filled-in document, a
    // preset read from JSON and a preset written in Rust produce the same realm.
    let input = filled();
    for (preset, doc) in PRESETS.iter().zip(derived()) {
        assert_eq!(
            doc.build(&input),
            preset.build(&input),
            "{} differs once it is a document",
            preset.name
        );
    }
}

#[test]
fn an_absent_parameter_falls_back_the_way_the_function_did() {
    // The other half, and the one that catches a wrong `parameter_defaults`:
    // nothing supplied at all, so both paths are running on their fallbacks.
    let nothing = PresetInput::default();
    for (preset, doc) in PRESETS.iter().zip(derived()) {
        assert_eq!(
            doc.build(&nothing),
            preset.build(&nothing),
            "{} differs when the document says nothing",
            preset.name
        );
    }
}

#[test]
fn a_cleared_cell_reads_as_an_unfilled_one() {
    // A spreadsheet cell somebody emptied and one they never touched are the
    // same intention, and a realm configured with `""` for an alias is not a
    // realm. `PresetInput::text` had this behaviour; keeping it is the point.
    let cleared = PresetInput::new(vec![(
        presets::PARAM_SAML_IDP_ALIAS.to_string(),
        Value::String("   ".to_string()),
    )]);

    let saml = derived()
        .into_iter()
        .find(|doc| doc.name == "saml_sso_idp_initiated")
        .expect("the shipped presets include SAML");

    assert_eq!(saml.build(&cleared), saml.build(&PresetInput::default()));
}

#[test]
fn an_unknown_placeholder_is_left_where_somebody_will_see_it() {
    // Emptying it would produce a realm that looks configured and is not. A
    // visible `{{typo_here}}` is found by reading the file.
    let doc = AuthPresetDoc {
        name: "invented".to_string(),
        summary: String::new(),
        uses_otp: false,
        requires: vec![],
        required_parameters: vec![],
        optional_parameters: vec![],
        profile_attributes: vec![],
        parameter_defaults: Map::new(),
        patch: PatchDoc {
            patch: match json!({"displayName": "{{typo_here}} realm"}) {
                Value::Object(map) => map,
                _ => Map::new(),
            },
            ..PatchDoc::default()
        },
    };

    assert_eq!(
        doc.build(&PresetInput::default()).patch.get("displayName"),
        Some(&Value::String("{{typo_here}} realm".to_string()))
    );
}

#[test]
fn a_placeholder_in_a_key_is_substituted_too() {
    // A user-profile patch is keyed by attribute name, and an attribute name is
    // exactly the kind of thing a client renames.
    let doc = AuthPresetDoc {
        name: "invented".to_string(),
        summary: String::new(),
        uses_otp: false,
        requires: vec![],
        required_parameters: vec![],
        optional_parameters: vec!["attribute".to_string()],
        profile_attributes: vec![],
        parameter_defaults: Map::new(),
        patch: PatchDoc {
            patch: Map::new(),
            bind_authenticator_config: None,
            user_profile: match json!({"{{attribute}}": {"required": true}}) {
                Value::Object(map) => Some(map),
                _ => None,
            },
        },
    };

    let input = PresetInput::new(vec![(
        "attribute".to_string(),
        Value::String("memberId".to_string()),
    )]);

    assert!(doc
        .build(&input)
        .user_profile
        .expect("a user profile was set")
        .contains_key("memberId"));
}

/// Every object key sorted, however this build spells a JSON map.
///
/// **Without this the shipped file depends on which crates the build happens to
/// include, and that is not a hypothetical.** `serde_json::Map` is a `BTreeMap`
/// normally and an `IndexMap` when the `preserve_order` feature is on — sorted keys
/// against insertion-order keys — and cargo unifies features across a build, so
/// anything in the graph asking for it changes the bytes this test produces. In
/// this crate that is `biscuit`, a JWT dependency the `keycloak` feature pulls in:
/// the file was generated without `keycloak` and CI runs
/// `--features keycloak,default_features,…`, so CI regenerated it into insertion
/// order and reported the shipped file as out of date. Both documents were
/// semantically identical; the diff was three hundred lines of nothing.
///
/// Sorting is the canonical form rather than an arbitrary pick: it is what every
/// feature set that does *not* pull `preserve_order` already produces, and it is
/// what the committed file already was, so choosing it changed no bytes.
///
/// Comparing the parsed documents instead would also have gone green, and is worse.
/// A byte comparison is the property actually wanted — the file is `include_str!`'d
/// and ships as itself, so "exactly reproducible" is worth keeping — and dropping to
/// a semantic comparison would leave whoever next runs
/// `SEQUENT_WRITE_DEFAULT_PROFILE=1` committing a three-hundred-line no-op diff that
/// passes.
fn sorted(value: Value) -> Value {
    match value {
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .collect::<std::collections::BTreeMap<_, _>>()
                .into_iter()
                .map(|(key, value)| (key, sorted(value)))
                .collect(),
        ),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(sorted).collect())
        }
        other => other,
    }
}

/// The file in git is what the functions produce.
///
/// Pinned rather than trusted, because the file is the thing that ships and the
/// functions are the thing that is reviewed. A preset changed in Rust without
/// regenerating fails here, which is the only moment anybody would notice.
#[test]
fn default_profile_json_matches_the_shipped_presets() {
    let written = serde_json::to_string_pretty(&sorted(json!({
        "id": "default",
        "display_name": "Sequent's own",
        "defaults": {},
        "locked": [],
        "hidden": [],
        "required": [],
        "auth_presets": derived(),
    })))
    .expect("the presets serialize")
        + "\n";

    if std::env::var("SEQUENT_WRITE_DEFAULT_PROFILE").is_ok() {
        std::fs::write(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/src/election_config/presets/default_profile.json"
            ),
            &written,
        )
        .expect("the profile is writable");
    }

    assert_eq!(
        super::super::profile::DEFAULT_PROFILE_JSON,
        written,
        "default_profile.json is out of date. Regenerate with \
         SEQUENT_WRITE_DEFAULT_PROFILE=1."
    );
}

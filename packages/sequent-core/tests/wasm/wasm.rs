// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json::Value;
use wasm_bindgen::JsValue;
use wasm_bindgen_test::*;
use web_sys::js_sys::JSON;

use sequent_core::wasm::wasm::verify_ballot_signature_js;

// Configure tests to run in a browser environment
wasm_bindgen_test_configure!(run_in_browser);

// Store the large valid JSON as a constant raw string
const VALID_BALLOT_JSON: &str = r#"{"version":1,"issue_date":"13/10/2025","config":{"id":"e18630c5-ed89-495f-8a1d-f488084c64f1","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"a00cbd54-d7c4-4440-a614-261d5d8d573b","election_id":"4104d326-9e7d-48d7-b047-b6908a11c90f","num_allowed_revotes":null,"description":null,"public_key":{"public_key":"zI/lPoirqhY8EzaAZuOGO5vwmxXxqRcGn3ubK+Z0GGw","is_demo":false},"area_id":"c260a35d-0cb5-4988-ae73-2e34cc734bbe","area_presentation":{"allow_early_voting":"no_early_voting"},"contests":[{"id":"108e054a-60a3-4bb4-b72b-50fe8163a958","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"a00cbd54-d7c4-4440-a614-261d5d8d573b","election_id":"4104d326-9e7d-48d7-b047-b6908a11c90f","name":"ee1e1q1","name_i18n":{"cat":"ee1e1q1","eu":"ee1e1q1","fr":"ee1e1q1","gl":"ee1e1q1","en":"ee1e1q1","nl":"ee1e1q1","tl":"ee1e1q1","es":"ee1e1q1"},"description":null,"description_i18n":{},"alias":null,"alias_i18n":{},"max_votes":1,"min_votes":0,"winning_candidates_num":1,"voting_type":"non-preferential","counting_algorithm":"plurality-at-large","is_encrypted":true,"candidates":[{"id":"7a9a87c8-8dbb-45c4-aeac-ee21597f06f4","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"a00cbd54-d7c4-4440-a614-261d5d8d573b","election_id":"4104d326-9e7d-48d7-b047-b6908a11c90f","contest_id":"108e054a-60a3-4bb4-b72b-50fe8163a958","name":"ee1e1q1a2","name_i18n":{"nl":"ee1e1q1a2","es":"ee1e1q1a2","gl":"ee1e1q1a2","cat":"ee1e1q1a2","fr":"ee1e1q1a2","en":"ee1e1q1a2","eu":"ee1e1q1a2","tl":"ee1e1q1a2"},"description":null,"description_i18n":{},"alias":null,"alias_i18n":{},"candidate_type":null,"presentation":{"i18n":{"cat":{"name":"ee1e1q1a2"},"gl":{"name":"ee1e1q1a2"},"eu":{"name":"ee1e1q1a2"},"tl":{"name":"ee1e1q1a2"},"nl":{"name":"ee1e1q1a2"},"fr":{"name":"ee1e1q1a2"},"es":{"name":"ee1e1q1a2"},"en":{"name":"ee1e1q1a2"}},"is_explicit_invalid":null,"is_explicit_blank":null,"is_disabled":null,"is_category_list":null,"invalid_vote_position":null,"is_write_in":null,"sort_order":null,"urls":null,"subtype":null},"annotations":null},{"id":"c77e57b8-93df-4dce-a8e3-4b2126a590c5","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"a00cbd54-d7c4-4440-a614-261d5d8d573b","election_id":"4104d326-9e7d-48d7-b047-b6908a11c90f","contest_id":"108e054a-60a3-4bb4-b72b-50fe8163a958","name":"ee1e1q1a1","name_i18n":{"fr":"ee1e1q1a1","nl":"ee1e1q1a1","gl":"ee1e1q1a1","tl":"ee1e1q1a1","en":"ee1e1q1a1","es":"ee1e1q1a1","cat":"ee1e1q1a1","eu":"ee1e1q1a1"},"description":null,"description_i18n":{},"alias":null,"alias_i18n":{},"candidate_type":null,"presentation":{"i18n":{"cat":{"name":"ee1e1q1a1"},"en":{"name":"ee1e1q1a1"},"eu":{"name":"ee1e1q1a1"},"tl":{"name":"ee1e1q1a1"},"nl":{"name":"ee1e1q1a1"},"fr":{"name":"ee1e1q1a1"},"es":{"name":"ee1e1q1a1"},"gl":{"name":"ee1e1q1a1"}},"is_explicit_invalid":null,"is_explicit_blank":null,"is_disabled":null,"is_category_list":null,"invalid_vote_position":null,"is_write_in":null,"sort_order":null,"urls":null,"subtype":null},"annotations":null}],"presentation":{"i18n":{"cat":{"name":"ee1e1q1"},"tl":{"name":"ee1e1q1"},"nl":{"name":"ee1e1q1"},"en":{"name":"ee1e1q1"},"eu":{"name":"ee1e1q1"},"gl":{"name":"ee1e1q1"},"fr":{"name":"ee1e1q1"},"es":{"name":"ee1e1q1"}},"allow_writeins":null,"base32_writeins":null,"invalid_vote_policy":null,"under_vote_policy":null,"blank_vote_policy":null,"over_vote_policy":null,"pagination_policy":null,"cumulative_number_of_checkboxes":null,"shuffle_categories":null,"shuffle_category_list":null,"show_points":null,"enable_checkable_lists":null,"candidates_order":"alphabetical","candidates_selection_policy":null,"candidates_icon_checkbox_policy":null,"max_selections_per_type":null,"types_presentation":null,"sort_order":null,"columns":null},"created_at":"2025-10-13T10:27:58.987774+00:00","annotations":null}],"election_event_presentation":{"i18n":{"nl":{"name":"ee1"},"eu":{"name":"ee1"},"fr":{"name":"ee1"},"en":{"name":"ee1"},"cat":{"name":"ee1"},"tl":{"name":"ee1"},"gl":{"name":"ee1"},"es":{"name":"ee1"}},"materials":{"activated":false},"language_conf":{"enabled_language_codes":["en"],"default_language_code":"en"},"logo_url":null,"redirect_finish_url":null,"css":null,"skip_election_list":false,"show_user_profile":false,"show_cast_vote_logs":"hide-logs-tab","elections_order":"alphabetical","voting_portal_countdown_policy":{"policy":"NO_COUNTDOWN","countdown_anticipation_secs":60,"countdown_alert_anticipation_secs":180},"custom_urls":{"login":null,"enrollment":null,"saml":null},"keys_ceremony_policy":null,"contest_encryption_policy":"single-contest","decoded_ballot_inclusion_policy":"not-included","locked_down":"not-locked-down","publish_policy":null,"enrollment":null,"otp":null,"voter_signing_policy":"no-signature","weighted_voting_policy":"disabled-weighted-voting"},"election_presentation":{"i18n":{"cat":{"name":"ee1e1"},"eu":{"name":"ee1e1"},"nl":{"name":"ee1e1"},"tl":{"name":"ee1e1"},"es":{"name":"ee1e1"},"en":{"name":"ee1e1"},"fr":{"name":"ee1e1"},"gl":{"name":"ee1e1"}},"dates":null,"language_conf":{"enabled_language_codes":["en"],"default_language_code":"en"},"contests_order":null,"audit_button_cfg":null,"sort_order":null,"cast_vote_confirm":null,"cast_vote_gold_level":null,"start_screen_title_policy":null,"is_grace_priod":null,"grace_period_policy":null,"grace_period_secs":null,"init_report":null,"manual_start_voting_period":null,"voting_period_end":null,"tally":null,"initialization_report_policy":null,"security_confirmation_policy":null},"election_dates":{"first_started_at":null,"last_started_at":null,"first_paused_at":null,"last_paused_at":null,"first_stopped_at":null,"last_stopped_at":null,"scheduled_event_dates":{}},"election_event_annotations":{},"election_annotations":{},"area_annotations":null},"contests":["JAAAADEwOGUwNTRhLTYwYTMtNGJiNC1iNzJiLTUwZmU4MTYzYTk1OPLkqkr18Pbn6SRi6n4DT4Lqgh146iw4VaZ5RZCIHCJZSHAt4m40ySEtHJQq/nsD/dacGyQXzOfLPK834dUk0S4BBAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADomKxAJLuXK5gWi9S/dL+jyfy+tHNEsWysN5iJr02kDlgJmTX9FOSgYQUkEXdXcbTuDHD4Y7tt+bM8mo3K99oBSKR+1R2cNIgsbpnojCw49NZLO6WCO7lPmpuGxsvSUAakDvDV7g05LeTmAGk7k1d8dD8Dt75L09POYxHOiCI3DQ"],"ballot_hash":"07ad7361eaa62d708a1df1785f0fc3366fc6eff71e16a416fc3de42a298dbc34","voter_signing_pk":"MCowBQYDK2VwAyEAmbV8qgSr7vizQPHQF8ORkGNbgzI+C0lsnT5HMz48I64=","voter_ballot_signature":"fqr3onxfU8E2EfwdyVsGzHJ20eWykh2PyjSj4T01wIVtz85D0miHcNg0Fjg5aRAFrtLwHCsOwtcHPVb3yugcBQ=="}"#;

#[wasm_bindgen_test]
fn test_verify_success() {
    let ballot_id = JsValue::from_str(
        "07ad7361eaa62d708a1df1785f0fc3366fc6eff71e16a416fc3de42a298dbc34",
    );
    let election_id = JsValue::from_str("4104d326-9e7d-48d7-b047-b6908a11c90f");
    let auditable_multi_ballot_json = JSON::parse(VALID_BALLOT_JSON).unwrap();

    let result = verify_ballot_signature_js(
        ballot_id,
        election_id,
        auditable_multi_ballot_json,
    );

    assert!(
        result.is_ok(),
        "Verification should succeed. Error: {:?}",
        result.err()
    );
    let js_val = result.unwrap();
    let verification_result: bool =
        serde_wasm_bindgen::from_value(js_val).unwrap();
    assert_eq!(
        verification_result, true,
        "The verification result should be true"
    );
}

#[wasm_bindgen_test]
fn test_verify_fails_on_bad_signature() {
    let ballot_id = JsValue::from_str(
        "e1c33f34f847dbacb2a33c2e122d5133731f58cc03d015c6a50667dcb06cce9a",
    );
    let election_id = JsValue::from_str("9ff8a69d-fa1b-4cc8-a7f0-507b57d0196e");

    // Change to a valid signature for another ballot
    let mut ballot_value: Value =
        serde_json::from_str(VALID_BALLOT_JSON).unwrap();
    ballot_value["voter_ballot_signature"] = Value::String("pi8aqhz3a/zCoCNE8x8hASwQfH+LmDB/KzThhD3MORliVcmZAej/ldanmL00mf0pgvft+8vaSYR8TqW+LYGLDQ==".to_string());

    let auditable_multi_ballot_json =
        JSON::parse(&ballot_value.to_string()).unwrap();

    let result = verify_ballot_signature_js(
        ballot_id,
        election_id,
        auditable_multi_ballot_json,
    );

    assert!(
        result.is_err(),
        "Verification should fail due to bad signature"
    );
    let error_string = result.err().unwrap().as_string().unwrap();
    assert_eq!(error_string, "Error verifying the ballot: Failed to verify signature: ecdsa error: signature error: Verification equation was not satisfied");
}

#[wasm_bindgen_test]
fn test_fails_on_malformed_auditable_ballot_json() {
    let ballot_id = JsValue::from_str(
        "e1c33f34f847dbacb2a33c2e122d5133731f58cc03d015c6a50667dcb06cce9a",
    );
    let election_id = JsValue::from_str("9ff8a69d-fa1b-4cc8-a7f0-507b57d0196e");
    let auditable_multi_ballot_json = JsValue::from_str("{ not valid json }");

    let result = verify_ballot_signature_js(
        ballot_id,
        election_id,
        auditable_multi_ballot_json,
    );

    assert!(result.is_err(), "Should fail on auditable ballot parsing");
    let error_string = result.err().unwrap().as_string().unwrap();
    assert!(error_string.contains("Error deserializing auditable multi ballot"));
}

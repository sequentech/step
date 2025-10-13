// SPDX-FileCopyrightText: 2025 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use wasm_bindgen_test::*;
use wasm_bindgen::JsValue;
use web_sys::js_sys::JSON;
use serde_json::Value;

use sequent_core::wasm::wasm::verify_ballot_signature_js;

// Configure tests to run in a browser environment
wasm_bindgen_test_configure!(run_in_browser);

// Store the large valid JSON as a constant raw string
const VALID_BALLOT_JSON: &str = r#"{"version":1,"issue_date":"10/10/2025","config":{"id":"f851d214-80f3-4e14-8ab4-8b139a7b03b6","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"d5d4a0fc-ce8c-40d4-8abd-20112099fb5f","election_id":"9ff8a69d-fa1b-4cc8-a7f0-507b57d0196e","num_allowed_revotes":null,"description":null,"public_key":{"public_key":"HHRvlnWvqmv/KkzCvf1gk+iSyZXABXl9BSJfa1CNHiw","is_demo":false},"area_id":"644d233d-852b-484a-a2e9-acc2a8c4df5d","contests":[{"id":"b318de41-4f1f-4fd7-bdad-c15116caf43b","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"d5d4a0fc-ce8c-40d4-8abd-20112099fb5f","election_id":"9ff8a69d-fa1b-4cc8-a7f0-507b57d0196e","name":"ee1e1q1","name_i18n":{"cat":"ee1e1q1","eu":"ee1e1q1","fr":"ee1e1q1","gl":"ee1e1q1","en":"ee1e1q1","nl":"ee1e1q1","tl":"ee1e1q1","es":"ee1e1q1"},"description":null,"description_i18n":{},"alias":null,"alias_i18n":{},"max_votes":1,"min_votes":0,"winning_candidates_num":1,"voting_type":"non-preferential","counting_algorithm":"plurality-at-large","is_encrypted":true,"candidates":[{"id":"625b7459-1b5f-4bd1-adc3-6939887d8d28","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"d5d4a0fc-ce8c-40d4-8abd-20112099fb5f","election_id":"9ff8a69d-fa1b-4cc8-a7f0-507b57d0196e","contest_id":"b318de41-4f1f-4fd7-bdad-c15116caf43b","name":"ee1e1q1a1","name_i18n":{"nl":"ee1e1q1a1","es":"ee1e1q1a1","gl":"ee1e1q1a1","cat":"ee1e1q1a1","fr":"ee1e1q1a1","en":"ee1e1q1a1","eu":"ee1e1q1a1","tl":"ee1e1q1a1"},"description":null,"description_i18n":{},"alias":null,"alias_i18n":{},"candidate_type":null,"presentation":{"i18n":{"cat":{"name":"ee1e1q1a1"},"gl":{"name":"ee1e1q1a1"},"eu":{"name":"ee1e1q1a1"},"tl":{"name":"ee1e1q1a1"},"nl":{"name":"ee1e1q1a1"},"fr":{"name":"ee1e1q1a1"},"es":{"name":"ee1e1q1a1"},"en":{"name":"ee1e1q1a1"}},"is_explicit_invalid":null,"is_explicit_blank":null,"is_disabled":null,"is_category_list":null,"invalid_vote_position":null,"is_write_in":null,"sort_order":null,"urls":null,"subtype":null},"annotations":null},{"id":"dc87d1a0-7f51-4fa0-bb1e-e52e91e4a2b7","tenant_id":"90505c8a-23a9-4cdf-a26b-4e19f6a097d5","election_event_id":"d5d4a0fc-ce8c-40d4-8abd-20112099fb5f","election_id":"9ff8a69d-fa1b-4cc8-a7f0-507b57d0196e","contest_id":"b318de41-4f1f-4fd7-bdad-c15116caf43b","name":"ee1e1q1a2","name_i18n":{"fr":"ee1e1q1a2","nl":"ee1e1q1a2","gl":"ee1e1q1a2","tl":"ee1e1q1a2","en":"ee1e1q1a2","es":"ee1e1q1a2","cat":"ee1e1q1a2","eu":"ee1e1q1a2"},"description":null,"description_i18n":{},"alias":null,"alias_i18n":{},"candidate_type":null,"presentation":{"i18n":{"cat":{"name":"ee1e1q1a2"},"en":{"name":"ee1e1q1a2"},"eu":{"name":"ee1e1q1a2"},"tl":{"name":"ee1e1q1a2"},"nl":{"name":"ee1e1q1a2"},"fr":{"name":"ee1e1q1a2"},"es":{"name":"ee1e1q1a2"},"gl":{"name":"ee1e1q1a2"}},"is_explicit_invalid":null,"is_explicit_blank":null,"is_disabled":null,"is_category_list":null,"invalid_vote_position":null,"is_write_in":null,"sort_order":null,"urls":null,"subtype":null},"annotations":null}],"presentation":{"i18n":{"cat":{"name":"ee1e1q1"},"tl":{"name":"ee1e1q1"},"nl":{"name":"ee1e1q1"},"en":{"name":"ee1e1q1"},"eu":{"name":"ee1e1q1"},"gl":{"name":"ee1e1q1"},"fr":{"name":"ee1e1q1"},"es":{"name":"ee1e1q1"}},"allow_writeins":null,"base32_writeins":null,"invalid_vote_policy":null,"under_vote_policy":null,"blank_vote_policy":null,"over_vote_policy":null,"pagination_policy":null,"cumulative_number_of_checkboxes":null,"shuffle_categories":null,"shuffle_category_list":null,"show_points":null,"enable_checkable_lists":null,"candidates_order":"alphabetical","candidates_selection_policy":null,"candidates_icon_checkbox_policy":null,"max_selections_per_type":null,"types_presentation":null,"sort_order":null,"columns":null},"created_at":"2025-09-29T19:48:04.926192+00:00","annotations":null}],"election_event_presentation":{"i18n":{"nl":{"name":"ee1"},"eu":{"name":"ee1"},"fr":{"name":"ee1"},"en":{"name":"ee1","alias":"ee1_TEST"},"cat":{"name":"ee1"},"tl":{"name":"ee1"},"gl":{"name":"ee1"},"es":{"name":"ee1"}},"materials":{"activated":false},"language_conf":{"enabled_language_codes":["en"],"default_language_code":"en"},"logo_url":null,"redirect_finish_url":null,"css":null,"skip_election_list":false,"show_user_profile":false,"show_cast_vote_logs":"hide-logs-tab","elections_order":"alphabetical","voting_portal_countdown_policy":{"policy":"NO_COUNTDOWN","countdown_anticipation_secs":60,"countdown_alert_anticipation_secs":180},"custom_urls":{"login":null,"enrollment":null,"saml":null},"keys_ceremony_policy":null,"contest_encryption_policy":"single-contest","decoded_ballot_inclusion_policy":"not-included","locked_down":"not-locked-down","publish_policy":null,"enrollment":null,"otp":null,"voter_signing_policy":"no-signature"},"election_presentation":{"i18n":{"cat":{"name":"ee1e1"},"eu":{"name":"ee1e1"},"nl":{"name":"ee1e1"},"tl":{"name":"ee1e1"},"es":{"name":"ee1e1"},"en":{"name":"ee1e1"},"fr":{"name":"ee1e1"},"gl":{"name":"ee1e1"}},"dates":null,"language_conf":{"enabled_language_codes":["en"],"default_language_code":"en"},"contests_order":null,"audit_button_cfg":null,"sort_order":null,"cast_vote_confirm":null,"cast_vote_gold_level":null,"start_screen_title_policy":null,"is_grace_priod":null,"grace_period_policy":null,"grace_period_secs":null,"init_report":null,"manual_start_voting_period":null,"voting_period_end":null,"tally":null,"initialization_report_policy":null,"security_confirmation_policy":null},"election_dates":{"first_started_at":null,"last_started_at":null,"first_paused_at":null,"last_paused_at":null,"first_stopped_at":null,"last_stopped_at":null,"scheduled_event_dates":{}},"election_event_annotations":{},"election_annotations":{}},"contests":["JAAAAGIzMThkZTQxLTRmMWYtNGZkNy1iZGFkLWMxNTExNmNhZjQzYtzImPbozVVqJ2+tEyzZ1WmNzM/Eh4x+mMJ6eeQ45alsslEctW1QyxUeq+M4bkIOpMe0zcoUApQppqnYT8WZNW4BAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADuevz5fVoHYx2YI+SvyrD0l8jvH9UJ7yTyQ2jS+ow5BsKiqqGJLHeZhp455LlDFG9tWLPxgbTLjnq76CSuQrM6Qjo5Of26WvWTJ9OhO6fVNIlFsk9r1KJK2baVhV9j5QUP1mCm0liylYKEiL2e2kIz8gukE4a4upWFpajsNPDsCQ"],"ballot_hash":"e1c33f34f847dbacb2a33c2e122d5133731f58cc03d015c6a50667dcb06cce9a","voter_signing_pk":"MCowBQYDK2VwAyEARKIAVaMx/5sKf2fzn3mavcoqpSuLny4Br84pvQ0yn/8=","voter_ballot_signature":"0oPB+IHpK91nQIueecuBDzQnuUrBPL0uSvKrL1H6qDDhmO/GWLsrmGAPOzuNvMkP7Q+VPFSEm2XJARI+w3lqCA=="}"#;

#[wasm_bindgen_test]
fn test_verify_success() {
    let ballot_id = JsValue::from_str(
        "e1c33f34f847dbacb2a33c2e122d5133731f58cc03d015c6a50667dcb06cce9a",
    );
    let election_id = JsValue::from_str("9ff8a69d-fa1b-4cc8-a7f0-507b57d0196e");
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
    assert_eq!(error_string, "Error signing the ballot: Invalid signature");
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

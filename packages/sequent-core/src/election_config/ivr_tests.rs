// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Tests for [`super`].

use super::*;

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn prompts(entries: &[(&str, &str, &str)]) -> Prompts {
    let mut all = Prompts::new();
    for (language, key, text) in entries {
        all.entry((*language).to_string())
            .or_default()
            .insert((*key).to_string(), (*text).to_string());
    }
    all
}

/// The Election Architect's default flow, as the wizard writes it.
const FLOW: &str = r#"{"flow": [
    {"phase": "announcement", "name": "welcome", "prompt_key": "greeting"},
    {"phase": "blacklist_check"},
    {"phase": "language_select"},
    {"phase": "auth"},
    {"phase": "eligibility_check"},
    {"phase": "announcement", "name": "declaration", "prompt_key": "declaration_text", "accept_key": "2"},
    {"phase": "announcement", "name": "pre_voting_statement", "prompt_key": "pre_voting_statement"},
    {"phase": "ballot_loop", "receipt_format": "phonetic_hex_4"},
    {"phase": "goodbye"}
]}"#;

const ENGLISH_ONLY: &str = r#"{"en": {
    "greeting": "Welcome.",
    "declaration_text": "Press {continue_input}.",
    "pre_voting_statement": "Your vote is cast once submitted.",
    "system_error": "Something has gone wrong."
}}"#;

#[test]
fn only_announcements_require_a_prompt_and_each_is_named_once() {
    let required = required_prompt_keys([
        ("announcement", "greeting"),
        ("language_select", ""),
        ("announcement", "greeting"),
        ("auth", "ignored"),
        ("announcement", "  "),
        ("announcement", "declaration_text"),
    ]);
    assert_eq!(required, strings(&["greeting", "declaration_text"]));
}

#[test]
fn spanish_with_only_english_words_is_missing_every_announcement() {
    // The Call Emulator's bug, stated as data: the sample's prompts are English,
    // Spanish is ticked, and the call refuses to start.
    let missing = missing_in_annotations(
        Some(FLOW),
        Some(ENGLISH_ONLY),
        &strings(&["en", "es"]),
    );
    assert_eq!(
        missing,
        vec![MissingPrompts {
            language: "es".to_string(),
            prompt_keys: strings(&[
                "greeting",
                "declaration_text",
                "pre_voting_statement"
            ]),
        }]
    );
}

#[test]
fn english_alone_is_complete() {
    assert!(missing_in_annotations(
        Some(FLOW),
        Some(ENGLISH_ONLY),
        &strings(&["en"])
    )
    .is_empty());
}

#[test]
fn french_is_spoken_too() {
    let missing = missing_in_annotations(
        Some(FLOW),
        Some(ENGLISH_ONLY),
        &strings(&["fr", "en"]),
    );
    assert_eq!(missing.len(), 1);
    assert_eq!(missing[0].language, "fr");
}

#[test]
fn a_language_the_call_cannot_speak_asks_for_nothing() {
    // The call skips Tagalog, Basque and the rest, so their absence cannot stop it.
    assert!(missing_in_annotations(
        Some(FLOW),
        Some(ENGLISH_ONLY),
        &strings(&["en", "tl", "eu", "cat", "gl", "nl"]),
    )
    .is_empty());
    assert_eq!(
        unspoken_languages(&strings(&["en", "tl", "es", "eu", "tl"])),
        strings(&["tl", "eu"])
    );
}

#[test]
fn a_three_letter_code_is_not_spoken_because_the_call_reads_it_as_unknown() {
    // ivr-core parses languages with strum's `en`/`fr`/`es` only; `spa` is its
    // `Language::Unknown`, skipped and never asked for prompts. Normalizing it here
    // would refuse events the call accepts. The plan reports it as unspoken instead.
    assert!(missing_in_annotations(
        Some(FLOW),
        Some(ENGLISH_ONLY),
        &strings(&["en", "spa"]),
    )
    .is_empty());
    assert_eq!(
        unspoken_languages(&strings(&["eng", "spa", "es"])),
        strings(&["eng", "spa"])
    );
}

#[test]
fn no_languages_means_english() {
    let missing =
        missing_prompts(&strings(&["greeting"]), &Prompts::new(), &Vec::new());
    assert_eq!(missing[0].language, "en");
    assert_eq!(spoken_languages(&[]), strings(&["en"]));
}

#[test]
fn blank_words_are_missing_words() {
    let missing = missing_prompts(
        &strings(&["greeting", "declaration_text"]),
        &prompts(&[
            ("es", "greeting", "   "),
            ("es", "declaration_text", "Pulse {continue_input}."),
        ]),
        &strings(&["es"]),
    );
    assert_eq!(
        missing,
        vec![MissingPrompts {
            language: "es".to_string(),
            prompt_keys: strings(&["greeting"]),
        }]
    );
}

#[test]
fn no_flow_is_no_call_and_unreadable_prompts_are_no_prompts() {
    assert!(missing_in_annotations(None, None, &strings(&["es"])).is_empty());
    assert!(
        missing_in_annotations(Some("not json"), None, &strings(&["es"]))
            .is_empty()
    );
    let missing = missing_in_annotations(
        Some(FLOW),
        Some("{not json"),
        &strings(&["en"]),
    );
    assert_eq!(missing[0].prompt_keys.len(), 3);
}

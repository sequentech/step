// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a telephone call has to be able to say before it can be placed.
//!
//! The IVR Lambda (`beyond/packages/ivr-core`) checks this at the start of every
//! call: each prompt an `announcement` phase names must have words in every
//! language the event offers that the call can speak. When one is missing it
//! refuses the call with `MissingRequiredPrompts` and the caller hears only the
//! system error. So an event can import cleanly, publish cleanly and then fail on
//! every call, which is how Spanish broke the Election Architect's Call Emulator:
//! ticking Spanish added a language and no words for it.
//!
//! This module is that rule, stated once, so the wizard, the bundle validator and
//! the importer refuse what the call would refuse. `ivr-core` carries a contract
//! test against [`missing_in_annotations`], so the two cannot drift apart without
//! a red test.

use super::problem::{Code, Problem};
use std::collections::BTreeMap;

/// The phase that plays one of the event's own prompts.
pub const ANNOUNCEMENT: &str = "announcement";

/// The languages the call can speak: `ivr-core`'s `Language`.
///
/// An event can offer more (Tagalog, Basque, Catalan...), and the call skips
/// those: it neither asks for their prompts nor offers them to a caller. That is
/// why they are left out of [`missing_prompts`] and reported by
/// [`unspoken_languages`] instead.
pub const SPOKEN_LANGUAGES: &[&str] = &["en", "fr", "es"];

/// The event's prompts: `{language: {prompt_key: text}}`, the `ivr:prompts`
/// annotation's shape.
pub type Prompts = BTreeMap<String, BTreeMap<String, String>>;

/// One language whose words for some required prompts are missing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingPrompts {
    /// The language, as the event names it (`es`).
    pub language: String,
    /// The prompt keys with no words in it, in the order the flow names them.
    pub prompt_keys: Vec<String>,
}

impl MissingPrompts {
    /// The problem both validators raise, pointing at `path`.
    ///
    /// Built here, once, because the plan and the bundle raise the same complaint
    /// in two vocabularies and one catalogue entry translates both. `count` is
    /// the number of prompts, which the catalogue picks its plural form by: one
    /// greeting "has" no words, two prompts "have" none.
    pub fn problem(&self, path: &str) -> Problem {
        let prompts = self.prompt_keys.join(", ");
        let (has, does) = if self.prompt_keys.len() == 1 {
            ("has", "it does")
        } else {
            ("have", "they do")
        };
        Problem::error(
            Code::MissingField,
            path,
            format!(
                "{prompts} {has} no words in '{}', and the telephone system \
                 refuses every call until {does}",
                self.language
            ),
        )
        .id("ivr.missing-prompts")
        .detail("language", &self.language)
        .detail("prompts", prompts)
        .detail("count", self.prompt_keys.len())
    }
}

/// The prompts a flow cannot run without, in the order it plays them.
///
/// Takes `(phase, prompt_key)` pairs so the plan's typed flow and the event's JSON
/// one can both be asked. Only `announcement` phases require anything: every other
/// phase speaks the platform's built-in prompts, which exist in every spoken
/// language.
pub fn required_prompt_keys<'a>(
    flow: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Vec<String> {
    let mut keys: Vec<String> = Vec::new();
    for (phase, prompt_key) in flow {
        let key = prompt_key.trim();
        if phase == ANNOUNCEMENT
            && !key.is_empty()
            && !keys.iter().any(|each| each == key)
        {
            keys.push(key.to_string());
        }
    }
    keys
}

/// The languages the call will actually speak, of those the event offers.
///
/// An event offering none is spoken in English, as `ivr-core` does.
pub fn spoken_languages(languages: &[String]) -> Vec<String> {
    if languages.is_empty() {
        return vec!["en".to_string()];
    }
    let mut spoken: Vec<String> = Vec::new();
    for language in languages {
        if SPOKEN_LANGUAGES.contains(&language.as_str())
            && !spoken.contains(language)
        {
            spoken.push(language.clone());
        }
    }
    spoken
}

/// The languages the event offers that no caller will hear.
pub fn unspoken_languages(languages: &[String]) -> Vec<String> {
    let mut unspoken: Vec<String> = Vec::new();
    for language in languages {
        if !SPOKEN_LANGUAGES.contains(&language.as_str())
            && !unspoken.contains(language)
        {
            unspoken.push(language.clone());
        }
    }
    unspoken
}

/// Every spoken language with a required prompt that has no words in it.
///
/// Blank counts as missing: `ivr-core` trims each prompt and drops the empty
/// ones before it checks.
pub fn missing_prompts(
    required: &[String],
    prompts: &Prompts,
    languages: &[String],
) -> Vec<MissingPrompts> {
    spoken_languages(languages)
        .into_iter()
        .filter_map(|language| {
            let said = prompts.get(&language);
            let prompt_keys: Vec<String> = required
                .iter()
                .filter(|key| {
                    said.and_then(|words| words.get(key.as_str()))
                        .is_none_or(|text| text.trim().is_empty())
                })
                .cloned()
                .collect();
            (!prompt_keys.is_empty()).then_some(MissingPrompts {
                language,
                prompt_keys,
            })
        })
        .collect()
}

/// The same question asked of an event, as the call reads it.
///
/// `ivr_config` and `ivr_prompts` are the `ivr:config` and `ivr:prompts`
/// annotations, both JSON strings; `languages` is the event's
/// `enabled_language_codes`. No flow means no call and so nothing missing; a
/// prompts annotation that does not parse is read as no prompts, as the Lambda
/// reads it.
pub fn missing_in_annotations(
    ivr_config: Option<&str>,
    ivr_prompts: Option<&str>,
    languages: &[String],
) -> Vec<MissingPrompts> {
    let Some(config) = ivr_config
        .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
    else {
        return Vec::new();
    };
    let flow = config
        .get("flow")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let required = required_prompt_keys(flow.iter().map(|phase| {
        let field = |name: &str| {
            phase
                .get(name)
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
        };
        (field("phase"), field("prompt_key"))
    }));
    let prompts = ivr_prompts
        .and_then(|text| serde_json::from_str::<Prompts>(text).ok())
        .unwrap_or_default();
    missing_prompts(&required, &prompts, languages)
}

#[cfg(test)]
#[path = "ivr_tests.rs"]
mod ivr_tests;

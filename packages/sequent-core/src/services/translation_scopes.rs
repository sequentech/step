// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Translation override scopes, the Rust port of ui-core's
//! `services/translationScopes.ts`.
//!
//! The Localization tabs store an override as `scope:key` (`global:`,
//! `votingPortal:`, `templates:`, ...). Keys stored before scopes existed
//! have no prefix ("legacy" keys). A reader keeps only the keys its scope
//! sees, with a fixed precedence: its own scope replaces legacy keys, and
//! both replace `global:` keys. Reports and notifications read the
//! `templates` scope with legacy keys included, so their precedence is
//! `templates:` > unprefixed > `global:`.

use std::collections::HashMap;

/// A translation override scope, as stored before the `:` of a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslationScope {
    Global,
    VotingPortal,
    BallotVerifier,
    ResultsPortal,
    AdminPortal,
    /// Reports and notifications (templates rendered by windmill).
    Templates,
}

impl TranslationScope {
    pub const ALL: [TranslationScope; 6] = [
        TranslationScope::Global,
        TranslationScope::VotingPortal,
        TranslationScope::BallotVerifier,
        TranslationScope::ResultsPortal,
        TranslationScope::AdminPortal,
        TranslationScope::Templates,
    ];

    /// The prefix stored before the `:`.
    pub fn as_str(&self) -> &'static str {
        match self {
            TranslationScope::Global => "global",
            TranslationScope::VotingPortal => "votingPortal",
            TranslationScope::BallotVerifier => "ballotVerifier",
            TranslationScope::ResultsPortal => "resultsPortal",
            TranslationScope::AdminPortal => "adminPortal",
            TranslationScope::Templates => "templates",
        }
    }

    pub fn from_prefix(prefix: &str) -> Option<TranslationScope> {
        TranslationScope::ALL
            .into_iter()
            .find(|scope| scope.as_str() == prefix)
    }
}

/// Splits a stored override key into its scope and key. Only a known scope
/// prefix is split off: an unknown prefix stays part of a legacy key, so a
/// typo cannot silently target another portal.
pub fn parse_translation_override_key(
    stored_key: &str,
) -> (Option<TranslationScope>, &str) {
    match stored_key.split_once(':') {
        Some((prefix, key)) => match TranslationScope::from_prefix(prefix) {
            Some(scope) => (Some(scope), key),
            None => (None, stored_key),
        },
        None => (None, stored_key),
    }
}

/// The overrides of one language that `scope` sees, keyed without their
/// prefix. Legacy (unprefixed) keys are included only when `legacy_scope`
/// is `scope`, as in ui-core. Empty values (`None`) are skipped.
pub fn filter_translation_overrides(
    translations: &HashMap<String, Option<String>>,
    scope: TranslationScope,
    legacy_scope: Option<TranslationScope>,
) -> HashMap<String, String> {
    let mut selected: HashMap<String, String> = HashMap::new();
    let mut apply = |entry_scope: Option<TranslationScope>| {
        for (stored_key, value) in translations {
            let Some(value) = value else {
                continue;
            };
            let (parsed_scope, key) =
                parse_translation_override_key(stored_key);
            if parsed_scope != entry_scope || key.is_empty() {
                continue;
            }
            selected.insert(key.to_string(), value.clone());
        }
    };

    apply(Some(TranslationScope::Global));
    if legacy_scope == Some(scope) {
        apply(None);
    }
    if scope != TranslationScope::Global {
        apply(Some(scope));
    }
    selected
}

/// The overrides of one language that reports and notifications see:
/// `templates:` > unprefixed > `global:`.
pub fn template_translations(
    translations: &HashMap<String, Option<String>>,
) -> HashMap<String, String> {
    filter_translation_overrides(
        translations,
        TranslationScope::Templates,
        Some(TranslationScope::Templates),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overrides(entries: &[(&str, &str)]) -> HashMap<String, Option<String>> {
        entries
            .iter()
            .map(|(key, value)| (key.to_string(), Some(value.to_string())))
            .collect()
    }

    #[test]
    fn templates_scope_beats_unprefixed_which_beats_global() {
        let translations = overrides(&[
            ("global:timezones.abbr.Asia/Manila", "global"),
            ("timezones.abbr.Asia/Manila", "legacy"),
            ("templates:timezones.abbr.Asia/Manila", "templates"),
            ("global:only.global", "g"),
            ("only.legacy", "l"),
            ("global:legacy.and.global", "g"),
            ("legacy.and.global", "l"),
        ]);

        let selected = template_translations(&translations);

        assert_eq!(selected["timezones.abbr.Asia/Manila"], "templates");
        assert_eq!(selected["only.global"], "g");
        assert_eq!(selected["only.legacy"], "l");
        assert_eq!(selected["legacy.and.global"], "l");
    }

    #[test]
    fn templates_ignore_other_portals_scopes() {
        let translations = overrides(&[
            ("votingPortal:timezones.name.Asia/Dubai", "Dubai time"),
            ("adminPortal:timezones.abbr.Asia/Dubai", "GST"),
            ("global:timezones.abbr.Asia/Manila", "PHT"),
        ]);

        let selected = template_translations(&translations);

        assert_eq!(selected.len(), 1);
        assert_eq!(selected["timezones.abbr.Asia/Manila"], "PHT");
    }

    #[test]
    fn unknown_prefixes_stay_legacy_keys() {
        let translations = overrides(&[("votingportal:name", "typo")]);

        let selected = template_translations(&translations);

        assert_eq!(selected["votingportal:name"], "typo");
    }

    #[test]
    fn portals_see_legacy_keys_only_when_they_own_them() {
        let translations = overrides(&[
            ("name", "legacy"),
            ("global:name", "global"),
            ("templates:name", "templates"),
        ]);

        let voting = filter_translation_overrides(
            &translations,
            TranslationScope::VotingPortal,
            Some(TranslationScope::VotingPortal),
        );
        let results = filter_translation_overrides(
            &translations,
            TranslationScope::ResultsPortal,
            Some(TranslationScope::VotingPortal),
        );

        assert_eq!(voting["name"], "legacy");
        assert_eq!(results["name"], "global");
    }

    #[test]
    fn empty_values_are_skipped() {
        let mut translations = overrides(&[("global:name", "global")]);
        translations.insert("templates:name".to_string(), None);

        let selected = template_translations(&translations);

        assert_eq!(selected["name"], "global");
    }

    #[test]
    fn parse_keeps_slashes_and_colons_after_the_scope() {
        assert_eq!(
            parse_translation_override_key(
                "templates:timezones.abbr.Asia/Manila"
            ),
            (
                Some(TranslationScope::Templates),
                "timezones.abbr.Asia/Manila"
            )
        );
        assert_eq!(
            parse_translation_override_key("global:a:b"),
            (Some(TranslationScope::Global), "a:b")
        );
        assert_eq!(parse_translation_override_key("plain"), (None, "plain"));
    }
}

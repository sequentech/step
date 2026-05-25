// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    ballot::{
        Contest, ContestEncryptionPolicy, ContestPresentation,
        DecodedBallotsInclusionPolicy, DelegatedVotingPolicy,
        ElectionEventPresentation, ElectionPresentation, I18nContent,
        LanguageDetectionPolicy,
    },
    serialization::deserialize_with_path::deserialize_value,
    types::hasura::core::{Election, ElectionEvent},
};

/// Default language code to fall back to if no translation is found for a given language.
pub const DEFAULT_LANG: &str = "en";

/// Parses a presentation `Value` into type P.
///
/// Returns None if the presentation is None or deserialization fails.
fn parse_presentation<P>(presentation: Option<&serde_json::Value>) -> Option<P>
where
    P: for<'de> serde::Deserialize<'de>,
{
    let val = presentation?;
    deserialize_value::<P>(val.clone()).ok()
}

/// Generic i18n getter for nested shape: `I18nContent<I18nContent<Option<String>>>`.
/// Reads `field` in this order: `language` -> `DEFAULT_LANG`.
fn i18n_field(
    i18n: Option<&I18nContent<I18nContent<Option<String>>>>,
    language: &str,
    field: &'static str,
) -> Option<String> {
    let i18n = i18n.as_ref()?;

    // Try requested language first, then default language.
    i18n.get(language)
        .and_then(|m| m.get(field))
        .cloned()
        .flatten()
        .or_else(|| {
            i18n.get(DEFAULT_LANG)
                .and_then(|m| m.get(field))
                .cloned()
                .flatten()
        })
}

/// Trait for a name of an entity.
pub trait Name {
    /// Get the name of an entity in the specified language.
    fn get_name(&self, language: &str) -> String;
}

/// Trait for an alias of an entity.
pub trait Alias {
    /// Get the alias of an entity in the specified language, falling back to a name if not available.
    fn get_alias(&self, language: &str) -> Option<String>;
}

/* ---------------------- ElectionEvent ---------------------- */

impl ElectionEvent {
    #[must_use]
    /// Get the default language from presentation for the election event,
    ///  falling back to "en" if not specified.
    pub fn get_default_language(&self) -> String {
        parse_presentation::<ElectionEventPresentation>(
            self.presentation.as_ref(),
        )
        .and_then(|p| p.language_conf)
        .and_then(|lc| lc.default_language_code)
        .unwrap_or_else(|| DEFAULT_LANG.into())
    }

    #[must_use]
    /// Get the contest encryption policy from presentation for the election event,
    ///  falling back to the default policy if not specified.
    pub fn get_contest_encryption_policy(&self) -> ContestEncryptionPolicy {
        parse_presentation::<ElectionEventPresentation>(
            self.presentation.as_ref(),
        )
        .and_then(|p| p.contest_encryption_policy)
        .unwrap_or_default()
    }

    #[must_use]
    /// Get the decoded ballots inclusion policy from presentation for the election event,
    ///  falling back to the default policy if not specified.
    pub fn get_decoded_ballots_inclusion_policy(
        &self,
    ) -> DecodedBallotsInclusionPolicy {
        parse_presentation::<ElectionEventPresentation>(
            self.presentation.as_ref(),
        )
        .and_then(|p| p.decoded_ballot_inclusion_policy)
        .unwrap_or_default()
    }

    #[must_use]
    /// Get the delegated voting policy from presentation for the election event,
    ///  falling back to the default policy if not specified.
    pub fn get_delegated_voting_policy(&self) -> DelegatedVotingPolicy {
        parse_presentation::<ElectionEventPresentation>(
            self.presentation.as_ref(),
        )
        .and_then(|p| p.delegated_voting_policy)
        .unwrap_or_default()
    }

    #[must_use]
    /// Get the language detection policy from presentation for the election event,
    pub fn get_language_detection_policy(&self) -> LanguageDetectionPolicy {
        parse_presentation::<ElectionEventPresentation>(
            self.presentation.as_ref(),
        )
        .and_then(|p| p.language_conf)
        .and_then(|c| c.language_detection_policy)
        .unwrap_or_default()
    }
}

impl Name for ElectionEvent {
    fn get_name(&self, language: &str) -> String {
        parse_presentation::<ElectionEventPresentation>(
            self.presentation.as_ref(),
        )
        .and_then(|p| i18n_field(p.i18n.as_ref(), language, "name"))
        .unwrap_or_else(|| "-".into())
    }
}

/* ------------------------- Election ------------------------- */

impl Election {
    #[must_use]
    /// Get the default language from presentation for the election,
    ///  falling back to "en" if not specified.
    pub fn get_default_language(&self) -> String {
        parse_presentation::<ElectionPresentation>(self.presentation.as_ref())
            .and_then(|p| p.language_conf)
            .and_then(|lc| lc.default_language_code)
            .unwrap_or_else(|| DEFAULT_LANG.into())
    }
}

impl Name for Election {
    fn get_name(&self, language: &str) -> String {
        parse_presentation::<ElectionPresentation>(self.presentation.as_ref())
            .and_then(|p| i18n_field(p.i18n.as_ref(), language, "name"))
            .unwrap_or_else(|| "-".into())
    }
}

impl Alias for Election {
    fn get_alias(&self, language: &str) -> Option<String> {
        let base = Some(self.get_name(language));

        parse_presentation::<ElectionPresentation>(self.presentation.as_ref())
            .and_then(|p| i18n_field(p.i18n.as_ref(), language, "alias"))
            .or(base)
    }
}

/* ===================== Contest ===================== */

impl Name for Contest {
    fn get_name(&self, language: &str) -> String {
        let alias = self.alias_i18n.clone().and_then(|alias_i18n| {
            alias_i18n
                .get(language)
                .cloned()
                .or(alias_i18n.get(DEFAULT_LANG).cloned())
                .or(Some(self.alias.clone()))
                .flatten()
        });
        let name = self.name_i18n.clone().and_then(|name_i18n| {
            name_i18n
                .get(language)
                .cloned()
                .or(name_i18n.get(DEFAULT_LANG).cloned())
                .or(Some(self.name.clone()))
                .flatten()
        });

        alias.or(name).unwrap_or("-".into())
    }
}

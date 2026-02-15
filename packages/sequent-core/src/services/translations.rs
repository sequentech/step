// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    ballot::{
        Contest, ContestEncryptionPolicy, ContestPresentation,
        DecodedBallotsInclusionPolicy, DelegatedVotingPolicy,
        ElectionEventPresentation, ElectionPresentation, I18nContent,
    },
    serialization::deserialize_with_path::deserialize_value,
    types::hasura::core::{Election, ElectionEvent},
};

pub const DEFAULT_LANG: &str = "en";

fn parse_presentation<P>(presentation: &Option<serde_json::Value>) -> Option<P>
where
    P: for<'de> serde::Deserialize<'de>,
{
    let val = presentation.as_ref()?;
    deserialize_value::<P>(val.clone()).ok()
}

/// Generic i18n getter for nested shape: I18nContent<I18nContent<Option<String>>>
/// Reads `field` in this order: `language` -> DEFAULT_LANG.
fn i18n_field(
    i18n: &Option<I18nContent<I18nContent<Option<String>>>>,
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

pub trait Name {
    fn get_name(&self, language: &str) -> String;
}

pub trait Alias {
    fn get_alias(&self, language: &str) -> Option<String>;
}

/* ---------------------- ElectionEvent ---------------------- */

impl ElectionEvent {
    pub fn get_default_language(&self) -> String {
        parse_presentation::<ElectionEventPresentation>(&self.presentation)
            .and_then(|p| p.language_conf)
            .and_then(|lc| lc.default_language_code)
            .unwrap_or_else(|| DEFAULT_LANG.into())
    }

    pub fn get_contest_encryption_policy(&self) -> ContestEncryptionPolicy {
        parse_presentation::<ElectionEventPresentation>(&self.presentation)
            .and_then(|p| p.contest_encryption_policy)
            .unwrap_or_default()
    }

    pub fn get_decoded_ballots_inclusion_policy(
        &self,
    ) -> DecodedBallotsInclusionPolicy {
        parse_presentation::<ElectionEventPresentation>(&self.presentation)
            .and_then(|p| p.decoded_ballot_inclusion_policy)
            .unwrap_or_default()
    }

    pub fn get_delegated_voting_policy(&self) -> DelegatedVotingPolicy {
        parse_presentation::<ElectionEventPresentation>(&self.presentation)
            .and_then(|p| p.delegated_voting_policy)
            .unwrap_or_default()
    }
}

impl Name for ElectionEvent {
    fn get_name(&self, language: &str) -> String {
        parse_presentation::<ElectionEventPresentation>(&self.presentation)
            .and_then(|p| i18n_field(&p.i18n, language, "name"))
            .unwrap_or_else(|| "-".into())
    }
}

/* ------------------------- Election ------------------------- */

impl Election {
    pub fn get_default_language(&self) -> String {
        parse_presentation::<ElectionPresentation>(&self.presentation)
            .and_then(|p| p.language_conf)
            .and_then(|lc| lc.default_language_code)
            .unwrap_or_else(|| DEFAULT_LANG.into())
    }
}

impl Name for Election {
    fn get_name(&self, language: &str) -> String {
        parse_presentation::<ElectionPresentation>(&self.presentation)
            .and_then(|p| i18n_field(&p.i18n, language, "name"))
            .unwrap_or_else(|| "-".into())
    }
}

impl Alias for Election {
    fn get_alias(&self, language: &str) -> Option<String> {
        // fallback: name (same behavior you had)
        let base = Some(self.get_name(language));

        parse_presentation::<ElectionPresentation>(&self.presentation)
            .and_then(|p| i18n_field(&p.i18n, language, "alias"))
            .or(base)
    }
}

/* ===================== Contest ===================== */

impl Name for Contest {
    fn get_name(&self, language: &str) -> String {
        let alias = self
            .alias_i18n
            .clone()
            .map(|alias_i18n| {
                alias_i18n
                    .get(language)
                    .cloned()
                    .or(alias_i18n.get(DEFAULT_LANG).cloned())
                    .or(Some(self.alias.clone()))
                    .flatten()
            })
            .flatten();
        let name = self
            .name_i18n
            .clone()
            .map(|name_i18n| {
                name_i18n
                    .get(language)
                    .cloned()
                    .or(name_i18n.get(DEFAULT_LANG).cloned())
                    .or(Some(self.name.clone()))
                    .flatten()
            })
            .flatten();

        alias.or(name).unwrap_or("-".into())
    }
}

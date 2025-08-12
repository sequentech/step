// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{
    ballot::{
        Contest, ContestEncryptionPolicy, ContestPresentation,
        DecodedBallotsInclusionPolicy, ElectionEventPresentation,
        ElectionPresentation, I18nContent,
    },
    serialization::deserialize_with_path::deserialize_value,
    types::hasura::core::{Election, ElectionEvent},
};

pub const DEFAULT_LANG: &str = "en";

impl ElectionEvent {
    /// Get the default language at Election Event level that´s configurable on
    /// the Admin portal
    pub fn get_default_language(&self) -> String {
        let Some(presentation_val) = self.presentation.clone() else {
            return DEFAULT_LANG.into();
        };
        let Ok(presentation) =
            deserialize_value::<ElectionEventPresentation>(presentation_val)
        else {
            return DEFAULT_LANG.into();
        };
        let language_conf = presentation.language_conf.unwrap_or_default();
        let lang = language_conf
            .default_language_code
            .unwrap_or(DEFAULT_LANG.into());
        lang
    }

    pub fn get_contest_encryption_policy(&self) -> ContestEncryptionPolicy {
        let Some(presentation_val) = self.presentation.clone() else {
            return ContestEncryptionPolicy::default();
        };
        let Ok(presentation) =
            deserialize_value::<ElectionEventPresentation>(presentation_val)
        else {
            return ContestEncryptionPolicy::default();
        };
        presentation.contest_encryption_policy.unwrap_or_default()
    }

    pub fn get_decoded_ballots_inclusion_policy(
        &self,
    ) -> DecodedBallotsInclusionPolicy {
        let Some(presentation_val) = self.presentation.clone() else {
            return DecodedBallotsInclusionPolicy::default();
        };
        let Ok(presentation) =
            deserialize_value::<ElectionEventPresentation>(presentation_val)
        else {
            return DecodedBallotsInclusionPolicy::default();
        };
        presentation
            .decoded_ballot_inclusion_policy
            .unwrap_or_default()
    }
}

pub trait Name {
    fn get_name(&self, default_language: &str) -> String;
}

fn get_name_from_i18n(
    i18n_ref: &Option<I18nContent<I18nContent<Option<String>>>>,
    language: &str,
) -> Option<String> {
    let Some(i18n) = i18n_ref.clone() else {
        return None;
    };

    let lang_name = if let Some(lang_i18n) = i18n.get(language) {
        let alias = lang_i18n.get("alias").cloned().flatten();
        let name = lang_i18n.get("name").cloned().flatten();
        alias.or(name)
    } else {
        None
    };
    let default_lang_name = if let Some(def_lang_i18n) = i18n.get(DEFAULT_LANG)
    {
        let alias = def_lang_i18n.get("alias").cloned().flatten();
        let name = def_lang_i18n.get("name").cloned().flatten();
        alias.or(name)
    } else {
        None
    };
    lang_name.or(default_lang_name)
}

impl Name for Election {
    fn get_name(&self, language: &str) -> String {
        let base_name = self.name.clone();
        let Some(presentation_val) = self.presentation.clone() else {
            return base_name;
        };
        let Ok(presentation) =
            deserialize_value::<ElectionPresentation>(presentation_val)
        else {
            return base_name;
        };
        get_name_from_i18n(&presentation.i18n, language).unwrap_or(base_name)
    }
}

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

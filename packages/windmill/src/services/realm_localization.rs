// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Publish-time copy of the event's timezone and enrollment texts into the
//! event realm's localization (VOTE-LIFECYCLE §4), so the enrollment pages
//! (Keycloak message keys) follow the overrides made in the event's
//! Localization tab.
//!
//! - Source: `presentation.i18n` keys in the `global:` and `votingPortal:`
//!   scopes under `timezones.` and `enrollment.`; `votingPortal:` wins.
//! - The realm keys under those prefixes belong to this copy: a key no longer
//!   overridden is removed, so the theme default shows again.
//! - Keycloak runs every text through Java's MessageFormat, so apostrophes
//!   are doubled in every copied text (admins write them plain, as in every
//!   other Localization override). `timezones.*` texts are written for
//!   i18next: `{{dateTime}}` and `{{zoneName}}` become `{0}` and `{1}`.
//!   `enrollment.*` texts take Keycloak's `{0}` arguments as written.

use anyhow::{anyhow, Context, Result};
use keycloak::KeycloakTokenSupplier;
use sequent_core::ballot::ElectionEventPresentation;
use sequent_core::services::keycloak::{get_event_realm, KeycloakAdminClient, PubKeycloakAdmin};
use sequent_core::services::reports::{is_invalid_timezone_text, normalize_placeholders};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use tracing::{info, instrument, warn};

/// The key prefixes this copy owns in the realm.
pub const SYNCED_PREFIXES: [&str; 2] = ["timezones.", "enrollment."];

const GLOBAL_SCOPE: &str = "global:";
const VOTING_PORTAL_SCOPE: &str = "votingPortal:";

/// i18next placeholders of the `timezones.*` texts and their Keycloak index
/// (the order the theme passes them: date and time, then the zone).
const PLACEHOLDERS: [(&str, &str); 2] = [("{{dateTime}}", "{0}"), ("{{zoneName}}", "{1}")];

/// locale -> key -> text.
pub type RealmTexts = BTreeMap<String, BTreeMap<String, String>>;

/// Copies the overrides into the event realm and removes the ones that are
/// gone.
#[instrument(skip(presentation), err)]
pub async fn sync_event_realm_localization(
    tenant_id: &str,
    election_event_id: &str,
    presentation: Option<&ElectionEventPresentation>,
) -> Result<()> {
    let realm = get_event_realm(tenant_id, election_event_id);
    let wanted = realm_texts(presentation);
    let client = KeycloakAdminClient::new().await?;
    let pub_admin = KeycloakAdminClient::pub_new().await?;

    let mut locales: BTreeSet<String> = wanted.keys().cloned().collect();
    locales.extend(
        client
            .client
            .realm_localization_get(&realm)
            .await
            .map_err(|err| anyhow!("Error listing the realm localization locales: {err:?}"))?,
    );

    let empty = BTreeMap::new();
    for locale in locales {
        let texts = wanted.get(&locale).unwrap_or(&empty);
        let current: HashMap<String, String> = client
            .client
            .realm_localization_with_locale_get(&realm, &locale, Some(false))
            .await
            .map_err(|err| anyhow!("Error reading the realm localization of {locale}: {err:?}"))?
            .into_iter()
            .collect();
        for key in stale_keys(&current, texts) {
            delete_text(&pub_admin, &realm, &locale, &key).await?;
        }
        if !texts.is_empty() {
            client
                .client
                .realm_localization_with_locale_post(
                    &realm,
                    &locale,
                    texts.clone().into_iter().collect(),
                )
                .await
                .map_err(|err| {
                    anyhow!("Error writing the realm localization of {locale}: {err:?}")
                })?;
        }
        info!(
            "Realm {realm} {locale}: {} timezone and enrollment texts",
            texts.len()
        );
    }
    Ok(())
}

/// The realm texts the event's overrides ask for.
pub fn realm_texts(presentation: Option<&ElectionEventPresentation>) -> RealmTexts {
    let Some(i18n) = presentation.and_then(|presentation| presentation.i18n.as_ref()) else {
        return RealmTexts::new();
    };
    let mut texts = RealmTexts::new();
    for (language, overrides) in i18n {
        let mut locale_texts = BTreeMap::new();
        // global: first, so votingPortal: overwrites it.
        for scope in [GLOBAL_SCOPE, VOTING_PORTAL_SCOPE] {
            for (stored_key, value) in overrides {
                let Some(key) = stored_key.strip_prefix(scope) else {
                    continue;
                };
                // This private default must survive invalid imported or legacy overrides.
                if key == "timezones.defaultVoterDateTimeZone" {
                    continue;
                }
                if !SYNCED_PREFIXES.iter().any(|prefix| key.starts_with(prefix)) {
                    continue;
                }
                let Some(value) = value.as_deref().filter(|value| !value.trim().is_empty()) else {
                    continue;
                };
                if is_invalid_timezone_text(key, value) {
                    warn!(
                        "Invalid {key} override for {language}: enrollment uses the theme default"
                    );
                    locale_texts.remove(key);
                    continue;
                }
                locale_texts.insert(key.to_string(), keycloak_text(key, value));
            }
        }
        if !locale_texts.is_empty() {
            texts.insert(keycloak_locale(language).to_string(), locale_texts);
        }
    }
    texts
}

/// Realm keys under the synced prefixes that the overrides no longer set.
pub fn stale_keys(
    current: &HashMap<String, String>,
    wanted: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut keys: Vec<String> = current
        .keys()
        .filter(|key| SYNCED_PREFIXES.iter().any(|prefix| key.starts_with(prefix)))
        .filter(|key| !wanted.contains_key(*key))
        .cloned()
        .collect();
    keys.sort();
    keys
}

/// The Keycloak message format of an override.
pub fn keycloak_text(key: &str, value: &str) -> String {
    let mut text = value.replace('\'', "''");
    if !key.starts_with("timezones.") {
        return text;
    }
    text = normalize_placeholders(&text);
    for (placeholder, index) in PLACEHOLDERS {
        text = text.replace(placeholder, index);
    }
    text
}

/// The Keycloak locale of an event language (the portals call Catalan `cat`).
pub fn keycloak_locale(language: &str) -> &str {
    match language {
        "cat" => "ca",
        other => other,
    }
}

/// `DELETE /admin/realms/{realm}/localization/{locale}/{key}`, with the key
/// as one path segment (zone keys contain `/`).
async fn delete_text(admin: &PubKeycloakAdmin, realm: &str, locale: &str, key: &str) -> Result<()> {
    let mut url = reqwest::Url::parse(&admin.url).with_context(|| "Invalid Keycloak URL")?;
    url.path_segments_mut()
        .map_err(|_| anyhow!("Keycloak URL can't take a path"))?
        .pop_if_empty()
        .extend(["admin", "realms", realm, "localization", locale, key]);
    let token = admin
        .token_supplier
        .get(&admin.url)
        .await
        .map_err(|err| anyhow!("Error getting a Keycloak token: {err:?}"))?;
    let response = admin
        .client
        .delete(url.as_str())
        .bearer_auth(token)
        .send()
        .await
        .with_context(|| format!("Error removing realm text {key} ({locale})"))?;
    check_delete_status(response.status(), realm, locale, key)
}

/// Removing an already absent override is idempotent. All other failures must
/// stop publication synchronization so stale enrollment text is not reported as updated.
fn check_delete_status(
    status: reqwest::StatusCode,
    realm: &str,
    locale: &str,
    key: &str,
) -> Result<()> {
    if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
        return Ok(());
    }
    Err(anyhow!("Keycloak answered {status} removing realm text {key} ({locale}) from {realm}: the old override still shows on the enrollment pages"))
}

#[cfg(test)]
#[path = "realm_localization_tests.rs"]
mod realm_localization_tests;

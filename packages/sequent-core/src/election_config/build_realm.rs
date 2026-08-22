// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The Keycloak realm: what a source document may ask of it, and how.
//!
//! A realm is ~165 kB of interdependent Keycloak configuration — authentication
//! flows, clients, a user profile carried as a stringified JSON blob — none of
//! which can be authored from a spreadsheet, and whose client URLs belong to the
//! environment it came from. So a realm someone exported from a working event is
//! patched, or none is emitted and the platform loads its own provisioned default.
//!
//! Emitting an invented realm would be worse than emitting none: the importer
//! takes `keycloak_event_realm` **wholesale**, so a present realm *replaces* the
//! environment's default rather than merging into it. When there is no base export
//! the patch is kept as its own artifact instead, so nothing the document asked
//! for is lost.

use super::tables::VOTER_LEADING_COLUMNS;
use super::{value_as_text, Builder};
use crate::election_config::branding;
use crate::election_config::paths::{deep_merge, set_path, split_path};
use crate::election_config::presets::{
    self, AuthPreset, PresetInput, RealmPatch, RequirementKind, PARAM_AUTH_TYPE,
};
use crate::election_config::problem::Code;
use crate::election_config::sheet::SHEET_VOTERS;
use crate::election_config::sheet::{
    Origin, SHEET_ADMIN_USERS, SHEET_ELECTIONS, SHEET_REPORTS,
};
use serde_json::json;
use serde_json::{Map, Value};

/// The parameter carrying the login page's stylesheet.
pub const PARAM_LOGIN_CUSTOM_CSS: &str = "login_custom_css";

impl Builder<'_> {
    /// The preset named by the caller, else by the document's `auth_type`.
    ///
    /// `None` leaves the realm entirely alone — no preset, no patch, and
    /// `keycloak_event_realm` stays whatever a base export gave, or null.
    pub(super) fn resolve_auth_preset(
        &mut self,
        explicit: Option<&str>,
    ) -> Option<&'static AuthPreset> {
        if explicit
            .map(|name| name.trim().eq_ignore_ascii_case(presets::NONE))
            .unwrap_or(false)
        {
            // Explicitly ignore whatever the document declares.
            return None;
        }

        let declared = self.parameter(PARAM_AUTH_TYPE).map(value_as_text);
        let name = match explicit {
            Some(explicit) if !explicit.trim().is_empty() => {
                explicit.trim().to_string()
            }
            _ => match declared {
                Some(declared) if !declared.trim().is_empty() => declared,
                _ => return None,
            },
        };

        let Some(preset) = presets::get(&name) else {
            // Whichever said it wrong is where the author has to look.
            let origin = if explicit.is_some() {
                Origin::sheet("auth preset option")
            } else {
                Origin::column("Parameters", "value")
            };
            let message = format!(
                "'{name}' is not an authentication preset; expected one of {}",
                presets::names().join(", ")
            );
            self.problem(origin, Code::InvalidValue, message);
            return None;
        };

        let missing: Vec<&str> = preset
            .required_parameters
            .iter()
            .filter(|key| {
                self.parameter(key)
                    .map(value_as_text)
                    .filter(|value| !value.trim().is_empty())
                    .is_none()
            })
            .copied()
            .collect();

        if !missing.is_empty() {
            let wanted: Vec<String> = missing
                .iter()
                .map(|key| format!("a '{key}' parameter"))
                .collect();
            let message = format!(
                "the '{}' preset needs {}. Add it to the Parameters sheet, or \
                 select the 'none' preset to build without configuring \
                 authentication.",
                preset.name,
                wanted.join(", ")
            );
            self.problem(
                Origin::sheet("Parameters"),
                Code::MissingField,
                message,
            );
            return None;
        }
        Some(preset)
    }

    /// Everything the document asks of the realm, as one patch.
    ///
    /// Kept as its own artifact whether or not a base export was given, so a
    /// `keycloak_event_realm.*` parameter or an `auth_type` is never silently
    /// dropped just because there was no realm to apply it to.
    pub(super) fn build_realm_patch(&mut self) -> RealmPatch {
        let mut result = RealmPatch::default();

        if let Some(preset) = self.auth_preset {
            let values: Vec<(String, Value)> = preset
                .consumes()
                .iter()
                .filter_map(|key| {
                    self.parameter(key)
                        .map(|value| ((*key).to_string(), value.clone()))
                })
                .collect();
            result = preset.build(&PresetInput::new(values));
        }

        result.patch =
            merge_maps(result.patch, self.event_derived_realm_patch());

        // Explicit parameters last, so they can override anything derived.
        let mut problems = Vec::new();
        for (path, value) in self.parameter_patches("keycloak_event_realm.") {
            if let Err(problem) =
                set_path(&mut result.patch, &split_path(&path), value)
            {
                problems.push(problem);
            }
        }
        for problem in problems {
            self.report.push(problem);
        }

        result
    }

    /// Realm settings the event already states, which nothing else carries over.
    ///
    /// The platform syncs the default locale only under a force-default detection
    /// policy, never syncs `supportedLocales`, and has no path at all from an event
    /// name to a realm display name. Stating them twice in a document would be a
    /// way to get them out of step, so they are derived from what the event says.
    fn event_derived_realm_patch(&mut self) -> Map<String, Value> {
        let presentation = self
            .event_row
            .overrides(&[])
            .ok()
            .and_then(|overrides| overrides.get("presentation").cloned())
            .unwrap_or(Value::Null);

        let language_conf = presentation.get("language_conf");
        let default_language = language_conf
            .and_then(|conf| conf.get("default_language_code"))
            .and_then(Value::as_str);

        let mut patch = branding::language_patch(language_conf);
        for (key, value) in
            branding::title_patch(presentation.get("i18n"), default_language)
        {
            patch.insert(key, value);
        }

        if let Some(css) = self
            .parameter(PARAM_LOGIN_CUSTOM_CSS)
            .map(value_as_text)
            .filter(|css| !css.trim().is_empty())
        {
            let locales = branding::locales_of(language_conf, "en");
            patch =
                merge_maps(patch, branding::login_css_patch(&css, &locales));
        }
        patch
    }

    /// `keycloak_admin_realm.*` parameters, as a nested object.
    ///
    /// Its own artifact: the admin realm is tenant-scoped, so it is not part of an
    /// election event import.
    pub(super) fn admin_realm_patch(&mut self) -> Map<String, Value> {
        let patches = self.parameter_patches("keycloak_admin_realm.");
        if patches.is_empty() {
            return Map::new();
        }
        match crate::election_config::paths::expand(&patches) {
            Ok(expanded) => expanded,
            Err(problem) => {
                self.report.push(problem);
                Map::new()
            }
        }
    }

    /// Patch the base export's realm, or emit none.
    pub(super) fn build_realm(&mut self) -> Value {
        let Some(Value::Object(realm)) =
            self.base_export.get("keycloak_event_realm").cloned()
        else {
            if !self.realm_patch.patch.is_empty() {
                let mut keys: Vec<&str> =
                    self.realm_patch.patch.keys().map(String::as_str).collect();
                keys.sort_unstable();
                let message = format!(
                    "no base export, so nothing here configures the login page: \
                     {} are kept as a realm patch instead of being applied. Until \
                     that patch reaches the realm, voters see the platform's \
                     default login page.",
                    keys.join(", ")
                );
                self.warn("keycloak_event_realm", message);
            }
            return Value::Null;
        };

        let mut realm = realm;

        // The realm name is structural: the voting portal and the smart-link URLs
        // derive it from tenant + event, so it is not a free choice.
        realm.insert(
            "realm".to_string(),
            Value::String(format!(
                "tenant-{}-event-{}",
                self.tenant_id, self.event_id
            )),
        );
        realm.insert("id".to_string(), Value::String(self.event_id.clone()));

        let realm = self.apply_realm_patch(realm);

        // Hosts in rootUrl/redirectUris belong to the environment and stay, but a
        // base export also embeds its OWN event id in those URLs. Import remaps
        // every UUID it finds, so a stale id would be remapped to something
        // unrelated; swapping it first makes the remap land right.
        let base_event_id = self
            .base_export
            .get("election_event")
            .and_then(|event| event.get("id"))
            .and_then(Value::as_str)
            .map(str::to_string);

        match base_event_id {
            Some(base_event_id) if base_event_id != self.event_id => {
                let encoded = Value::Object(realm.clone()).to_string();
                let swapped = encoded.replace(&base_event_id, &self.event_id);
                // The un-swapped realm, not `Value::String(swapped)`: a string here
                // means `keycloak_event_realm` holds text where the importer expects
                // an object, and it takes it wholesale. Keeping the base event's ids
                // is the lesser fault, and it is said out loud.
                match serde_json::from_str(&swapped) {
                    Ok(reparsed) => reparsed,
                    Err(error) => {
                        self.warn(
                            "keycloak_event_realm",
                            format!(
                                "the base export's realm could not be re-read after \
                                 swapping the event id ({error}), so it is carried \
                                 over unchanged"
                            ),
                        );
                        Value::Object(realm)
                    }
                }
            }
            _ => Value::Object(realm),
        }
    }

    /// Apply the patch to a real realm, checking what the preset assumes.
    ///
    /// Alias-keyed collections are merged by alias rather than replaced: a realm's
    /// `identityProviders` and `authenticatorConfig` are referenced by alias from
    /// elsewhere, so replacing either wholesale would strip providers the
    /// environment configured on purpose.
    fn apply_realm_patch(
        &mut self,
        realm: Map<String, Value>,
    ) -> Map<String, Value> {
        let mut realm = realm;
        let mut patch = self.realm_patch.patch.clone();

        if self.auth_preset.is_some() {
            self.check_realm_requirements(&realm);
        }

        for key in ["identityProviders", "authenticatorConfig"] {
            if let Some(Value::Array(additions)) = patch.remove(key) {
                if additions.is_empty() {
                    continue;
                }
                let existing = match realm.get(key) {
                    Some(Value::Array(existing)) => existing.clone(),
                    _ => Vec::new(),
                };
                realm.insert(
                    key.to_string(),
                    Value::Array(merge_by_alias(existing, additions)),
                );
            }
        }

        let mut realm = merge_maps(realm, patch);

        if let Some((authenticator, config_alias)) =
            self.realm_patch.bind_authenticator_config.clone()
        {
            bind_authenticator_config(
                &mut realm,
                &authenticator,
                &config_alias,
            );
        }
        if let Some(profile) = self.realm_patch.user_profile.clone() {
            self.patch_user_profile(&mut realm, &profile);
        }
        self.declare_census_attributes(&mut realm);
        realm
    }

    /// Report anything the preset needs that the target realm lacks.
    fn check_realm_requirements(&mut self, realm: &Map<String, Value>) {
        let Some(preset) = self.auth_preset else {
            return;
        };

        let flows = aliases_of(realm.get("authenticationFlows"), "alias");
        let configs = aliases_of(realm.get("authenticatorConfig"), "alias");

        let authenticators: Vec<String> = realm
            .get("authenticationFlows")
            .and_then(Value::as_array)
            .map(|flows| {
                flows
                    .iter()
                    .filter_map(|flow| {
                        flow.get("authenticationExecutions")?.as_array()
                    })
                    .flatten()
                    .filter_map(|execution| {
                        execution.get("authenticator")?.as_str()
                    })
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let mut warnings = Vec::new();
        for requirement in preset.requires {
            // Exhaustive: the `_ =>` arm this replaces meant a misspelled kind was
            // checked against the authenticators and reported nothing.
            let present = match requirement.kind {
                RequirementKind::Flow => &flows,
                RequirementKind::Authenticator => &authenticators,
                RequirementKind::AuthenticatorConfig => &configs,
            };
            if !present.iter().any(|name| name == requirement.name) {
                warnings.push(format!(
                    "the base export's realm has no {} '{}', which the '{}' \
                     preset needs: {}. The patch is still written out on its own.",
                    requirement.kind,
                    requirement.name,
                    preset.name,
                    requirement.why
                ));
            }
        }
        for warning in warnings {
            self.warn("keycloak_event_realm", warning);
        }
    }

    /// Declare the census's own extra columns in the realm's user profile.
    ///
    /// The gap this closes: a census column the wizard has no field for becomes a
    /// **Keycloak user attribute** — that passthrough is the whole reason a client can
    /// carry a reporting breakout without a code change. But Keycloak only stores an
    /// attribute its user profile declares. An undeclared one is dropped or refused
    /// depending on the realm's unmanaged-attribute policy, and either way the column
    /// is in the file, in the import, and not on the voter.
    ///
    /// Nothing reported it. `patch_user_profile` above *warns* about an attribute a
    /// preset wanted and the realm lacks, which is right — a preset and a realm
    /// disagreeing is a mismatch somebody has to resolve. A census column is different:
    /// the author is declaring a new attribute rather than expecting an existing one,
    /// so the answer is to add it.
    ///
    /// Added minimally and permissively: a name, and permissions letting an
    /// administrator read and write it. Not required, not user-editable, no validator —
    /// this knows the column exists and nothing about what belongs in it, and a guessed
    /// validator would refuse data the client's own file contains.
    fn declare_census_attributes(&mut self, realm: &mut Map<String, Value>) {
        let extra = self.census_attributes();
        if extra.is_empty() {
            return;
        }

        let Some(component) = realm
            .get_mut("components")
            .and_then(|components| {
                components
                    .get_mut("org.keycloak.userprofile.UserProfileProvider")
            })
            .and_then(Value::as_array_mut)
            .and_then(|components| components.first_mut())
        else {
            // Said once, naming the columns, because the consequence is per column
            // and the cause is one missing component.
            let message = format!(
                "the realm has no user profile component, so the census's own \
                 columns ({}) cannot be declared — Keycloak will drop them",
                extra.join(", ")
            );
            self.warn("keycloak_event_realm", message);
            return;
        };

        let raw = component
            .get("config")
            .and_then(|config| config.get("kc.user.profile.config"))
            .map(|value| match value {
                Value::Array(items) => items
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                Value::String(text) => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        let Ok(mut profile) = serde_json::from_str::<Value>(&raw) else {
            // `patch_user_profile` has already reported an unreadable profile; a
            // second copy of the same problem is noise.
            return;
        };

        let mut added = Vec::new();
        if let Some(attributes) =
            profile.get_mut("attributes").and_then(Value::as_array_mut)
        {
            for name in &extra {
                let known = attributes.iter().any(|attribute| {
                    attribute.get("name").and_then(Value::as_str)
                        == Some(name.as_str())
                });
                if known {
                    continue;
                }
                attributes.push(json!({
                    "name": name,
                    "displayName": name,
                    "permissions": {"view": ["admin"], "edit": ["admin"]},
                    "multivalued": false,
                }));
                added.push(name.clone());
            }
        }

        if added.is_empty() {
            return;
        }

        let encoded = profile.to_string();
        if let Some(config) = component
            .as_object_mut()
            .and_then(|component| component.get_mut("config"))
            .and_then(Value::as_object_mut)
        {
            config
                .insert("kc.user.profile.config".to_string(), json!([encoded]));
        }

        // Nothing is reported. `Severity` is `Error` or `Warning`, and neither fits:
        // this is the wizard doing exactly what the author asked for, and a warning
        // saying so is a warning that teaches people to skim the ones that matter.
        //
        // It is not silent, though — the wizard shows a dialog when the column is
        // added, which is the moment somebody can still change their mind. Saying it
        // there rather than on the review screen is the whole difference between
        // information and a footnote.
    }

    /// The census's columns that are not the platform's own.
    ///
    /// From the headers rather than from the rows: a column present in the header
    /// and empty in every row is still a column the author declared, and dropping it
    /// here would mean an attribute that appears the moment somebody fills one cell
    /// in.
    ///
    /// **From the source's headers where there is a source.** This is the one place
    /// the census's *shape* decides something outside the census, and it decides it
    /// silently — Keycloak drops an attribute its user profile does not declare, so
    /// a column that never reaches this list is a value the sign-in flow reads as
    /// absent, with no error anywhere. A census that is a file rather than a `Vec`
    /// can answer "which columns" from its first line, which is why
    /// [`CensusSource::columns`] exists at all; asking the Voters sheet instead
    /// would mean the answer could not be given until every row had been read into
    /// memory.
    ///
    /// The sheet is still the answer for a workbook built straight from xlsx, which
    /// has no source. `the_realm_declares_the_same_attributes_either_way` is what
    /// keeps the two from drifting.
    fn census_attributes(&self) -> Vec<String> {
        let sheet_headers = || {
            self.workbook
                .sheet(SHEET_VOTERS)
                .map(|sheet| sheet.headers.clone())
                .unwrap_or_default()
        };
        let headers = match self.census {
            Some(census) => census.columns().to_vec(),
            None => sheet_headers(),
        };

        let mut found: Vec<String> = Vec::new();
        for header in &headers {
            let name = header.trim();
            if name.is_empty()
                || VOTER_LEADING_COLUMNS.contains(&name)
                || name == "area.external_id"
                || found.iter().any(|seen| seen == name)
            {
                continue;
            }
            found.push(name.to_string());
        }
        found.sort();
        found
    }

    /// Patch the user profile, which travels as a stringified JSON blob.
    ///
    /// It lives inside a Keycloak component's config as a single JSON string, so it
    /// has to be parsed, patched and re-serialised rather than merged.
    fn patch_user_profile(
        &mut self,
        realm: &mut Map<String, Value>,
        profile_patch: &Map<String, Value>,
    ) {
        let preset_name =
            self.auth_preset.map_or("selected", |preset| preset.name);

        let component = realm
            .get_mut("components")
            .and_then(|components| {
                components
                    .get_mut("org.keycloak.userprofile.UserProfileProvider")
            })
            .and_then(Value::as_array_mut)
            .and_then(|components| components.first_mut());

        let Some(component) = component else {
            let message = format!(
                "the base export's realm has no user profile component, so the \
                 '{preset_name}' preset could not set which login fields are \
                 typeable"
            );
            self.warn("keycloak_event_realm", message);
            return;
        };

        // The config value is a one-element list of JSON text.
        let raw = component
            .get("config")
            .and_then(|config| config.get("kc.user.profile.config"))
            .map(|value| match value {
                Value::Array(items) => items
                    .first()
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                Value::String(text) => text.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();

        if raw.is_empty() {
            self.warn(
                "keycloak_event_realm",
                "the base export's realm user profile component is empty",
            );
            return;
        }

        let mut profile: Value = match serde_json::from_str(&raw) {
            Ok(profile) => profile,
            Err(error) => {
                let message = format!(
                    "the realm's user profile is not readable JSON: {error}"
                );
                self.problem(
                    Origin::sheet("base export"),
                    Code::InvalidValue,
                    message,
                );
                return;
            }
        };

        let mut missing = Vec::new();
        {
            let attributes =
                profile.get_mut("attributes").and_then(Value::as_array_mut);
            let Some(attributes) = attributes else {
                self.warn(
                    "keycloak_event_realm",
                    "the base export's realm user profile lists no attributes",
                );
                return;
            };

            for (name, changes) in profile_patch {
                let attribute = attributes.iter_mut().find(|attribute| {
                    attribute.get("name").and_then(Value::as_str) == Some(name)
                });
                match attribute {
                    Some(attribute) => {
                        if let (Some(target), Some(changes)) =
                            (attribute.as_object_mut(), changes.as_object())
                        {
                            for (key, value) in changes {
                                target.insert(key.clone(), value.clone());
                            }
                        }
                    }
                    None => missing.push(name.clone()),
                }
            }
        }

        for name in missing {
            let message = format!(
                "the realm's user profile has no '{name}' attribute, so the \
                 '{preset_name}' preset could not configure it"
            );
            self.warn("keycloak_event_realm", message);
        }

        let encoded = profile.to_string();
        // Reported rather than asserted: the array element can be any JSON value, and
        // a base export holding a string there would otherwise abort the whole build.
        // Same shape as the missing-attributes warning above.
        let Some(component) = component.as_object_mut() else {
            self.warn(
                "keycloak_event_realm",
                "the base export's user profile component is not an object, so the \
                 census columns were left undeclared",
            );
            return;
        };
        let config = component
            .entry("config")
            .or_insert_with(|| Value::Object(Map::new()));
        if let Some(config) = config.as_object_mut() {
            config.insert(
                "kc.user.profile.config".to_string(),
                Value::Array(vec![Value::String(encoded)]),
            );
        }
    }

    /// Check every permission label against the administrators who hold one.
    ///
    /// A permission label scopes an entity to administrators carrying that label.
    /// Hasura filters `election` and `report` on `permission_label IS NULL OR
    /// permission_label IN X-Hasura-Permission-Labels`, where the claim comes from
    /// the `permission_labels` attribute on the Keycloak administrator.
    ///
    /// The failure this guards against is quiet and expensive: an election whose
    /// label nobody holds imports cleanly, reports no error, and then does not
    /// appear in the Elections list at all. It happened on the first real import,
    /// where a document labelled an election `dlc-officers-dburs` while its own
    /// administrators carried `dlc-officers`. Nothing anywhere said so.
    ///
    /// Warnings rather than errors, because the document is not the whole picture:
    /// administrators may already exist in the target tenant carrying labels this
    /// file knows nothing about.
    pub(super) fn warn_permission_labels(&mut self) {
        // Insertion-ordered so the message does not depend on a hash.
        let mut used: Vec<(String, Vec<String>)> = Vec::new();
        let note = |used: &mut Vec<(String, Vec<String>)>,
                    label: String,
                    entity: String| {
            match used.iter_mut().find(|(seen, _)| seen == &label) {
                Some((_, entities)) => entities.push(entity),
                None => used.push((label, vec![entity])),
            }
        };

        for row in self.workbook.rows(SHEET_ELECTIONS) {
            if let Some(label) = row.get("permission_label").map(value_as_text)
            {
                let label = label.trim().to_string();
                if label.is_empty() {
                    continue;
                }
                let entity = format!(
                    "election '{}'",
                    row.text("external_id")
                        .map(str::to_string)
                        .unwrap_or_else(|| row.number.to_string())
                );
                note(&mut used, label, entity);
            }
        }

        for row in self.workbook.rows(SHEET_REPORTS) {
            for label in labels_of(row.get("permission_label")) {
                note(&mut used, label, format!("report on row {}", row.number));
            }
        }

        if used.is_empty() {
            return;
        }

        let mut granted: Vec<String> = Vec::new();
        for row in self.workbook.rows(SHEET_ADMIN_USERS) {
            for label in labels_of(row.get("permission_labels")) {
                if !granted.contains(&label) {
                    granted.push(label);
                }
            }
        }

        let mut unmatched: Vec<&(String, Vec<String>)> = used
            .iter()
            .filter(|(label, _)| !granted.contains(label))
            .collect();
        unmatched.sort_by(|left, right| left.0.cmp(&right.0));

        let mut warnings = Vec::new();
        for (label, entities) in unmatched {
            let listed: Vec<&str> =
                entities.iter().take(4).map(String::as_str).collect();
            let more = if entities.len() > 4 {
                format!(" and {} more", entities.len() - 4)
            } else {
                String::new()
            };
            let nobody = if granted.is_empty() {
                ", and this document grants no permission labels to anyone"
            } else {
                ", but no administrator in the Admin Users sheet carries it"
            };
            warnings.push(format!(
                "permission label '{label}' is used by {}{more}{nobody}. \
                 Anything carrying a label is hidden from every administrator \
                 without it — the event imports cleanly and then lists nothing.",
                listed.join(", ")
            ));
        }

        let mut all: Vec<&str> =
            used.iter().map(|(label, _)| label.as_str()).collect();
        all.sort_unstable();
        warnings.push(format!(
            "permission labels in use: {}. Whoever imports this event needs one \
             of them on their own 'permission_labels' attribute, or the Admin \
             Portal will show them an empty list.",
            all.join(", ")
        ));

        for warning in warnings {
            self.warn("permission_label", warning);
        }
    }

    /// Say out loud what a base export contributed to the voter's screen.
    ///
    /// Entity fields the template does not set are inherited from the base, and
    /// `presentation.i18n` merges key by key. That is useful when the base is a
    /// reference event and wrong when it is another client's: their login title and
    /// instruction copy would come along silently. A base export should be a
    /// generic reference event, and if it is not, this is where you find out.
    pub(super) fn warn_inherited_branding(
        &mut self,
        base: &Map<String, Value>,
        event: &Map<String, Value>,
    ) {
        let base_presentation = base.get("presentation");
        let event_presentation = event.get("presentation");

        let mut inherited: Vec<&String> = Vec::new();
        if let Some(Value::Object(base_presentation)) = base_presentation {
            for (key, value) in base_presentation {
                if ["i18n", "css", "logo_url"].contains(&key.as_str()) {
                    continue;
                }
                if value.is_null()
                    || value == &Value::String(String::new())
                    || value == &Value::Object(Map::new())
                    || value == &Value::Array(Vec::new())
                {
                    continue;
                }
                if event_presentation.and_then(|event| event.get(key))
                    == Some(value)
                {
                    inherited.push(key);
                }
            }
        }
        inherited.sort();

        let base_copy: Vec<&String> = base_presentation
            .and_then(|presentation| presentation.get("i18n"))
            .and_then(|i18n| i18n.get("en"))
            .and_then(Value::as_object)
            .map(|english| english.keys().collect())
            .unwrap_or_default();

        let own_copy: Vec<String> = self
            .event_row
            .overrides(&[])
            .ok()
            .and_then(|overrides| {
                overrides
                    .get("presentation")?
                    .get("i18n")?
                    .get("en")?
                    .as_object()
                    .map(|english| english.keys().cloned().collect())
            })
            .unwrap_or_default();

        let mut inherited_copy: Vec<&String> = base_copy
            .into_iter()
            .filter(|key| !own_copy.contains(key))
            .collect();
        inherited_copy.sort();

        let mut warnings = Vec::new();
        if !inherited_copy.is_empty() {
            let listed: Vec<String> = inherited_copy
                .iter()
                .take(6)
                .map(|key| format!("presentation.i18n.en.{key}"))
                .collect();
            let more = if inherited_copy.len() > 6 {
                format!(" and {} more", inherited_copy.len() - 6)
            } else {
                String::new()
            };
            warnings.push(format!(
                "the base export's voter-facing copy is inherited: {}{more}. Use \
                 a reference event as the base, not another client's, or set these \
                 in the ElectionEvent sheet.",
                listed.join(", ")
            ));
        }
        if !inherited.is_empty() {
            let listed: Vec<&str> =
                inherited.iter().take(8).map(|key| key.as_str()).collect();
            let more = if inherited.len() > 8 {
                format!(" and {} more", inherited.len() - 8)
            } else {
                String::new()
            };
            warnings.push(format!(
                "presentation settings inherited from the base export: {}{more}",
                listed.join(", ")
            ));
        }

        for warning in warnings {
            self.warn("election_event.presentation", warning);
        }
    }
}

/// Merge alias-keyed realm collections, replacing by alias and appending the rest.
fn merge_by_alias(existing: Vec<Value>, additions: Vec<Value>) -> Vec<Value> {
    let mut merged = existing;
    for addition in additions {
        let alias = addition
            .get("alias")
            .and_then(Value::as_str)
            .map(str::to_string);
        let at = alias.as_ref().and_then(|alias| {
            merged.iter().position(|item| {
                item.get("alias").and_then(Value::as_str) == Some(alias)
            })
        });
        match at {
            Some(index) => {
                let existing = merged[index].clone();
                merged[index] = deep_merge(existing, addition);
            }
            None => merged.push(addition),
        }
    }
    merged
}

/// Point every execution of an authenticator at a config alias.
fn bind_authenticator_config(
    realm: &mut Map<String, Value>,
    authenticator: &str,
    config_alias: &str,
) {
    let Some(flows) = realm
        .get_mut("authenticationFlows")
        .and_then(Value::as_array_mut)
    else {
        return;
    };

    for flow in flows {
        let Some(executions) = flow
            .get_mut("authenticationExecutions")
            .and_then(Value::as_array_mut)
        else {
            continue;
        };
        for execution in executions {
            if execution.get("authenticator").and_then(Value::as_str)
                == Some(authenticator)
            {
                if let Some(execution) = execution.as_object_mut() {
                    execution.insert(
                        "authenticatorConfig".to_string(),
                        Value::String(config_alias.to_string()),
                    );
                }
            }
        }
    }
}

/// The values of `key` across a list of objects.
fn aliases_of(value: Option<&Value>, key: &str) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.get(key)?.as_str())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// A cell that may hold one label or a list of them.
fn labels_of(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::Array(items)) => items
            .iter()
            .map(value_as_text)
            .map(|label| label.trim().to_string())
            .filter(|label| !label.is_empty())
            .collect(),
        Some(Value::Null) | None => Vec::new(),
        Some(other) => {
            let label = value_as_text(other).trim().to_string();
            if label.is_empty() {
                Vec::new()
            } else {
                vec![label]
            }
        }
    }
}

/// Deep-merge one object over another.
fn merge_maps(
    base: Map<String, Value>,
    over: Map<String, Value>,
) -> Map<String, Value> {
    match deep_merge(Value::Object(base), Value::Object(over)) {
        Value::Object(merged) => merged,
        _ => unreachable!("merging two objects yields an object"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_provider_with_the_same_alias_is_merged_not_appended() {
        // identityProviders is referenced by alias from elsewhere in the realm, so
        // two entries under one alias is a realm nothing can resolve.
        let merged = merge_by_alias(
            vec![json!({"alias": "a", "enabled": false, "keep": 1})],
            vec![json!({"alias": "a", "enabled": true})],
        );
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0],
            json!({"alias": "a", "enabled": true, "keep": 1})
        );
    }

    #[test]
    fn a_provider_the_realm_does_not_have_is_appended() {
        // Replacing the list wholesale would strip providers the environment
        // configured on purpose.
        let merged = merge_by_alias(
            vec![json!({"alias": "environment-idp"})],
            vec![json!({"alias": "client-saml-idp"})],
        );
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0]["alias"], json!("environment-idp"));
        assert_eq!(merged[1]["alias"], json!("client-saml-idp"));
    }

    #[test]
    fn an_addition_with_no_alias_is_appended_rather_than_dropped() {
        let merged =
            merge_by_alias(vec![json!({"alias": "a"})], vec![json!({"x": 1})]);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn every_execution_of_the_authenticator_gets_the_config() {
        // A realm may run the same authenticator in more than one flow, and a step
        // left unbound is a step with no configuration.
        let mut realm = match json!({
            "authenticationFlows": [
                {"authenticationExecutions": [
                    {"authenticator": "message-otp-authenticator"},
                    {"authenticator": "something-else"},
                ]},
                {"authenticationExecutions": [
                    {"authenticator": "message-otp-authenticator"},
                ]},
            ]
        }) {
            Value::Object(realm) => realm,
            _ => unreachable!(),
        };

        bind_authenticator_config(
            &mut realm,
            "message-otp-authenticator",
            "janitor-otp-by-availability",
        );

        let flows = realm["authenticationFlows"].as_array().unwrap();
        assert_eq!(
            flows[0]["authenticationExecutions"][0]["authenticatorConfig"],
            json!("janitor-otp-by-availability")
        );
        assert!(flows[0]["authenticationExecutions"][1]
            .get("authenticatorConfig")
            .is_none());
        assert_eq!(
            flows[1]["authenticationExecutions"][0]["authenticatorConfig"],
            json!("janitor-otp-by-availability")
        );
    }

    #[test]
    fn binding_a_realm_with_no_flows_does_nothing_rather_than_panicking() {
        let mut realm = Map::new();
        bind_authenticator_config(&mut realm, "a", "b");
        assert!(realm.is_empty());
    }

    #[test]
    fn a_label_cell_reads_as_one_label_or_a_list() {
        assert_eq!(labels_of(Some(&json!("one"))), ["one"]);
        assert_eq!(labels_of(Some(&json!(["a", "b"]))), ["a", "b"]);
        assert_eq!(labels_of(Some(&json!(["a", "  ", "b"]))), ["a", "b"]);
        assert!(labels_of(Some(&json!(" "))).is_empty());
        assert!(labels_of(None).is_empty());
    }
}

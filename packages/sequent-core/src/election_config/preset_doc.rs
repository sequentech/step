// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! An authentication preset as **data**, so a client profile can carry one.
//!
//! [`super::presets`] wrote each preset as a Rust function returning a
//! [`RealmPatch`]. That is fine for four presets nobody outside this crate needs
//! to change, and wrong the moment a client's Keycloak differs from ours — which
//! is the ordinary case, not the exception. A function cannot be edited in a
//! profile, cannot be reviewed by whoever runs that client's identity provider,
//! and cannot be added to without a release.
//!
//! So a preset is a document: the metadata a screen needs to offer it, and the
//! realm configuration as a **template**. The only thing separating the template
//! from the finished configuration is parameter substitution, and the only
//! substitution is `{{parameter_name}}` inside a string.
//!
//! **Deliberately not a general template language.** Handlebars is already a
//! dependency and was rejected here: conditionals and loops in a Keycloak patch
//! would make a preset a program, and a program is the thing this exists to stop
//! being. Every interpolation the four shipped presets need is one value in one
//! string — an alias, a URL, an OTP length — and anything a placeholder cannot
//! express is a new preset rather than a cleverer template.
//!
//! ## Why the shipped four live in a file
//!
//! `default_profile.json` is `include_str!`'d, so the configuration exists **once**
//! and in git. Transcribing it into Rust as well would be two copies of a Keycloak
//! realm patch to keep in step, and the first one to drift would be found by an
//! election that nobody could log into.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::presets::{PresetInput, RealmPatch, Requirement};

/// Something a preset needs the target realm to already have.
///
/// The owned twin of [`Requirement`], which is `&'static` because it is written
/// in Rust. A profile's requirements arrive at run time and cannot be.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NeedsDoc {
    /// `flow`, `authenticator` or `authenticator_config`.
    pub kind: String,
    pub name: String,
    /// What breaks without it, in a sentence a report can print.
    pub why: String,
}

impl NeedsDoc {
    /// Borrowed for the checks in [`super::presets`], which take `Requirement`.
    pub fn as_requirement(&self) -> Requirement {
        Requirement {
            kind: leak(&self.kind),
            name: leak(&self.name),
            why: leak(&self.why),
        }
    }
}

/// `&'static str` from an owned one.
///
/// `Requirement` is `Copy` and `&'static` throughout the build path, and a
/// profile's strings are not. Leaking is the honest way across: the alternative
/// is threading a lifetime through every caller of a type that exists to be
/// cheap. The quantity is bounded by the presets in one profile — four, in
/// practice — and a profile is read once per process.
fn leak(text: &str) -> &'static str {
    Box::leak(text.to_string().into_boxed_str())
}

/// One way of authenticating voters, as a profile carries it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthPresetDoc {
    /// What a plan's `auth_preset` names. Stable; the summary is not.
    pub name: String,

    /// One line for the person choosing, in their language.
    pub summary: String,

    /// Whether a voter needs an email address or mobile number to log in.
    ///
    /// The difference between "56 of 56 voters cannot be sent a one-time code"
    /// being a real problem and being noise: under SAML the client's identity
    /// provider authenticates the voter, and asking for contact details is not
    /// this tool's business.
    #[serde(default)]
    pub uses_otp: bool,

    /// What the target realm must already have for this to work.
    #[serde(default)]
    pub requires: Vec<NeedsDoc>,

    /// Parameters that must be supplied, by key.
    #[serde(default)]
    pub required_parameters: Vec<String>,

    #[serde(default)]
    pub optional_parameters: Vec<String>,

    /// User-profile attributes this preset reads off a voter.
    ///
    /// The census's column chooser offers exactly these, so a column somebody
    /// adds is one the sign-in flow can read rather than one Keycloak drops. The
    /// everyday five are not listed: they are in every realm, and repeating them
    /// per preset would be four chances to forget one.
    #[serde(default)]
    pub profile_attributes: Vec<String>,

    /// What a parameter is worth when the document does not say.
    ///
    /// Here rather than in the template so a placeholder means one thing —
    /// "substitute this" — and the fallback for `otp_length` is somewhere a
    /// profile author can see and change it.
    #[serde(default)]
    pub parameter_defaults: Map<String, Value>,

    /// The realm configuration, before substitution.
    #[serde(default)]
    pub patch: PatchDoc,
}

/// A [`RealmPatch`] with `{{parameter}}` placeholders still in it.
///
/// The two directives are separate fields rather than magic keys inside the
/// patch, exactly as `RealmPatch` has them — so writing the patch out never has
/// to strip them, and a caller cannot forget to.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PatchDoc {
    /// Merged into the realm, deeply.
    #[serde(default)]
    pub patch: Map<String, Value>,

    /// Point every execution of an authenticator at a config alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind_authenticator_config: Option<BindDoc>,

    /// Changes to named user-profile attributes.
    ///
    /// The user profile travels as a stringified JSON blob inside a Keycloak
    /// component, so it is parsed, patched and re-serialised rather than merged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_profile: Option<Map<String, Value>>,
}

/// Named fields rather than a tuple, because a JSON pair `["a", "b"]` is two
/// strings nobody can tell apart in a file somebody is editing by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindDoc {
    pub authenticator: String,
    pub config_alias: String,
}

impl AuthPresetDoc {
    /// Every parameter key this preset reads, `auth_type` included.
    pub fn consumes(&self) -> Vec<String> {
        let mut keys = vec![super::presets::PARAM_AUTH_TYPE.to_string()];
        keys.extend(self.required_parameters.iter().cloned());
        keys.extend(self.optional_parameters.iter().cloned());
        keys
    }

    /// The finished realm configuration for one document's parameters.
    pub fn build(&self, input: &PresetInput) -> RealmPatch {
        let fill = |value: &Value| self.substituted(value, input);

        RealmPatch {
            patch: match fill(&Value::Object(self.patch.patch.clone())) {
                Value::Object(map) => map,
                _ => Map::new(),
            },
            bind_authenticator_config: self
                .patch
                .bind_authenticator_config
                .as_ref()
                .map(|bind| {
                    (
                        text_of(&fill(&Value::String(
                            bind.authenticator.clone(),
                        ))),
                        text_of(&fill(&Value::String(
                            bind.config_alias.clone(),
                        ))),
                    )
                }),
            user_profile: self.patch.user_profile.as_ref().and_then(
                |profile| match fill(&Value::Object(profile.clone())) {
                    Value::Object(map) => Some(map),
                    _ => None,
                },
            ),
        }
    }

    /// `{{key}}` replaced throughout, recursively.
    ///
    /// Object *keys* are substituted as well as values: a user-profile patch is
    /// keyed by attribute name, and an attribute name is exactly the kind of
    /// thing a client renames.
    fn substituted(&self, value: &Value, input: &PresetInput) -> Value {
        match value {
            Value::String(text) => Value::String(self.filled(text, input)),
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|item| self.substituted(item, input))
                    .collect(),
            ),
            Value::Object(map) => Value::Object(
                map.iter()
                    .map(|(key, item)| {
                        (self.filled(key, input), self.substituted(item, input))
                    })
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// One string's placeholders filled in.
    ///
    /// An unknown placeholder is left **exactly as written** rather than emptied.
    /// A realm carrying a visible `{{typo_here}}` is a mistake somebody finds by
    /// reading the file; the same realm carrying `""` is a silently broken flow
    /// that looks configured.
    fn filled(&self, text: &str, input: &PresetInput) -> String {
        let mut out = String::with_capacity(text.len());
        let mut rest = text;

        while let Some(start) = rest.find("{{") {
            let Some(end) = rest[start..].find("}}") else {
                break;
            };
            let key = rest[start + 2..start + end].trim();
            out.push_str(&rest[..start]);

            match self.value_of(key, input) {
                Some(found) => out.push_str(&found),
                None => out.push_str(&rest[start..start + end + 2]),
            }
            rest = &rest[start + end + 2..];
        }

        out.push_str(rest);
        out
    }

    /// A parameter, from the document or from this preset's own defaults.
    ///
    /// Empty and absent are the same thing, deliberately: a spreadsheet cell
    /// somebody cleared and one they never filled in are the same intention, and
    /// a realm configured with `""` for an alias is not a realm.
    fn value_of(&self, key: &str, input: &PresetInput) -> Option<String> {
        let given = input.get(key).map(text_of).filter(|text| !text.is_empty());
        given.or_else(|| self.parameter_defaults.get(key).map(text_of))
    }
}

/// A JSON value as the string a Keycloak config field wants.
///
/// Numbers arrive as numbers from a spreadsheet cell and as strings from a form,
/// and Keycloak's config maps are stringly typed either way — so `6` and `"6"`
/// have to reach the realm identically. `to_string()` on a `Value::String` would
/// have written `"\"6\""`.
fn text_of(value: &Value) -> String {
    match value {
        Value::String(text) => text.trim().to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

#[cfg(test)]
#[path = "preset_doc_tests.rs"]
mod preset_doc_tests;

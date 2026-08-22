// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Authentication presets: how voters prove who they are.
//!
//! A source document declares this as one `auth_type` parameter. A preset turns
//! that one cell into the realm configuration it implies — an identity provider,
//! an authenticator config, the user-profile permissions that make a field
//! typeable on the login form.
//!
//! Presets are **patches, not realms.** Two reasons, and the second is the
//! important one:
//!
//! * A realm is ~165 kB of interdependent Keycloak configuration whose client
//!   `redirectUris` and endpoint URLs belong to the environment it was exported
//!   from. Every realm available to copy from is saturated with them.
//! * `keycloak_event_realm` is taken wholesale by the importer: if it is present
//!   it *replaces* the environment's own provisioned default rather than merging
//!   with it. Emitting an invented realm would silently override configuration
//!   that was deployed on purpose.
//!
//! So a preset is applied to a realm supplied as a base export, and is always
//! also written out on its own so that nothing the document asked for is silently
//! dropped.
//!
//! Every value here is transcribed from a realm that works, not invented. The
//! SAML provider relies on `enabledFromMetadata`, which is what lets it carry
//! only a metadata URL: Keycloak fetches the SSO endpoints and the signing
//! certificate from the IdP's metadata, so no certificate is embedded here and
//! none goes stale.

use serde_json::{json, Map, Value};

/// Parameter keys a preset may consume.
///
/// Anything a preset takes is kept out of the "carried but not interpreted"
/// bucket, so the two never contradict each other.
pub const PARAM_AUTH_TYPE: &str = "auth_type";
pub const PARAM_SAML_METADATA_URL: &str = "saml_idp_metadata_url";
pub const PARAM_SAML_IDP_ALIAS: &str = "saml_idp_alias";
pub const PARAM_SAML_PRINCIPAL_ATTRIBUTE: &str = "saml_principal_attribute";
pub const PARAM_OTP_SENDER_ID: &str = "otp_sender_id";
pub const PARAM_OTP_LENGTH: &str = "otp_length";
pub const PARAM_OTP_TTL_SECONDS: &str = "otp_ttl_seconds";

/// The flow an imported SAML identity provider hands first-time logins to.
///
/// Present in the platform's event realm; a preset naming a flow the target realm
/// does not have is reported rather than applied blindly.
pub const SAML_FIRST_BROKER_FLOW: &str = "saml-first-broker-flow";
pub const CERTIFICATE_FIRST_LOGIN_FLOW: &str = "certificate-first-login-flow";

/// `crate::types::keycloak::CERTIFICATES_IDP_ALIAS`.
///
/// Import special-cases this alias: it generates a client secret for the matching
/// client and rewrites the provider's URLs, so the alias is not a free choice.
pub const CERTIFICATES_IDP_ALIAS: &str = "digital-certificates";

/// The Keycloak authenticator each preset needs the target realm to contain.
pub const OTP_AUTHENTICATOR: &str = "message-otp-authenticator";
pub const DEFERRED_AUTHENTICATOR_CONFIG: &str = "deferred";

/// The OTP config a preset registers under.
pub const OTP_CONFIG_ALIAS: &str = "janitor-otp-by-availability";

/// Names the preset that leaves the realm alone, whatever the document declares.
///
/// Useful while a client has not supplied what a preset needs — the SEIU document,
/// for instance, declares SAML but leaves the IdP metadata URL blank pending their
/// identity provider.
pub const NONE: &str = "none";

/// Something a preset needs the target realm to already have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    /// `flow`, `authenticator` or `authenticator_config`.
    pub kind: &'static str,
    pub name: &'static str,
    pub why: &'static str,
}

/// What a preset asks of a realm: a merge, plus two things a merge cannot do.
///
/// The two directives are separate fields rather than magic keys inside the patch
/// — which is how the Python carried them — so that writing the patch out never
/// has to strip them, and so a caller cannot forget to.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RealmPatch {
    /// Merged into the realm, deeply.
    pub patch: Map<String, Value>,

    /// Point every execution of an authenticator at a config alias.
    pub bind_authenticator_config: Option<(String, String)>,

    /// Changes to named user-profile attributes.
    ///
    /// The user profile travels as a stringified JSON blob inside a Keycloak
    /// component, so it has to be parsed, patched and re-serialised rather than
    /// merged.
    pub user_profile: Option<Map<String, Value>>,
}

/// The parameters a preset reads, as resolved from the source document.
#[derive(Debug, Clone, Default)]
pub struct PresetInput {
    values: Vec<(String, Value)>,
}

impl PresetInput {
    pub fn new(values: Vec<(String, Value)>) -> Self {
        PresetInput { values }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.values
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// A parameter as text, or `fallback` when absent or empty.
    fn text(&self, key: &str, fallback: &str) -> String {
        match self.get(key) {
            Some(Value::String(text)) if !text.trim().is_empty() => {
                text.trim().to_string()
            }
            Some(Value::Null) | None => fallback.to_string(),
            Some(Value::String(_)) => fallback.to_string(),
            Some(other) => other.to_string(),
        }
    }
}

/// One way of authenticating voters.
#[derive(Debug, Clone, Copy)]
pub struct AuthPreset {
    pub name: &'static str,
    pub summary: &'static str,

    /// Whether a voter needs an email address or mobile number to log in.
    ///
    /// This is the difference between "56 of 56 voters cannot be sent a one-time
    /// code" being a real problem and being noise: under SAML the client's
    /// identity provider authenticates the voter, and asking for contact details
    /// is not this tool's business.
    pub uses_otp: bool,

    pub requires: &'static [Requirement],
    pub required_parameters: &'static [&'static str],
    pub optional_parameters: &'static [&'static str],

    /// User-profile attributes this preset expects the realm to declare.
    ///
    /// Named statically rather than derived by calling `build` with a dummy input,
    /// because a caller wants this *before* deciding anything — the census's column
    /// chooser offers exactly these, so a column somebody adds is a column the
    /// sign-in flow can actually read.
    ///
    /// The everyday five (`username`, `email`, `first_name`, `last_name`, plus the
    /// area) are not listed: they are the platform's own, present in every realm, and
    /// repeating them per preset would be four chances to forget one.
    pub profile_attributes: &'static [&'static str],

    build: fn(&PresetInput) -> RealmPatch,
}

impl AuthPreset {
    /// Turn the resolved parameters into what the realm needs.
    pub fn build(&self, input: &PresetInput) -> RealmPatch {
        (self.build)(input)
    }

    /// Every parameter key this preset reads.
    pub fn consumes(&self) -> Vec<&'static str> {
        let mut keys = vec![PARAM_AUTH_TYPE];
        keys.extend(self.required_parameters);
        keys.extend(self.optional_parameters);
        keys
    }
}

// -- saml_sso_idp_initiated ------------------------------------------------

/// A SAML identity provider driven entirely by the IdP's metadata.
///
/// `enabledFromMetadata` is the whole reason this preset is portable: Keycloak
/// reads `singleSignOnServiceUrl`, `singleLogoutServiceUrl`, the entity id and the
/// signing certificate out of the document at `metadataDescriptorUrl`. So nothing
/// environment-specific and nothing that expires is written here.
///
/// For IdP-initiated SSO the client's IdP posts an unsolicited assertion to
/// Keycloak's broker endpoint, which is derived from the alias — which is why the
/// alias is worth being able to set.
fn saml_patch(input: &PresetInput) -> RealmPatch {
    let alias = input.text(PARAM_SAML_IDP_ALIAS, "client-saml-idp");
    let metadata_url = input.text(PARAM_SAML_METADATA_URL, "");
    let principal_attribute =
        input.text(PARAM_SAML_PRINCIPAL_ATTRIBUTE, "username");

    let patch = json!({
        "identityProviders": [{
            "alias": alias,
            "displayName": "Sign in with your organisation",
            "providerId": "saml",
            "enabled": true,
            "trustEmail": false,
            "storeToken": false,
            "addReadTokenRoleOnCreate": false,
            // SP-initiated redirect stays off: the voter arrives from the IdP.
            "authenticateByDefault": false,
            "linkOnly": false,
            "updateProfileFirstLoginMode": "off",
            "firstBrokerLoginFlowAlias": SAML_FIRST_BROKER_FLOW,
            "config": {
                "metadataDescriptorUrl": metadata_url,
                // Endpoints and signing certificate come from the metadata.
                "enabledFromMetadata": "true",
                "validateSignature": "true",
                "wantAssertionsSigned": "true",
                "wantAssertionsEncrypted": "false",
                "wantAuthnRequestsSigned": "false",
                "signSpMetadata": "false",
                "forceAuthn": "false",
                "postBindingResponse": "true",
                "postBindingAuthnRequest": "true",
                "postBindingLogout": "true",
                "backchannelSupported": "false",
                // Match the assertion to an existing census voter rather than
                // creating one: the census is the roll of who may vote.
                "principalType": "ATTRIBUTE",
                "principalAttribute": principal_attribute,
                "nameIDPolicyFormat":
                    "urn:oasis:names:tc:SAML:2.0:nameid-format:transient",
                "syncMode": "IMPORT",
                "allowCreate": "false",
                "hideOnLoginPage": "false",
                "authnContextComparisonType": "exact",
                "allowedClockSkew": "30",
            },
        }],
        // The IdP is the authority on who the voter is, so the realm must not
        // offer a local password or a self-service registration path.
        "registrationAllowed": false,
        "resetPasswordAllowed": false,
        "loginWithEmailAllowed": false,
        "rememberMe": false,
    });

    RealmPatch {
        patch: object(patch),
        ..RealmPatch::default()
    }
}

// -- otp_email_or_sms ------------------------------------------------------

/// One OTP step that follows whatever the voter actually has.
///
/// `messageCourierAttribute: BOTH` is availability-driven rather than a demand for
/// both channels: `Utils.sendCode` guards each channel on a non-empty address, so
/// an email-only voter gets email, a mobile-only voter gets SMS, and a voter with
/// both gets both.
fn otp_config(input: &PresetInput) -> Value {
    json!({
        "alias": OTP_CONFIG_ALIAS,
        "config": {
            "messageCourierAttribute": "BOTH",
            "telUserAttribute": "sequent.read-only.mobile-number",
            "deferredUserAttribute": "false",
            "one-time-link": "false",
            "length": input.text(PARAM_OTP_LENGTH, "6"),
            "ttl": input.text(PARAM_OTP_TTL_SECONDS, "900"),
            "resendCoudActivationTimer": "60",
            "max-receiver-reuse": "1",
            "senderId": input.text(PARAM_OTP_SENDER_ID, "Sequent"),
            "test-mode": "false",
        },
    })
}

fn otp_patch(input: &PresetInput) -> RealmPatch {
    let patch = json!({
        "authenticatorConfig": [otp_config(input)],
        "registrationAllowed": true,
        "registrationEmailAsUsername": false,
        "loginWithEmailAllowed": false,
        // There is no password in this flow, so a "forgot password" link is a
        // dead end.
        "resetPasswordAllowed": false,
        "bruteForceProtected": true,
    });

    RealmPatch {
        patch: object(patch),
        bind_authenticator_config: Some((
            OTP_AUTHENTICATOR.to_string(),
            OTP_CONFIG_ALIAS.to_string(),
        )),
        user_profile: None,
    }
}

// -- voter_link_plus_dob ---------------------------------------------------

/// Member id prefilled from the link, date of birth typed, then OTP.
///
/// The date of birth is deliberately **not** prefillable. If it were, the link
/// alone would authenticate the voter, and links get forwarded.
fn voter_link_dob_patch(input: &PresetInput) -> RealmPatch {
    let mut result = otp_patch(input);

    let deferred = json!({
        "alias": DEFERRED_AUTHENTICATOR_CONFIG,
        "config": {
            "form-mode": "LOGIN",
            "password-required": "false",
            "search-attributes": "username,dateOfBirth",
            "hidden-profile-attributes": "locale,firstName,lastName,ssn4",
            "prefill-parameters-policy": "ACCEPT",
            "user-status": "sequent.read-only.id-card-number-validated",
            "password-expiration-user-attribute":
                "sequent.read-only.expirationDate",
        },
    });

    if let Some(Value::Array(configs)) =
        result.patch.get_mut("authenticatorConfig")
    {
        configs.push(deferred);
    }

    result.user_profile = Some(object(json!({
        "username": {
            "permissions": {"view": ["admin", "user"], "edit": ["admin", "user"]},
            "annotations": {"loginHintPrefillPolicy": "READ_ONLY"},
        },
        "dateOfBirth": {
            "permissions": {"view": ["admin", "user"], "edit": ["admin", "user"]},
            "required": {"roles": ["user"]},
            // IGNORE on purpose: a prefillable date of birth means the link alone
            // authenticates the voter.
            "annotations": {
                "inputType": "html5-date",
                "loginHintPrefillPolicy": "IGNORE",
            },
        },
    })));
    result
}

// -- digital_certificates --------------------------------------------------

/// Enable the digital-certificates provider the platform already knows.
///
/// Only the alias and the enabled flag are set. Import rewrites this provider's
/// URLs and generates its client secret itself — `import_election_event.rs`
/// special-cases exactly this alias — so writing endpoints here would be
/// overwritten at best and wrong at worst.
fn certificates_patch(_: &PresetInput) -> RealmPatch {
    let patch = json!({
        "identityProviders": [{
            "alias": CERTIFICATES_IDP_ALIAS,
            "providerId": "keycloak-oidc",
            "displayName": "Digital Certificates",
            "enabled": true,
            "trustEmail": false,
            "storeToken": false,
            "addReadTokenRoleOnCreate": false,
            "authenticateByDefault": false,
            "linkOnly": false,
            "updateProfileFirstLoginMode": "off",
            "firstBrokerLoginFlowAlias": CERTIFICATE_FIRST_LOGIN_FLOW,
        }],
        "registrationAllowed": false,
        "resetPasswordAllowed": false,
    });

    RealmPatch {
        patch: object(patch),
        ..RealmPatch::default()
    }
}

/// Every preset, in the order a caller should see them listed.
pub const PRESETS: &[AuthPreset] = &[
    AuthPreset {
        name: "digital_certificates",
        summary: "The voter presents a digital certificate, brokered through the \
                  platform's digital-certificates provider.",
        uses_otp: false,
        build: certificates_patch,
        required_parameters: &[],
        optional_parameters: &[],
        // The certificate carries the identity; nothing extra is read off the voter.
        profile_attributes: &[],
        requires: &[Requirement {
            kind: "flow",
            name: CERTIFICATE_FIRST_LOGIN_FLOW,
            why: "a first-time certificate login is handed to this flow",
        }],
    },
    AuthPreset {
        name: "otp_email_or_sms",
        summary: "The voter types their member id, then a one-time code sent to \
                  whichever of email and mobile the census holds for them.",
        uses_otp: true,
        build: otp_patch,
        required_parameters: &[],
        optional_parameters: &[
            PARAM_OTP_SENDER_ID,
            PARAM_OTP_LENGTH,
            PARAM_OTP_TTL_SECONDS,
        ],
        // The code goes to whichever of these the census holds, so both are read.
        profile_attributes: &["email", "mobile"],
        requires: &[Requirement {
            kind: "authenticator",
            name: OTP_AUTHENTICATOR,
            why: "the one-time code step is bound to this authenticator's config",
        }],
    },
    AuthPreset {
        name: "saml_sso_idp_initiated",
        summary: "The client's SAML identity provider authenticates the voter and \
                  posts an assertion to Keycloak. Voters need no email address or \
                  mobile number.",
        uses_otp: false,
        build: saml_patch,
        // The identity provider authenticates; the assertion is matched against a
        // census voter by the principal attribute, which is a *parameter* rather than
        // a fixed column.
        profile_attributes: &[],
        required_parameters: &[PARAM_SAML_METADATA_URL],
        optional_parameters: &[
            PARAM_SAML_IDP_ALIAS,
            PARAM_SAML_PRINCIPAL_ATTRIBUTE,
        ],
        requires: &[Requirement {
            kind: "flow",
            name: SAML_FIRST_BROKER_FLOW,
            why: "a first-time SAML login is handed to this flow to match the \
                  assertion against an existing census voter",
        }],
    },
    AuthPreset {
        name: "voter_link_plus_dob",
        summary: "A voter-specific link carries the member id read-only; the voter \
                  types their date of birth, then a one-time code.",
        uses_otp: true,
        build: voter_link_dob_patch,
        // What `voter_link_dob_patch` configures, and what its `search-attributes`
        // and `hidden-profile-attributes` name. A census for this preset without a
        // `dateOfBirth` column is a census nobody can log in with.
        profile_attributes: &["dateOfBirth", "mobile"],
        required_parameters: &[],
        optional_parameters: &[
            PARAM_OTP_SENDER_ID,
            PARAM_OTP_LENGTH,
            PARAM_OTP_TTL_SECONDS,
        ],
        requires: &[
            Requirement {
                kind: "authenticator",
                name: OTP_AUTHENTICATOR,
                why: "the one-time code step is bound to this authenticator's \
                      config",
            },
            Requirement {
                kind: "authenticator_config",
                name: DEFERRED_AUTHENTICATOR_CONFIG,
                why: "the login form's fields and prefill policy are set on this \
                      config",
            },
        ],
    },
];

/// The preset named, whatever the case and padding.
pub fn get(name: &str) -> Option<&'static AuthPreset> {
    let name = name.trim().to_lowercase();
    PRESETS.iter().find(|preset| preset.name == name)
}

/// Every preset name, in declaration order.
pub fn names() -> Vec<&'static str> {
    PRESETS.iter().map(|preset| preset.name).collect()
}

/// Keys the presets may consume.
///
/// A caller keeps these out of the "carried but not interpreted" bucket even when
/// no preset is selected: reporting a key as uninterpreted while a preset would
/// have acted on it contradicts itself.
pub fn all_preset_parameters() -> Vec<&'static str> {
    let mut keys: Vec<&'static str> =
        PRESETS.iter().flat_map(AuthPreset::consumes).collect();
    keys.sort_unstable();
    keys.dedup();
    keys
}

/// A `json!` object as a `Map`. The macro's input is an object literal in every
/// call above, so the panic is unreachable.
fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(object) => object,
        _ => unreachable!("preset patches are written as object literals"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(values: &[(&str, &str)]) -> PresetInput {
        PresetInput::new(
            values
                .iter()
                .map(|(key, value)| ((*key).to_string(), json!(*value)))
                .collect(),
        )
    }

    #[test]
    fn the_presets_are_listed_in_a_stable_order() {
        // A CLI's --help and an SPA's dropdown both read this.
        assert_eq!(
            names(),
            [
                "digital_certificates",
                "otp_email_or_sms",
                "saml_sso_idp_initiated",
                "voter_link_plus_dob",
            ]
        );
    }

    #[test]
    fn a_preset_is_found_however_it_is_written() {
        assert_eq!(get("otp_email_or_sms").unwrap().name, "otp_email_or_sms");
        assert_eq!(
            get("  OTP_Email_Or_SMS ").unwrap().name,
            "otp_email_or_sms"
        );
        assert!(get("otp").is_none());
        assert!(get(NONE).is_none());
    }

    #[test]
    fn every_preset_declares_what_the_realm_must_already_have() {
        // A preset naming a flow the target realm lacks is reported rather than
        // applied blindly, which is only possible if it says what it needs.
        for preset in PRESETS {
            assert!(!preset.requires.is_empty(), "{}", preset.name);
            for requirement in preset.requires {
                assert!(
                    ["flow", "authenticator", "authenticator_config"]
                        .contains(&requirement.kind),
                    "{}: {}",
                    preset.name,
                    requirement.kind
                );
                assert!(!requirement.why.is_empty());
            }
        }
    }

    #[test]
    fn every_preset_consumes_the_auth_type_that_selects_it() {
        for preset in PRESETS {
            assert!(preset.consumes().contains(&PARAM_AUTH_TYPE));
        }
        assert!(all_preset_parameters().contains(&PARAM_SAML_METADATA_URL));
        assert!(all_preset_parameters().contains(&PARAM_OTP_LENGTH));
    }

    #[test]
    fn the_saml_provider_carries_only_a_metadata_url() {
        // enabledFromMetadata is what makes it portable: no endpoint and no
        // certificate is written, so nothing here goes stale or belongs to one
        // environment.
        let patch = get("saml_sso_idp_initiated").unwrap().build(&input(&[(
            PARAM_SAML_METADATA_URL,
            "https://idp/metadata",
        )]));
        let provider = &patch.patch["identityProviders"][0];

        assert_eq!(provider["alias"], json!("client-saml-idp"));
        assert_eq!(
            provider["config"]["metadataDescriptorUrl"],
            json!("https://idp/metadata")
        );
        assert_eq!(provider["config"]["enabledFromMetadata"], json!("true"));
        assert!(provider["config"].get("singleSignOnServiceUrl").is_none());
        assert!(provider["config"].get("signingCertificate").is_none());
    }

    #[test]
    fn the_saml_assertion_is_matched_to_a_census_voter_not_used_to_create_one()
    {
        // The census is the roll of who may vote.
        let patch = get("saml_sso_idp_initiated").unwrap().build(&input(&[(
            PARAM_SAML_METADATA_URL,
            "https://idp/metadata",
        )]));
        let config = &patch.patch["identityProviders"][0]["config"];
        assert_eq!(config["principalType"], json!("ATTRIBUTE"));
        assert_eq!(config["principalAttribute"], json!("username"));
        assert_eq!(config["allowCreate"], json!("false"));
    }

    #[test]
    fn saml_leaves_no_local_password_or_registration_path() {
        // The IdP is the authority on who the voter is.
        let patch = get("saml_sso_idp_initiated").unwrap().build(&input(&[(
            PARAM_SAML_METADATA_URL,
            "https://idp/metadata",
        )]));
        assert_eq!(patch.patch["registrationAllowed"], json!(false));
        assert_eq!(patch.patch["resetPasswordAllowed"], json!(false));
        assert_eq!(patch.patch["loginWithEmailAllowed"], json!(false));
    }

    #[test]
    fn the_saml_alias_and_principal_attribute_can_be_set() {
        // The broker endpoint the IdP posts to is derived from the alias.
        let patch = get("saml_sso_idp_initiated").unwrap().build(&input(&[
            (PARAM_SAML_METADATA_URL, "https://idp/metadata"),
            (PARAM_SAML_IDP_ALIAS, "acme-idp"),
            (PARAM_SAML_PRINCIPAL_ATTRIBUTE, "employeeNumber"),
        ]));
        let provider = &patch.patch["identityProviders"][0];
        assert_eq!(provider["alias"], json!("acme-idp"));
        assert_eq!(
            provider["config"]["principalAttribute"],
            json!("employeeNumber")
        );
    }

    #[test]
    fn the_otp_step_follows_whatever_the_voter_actually_has() {
        // BOTH is availability-driven, not a demand for both channels:
        // Utils.sendCode guards each on a non-empty address.
        let patch = get("otp_email_or_sms").unwrap().build(&input(&[]));
        let config = &patch.patch["authenticatorConfig"][0]["config"];
        assert_eq!(config["messageCourierAttribute"], json!("BOTH"));
        assert_eq!(config["length"], json!("6"));
        assert_eq!(config["ttl"], json!("900"));
        assert_eq!(config["senderId"], json!("Sequent"));
    }

    #[test]
    fn the_otp_config_is_bound_to_the_authenticator_that_runs_it() {
        // Registering the config without binding it leaves the step unconfigured.
        let patch = get("otp_email_or_sms").unwrap().build(&input(&[]));
        assert_eq!(
            patch.bind_authenticator_config,
            Some((OTP_AUTHENTICATOR.to_string(), OTP_CONFIG_ALIAS.to_string()))
        );
    }

    #[test]
    fn the_otp_length_and_lifetime_can_be_set_and_arrive_as_strings() {
        // Keycloak config values are strings, whatever a spreadsheet cell was.
        let patch =
            get("otp_email_or_sms")
                .unwrap()
                .build(&PresetInput::new(vec![
                    (PARAM_OTP_LENGTH.to_string(), json!(8)),
                    (PARAM_OTP_TTL_SECONDS.to_string(), json!(300)),
                    (PARAM_OTP_SENDER_ID.to_string(), json!("SEIU")),
                ]));
        let config = &patch.patch["authenticatorConfig"][0]["config"];
        assert_eq!(config["length"], json!("8"));
        assert_eq!(config["ttl"], json!("300"));
        assert_eq!(config["senderId"], json!("SEIU"));
    }

    #[test]
    fn there_is_no_forgotten_password_link_where_there_is_no_password() {
        let patch = get("otp_email_or_sms").unwrap().build(&input(&[]));
        assert_eq!(patch.patch["resetPasswordAllowed"], json!(false));
        assert_eq!(patch.patch["bruteForceProtected"], json!(true));
    }

    #[test]
    fn the_link_preset_adds_a_login_form_on_top_of_the_otp_step() {
        let patch = get("voter_link_plus_dob").unwrap().build(&input(&[]));
        let configs = patch.patch["authenticatorConfig"].as_array().unwrap();
        assert_eq!(configs.len(), 2);
        assert_eq!(configs[0]["alias"], json!(OTP_CONFIG_ALIAS));
        assert_eq!(configs[1]["alias"], json!(DEFERRED_AUTHENTICATOR_CONFIG));
        assert_eq!(
            configs[1]["config"]["search-attributes"],
            json!("username,dateOfBirth")
        );
    }

    #[test]
    fn a_date_of_birth_is_never_prefillable_from_the_link() {
        // If it were, the link alone would authenticate the voter — and links get
        // forwarded.
        let patch = get("voter_link_plus_dob").unwrap().build(&input(&[]));
        let profile = patch.user_profile.expect("a user profile patch");
        assert_eq!(
            profile["username"]["annotations"]["loginHintPrefillPolicy"],
            json!("READ_ONLY")
        );
        assert_eq!(
            profile["dateOfBirth"]["annotations"]["loginHintPrefillPolicy"],
            json!("IGNORE")
        );
        assert_eq!(
            profile["dateOfBirth"]["required"]["roles"],
            json!(["user"])
        );
    }

    #[test]
    fn the_certificates_preset_sets_the_alias_import_special_cases_and_no_urls()
    {
        // Import rewrites this provider's URLs and generates its client secret, so
        // writing endpoints would be overwritten at best.
        let patch = get("digital_certificates").unwrap().build(&input(&[]));
        let provider = &patch.patch["identityProviders"][0];
        assert_eq!(provider["alias"], json!(CERTIFICATES_IDP_ALIAS));
        assert_eq!(provider["providerId"], json!("keycloak-oidc"));
        assert_eq!(provider["enabled"], json!(true));
        assert!(provider.get("config").is_none());
    }

    #[test]
    fn only_the_presets_that_send_a_code_say_they_use_otp() {
        // What decides whether "no voter can be sent a code" is a real problem.
        assert!(get("otp_email_or_sms").unwrap().uses_otp);
        assert!(get("voter_link_plus_dob").unwrap().uses_otp);
        assert!(!get("saml_sso_idp_initiated").unwrap().uses_otp);
        assert!(!get("digital_certificates").unwrap().uses_otp);
    }

    #[test]
    fn no_preset_writes_an_environment_specific_url() {
        // The reason presets are patches: every realm available to copy from is
        // saturated with the hosts of the environment it came from.
        for preset in PRESETS {
            let built = preset.build(&input(&[(
                PARAM_SAML_METADATA_URL,
                "https://idp.example.org/metadata",
            )]));
            let encoded = serde_json::to_string(&built.patch).unwrap();
            for host in [
                "localhost",
                "127.0.0.1",
                ".sequentech.io",
                "https://sequent",
            ] {
                assert!(
                    !encoded.contains(host),
                    "{} writes {host}",
                    preset.name
                );
            }
        }
    }
}

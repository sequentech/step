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
use strum_macros::{Display, EnumString};

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
pub const PARAM_SCANOVATE_OCR_URL: &str = "scanovate_ocr_url";
pub const PARAM_SCANOVATE_LIVENESS_URL: &str = "scanovate_liveness_url";
pub const PARAM_SCANOVATE_FACE_MATCH_URL: &str = "scanovate_face_match_url";
pub const PARAM_SCANOVATE_FACE_MATCH_MIN_SIMILARITY: &str =
    "scanovate_face_match_min_similarity";
/// ISO 3166 alpha-3 codes separated by `|`, as the regex alternation they
/// become: a passport issued anywhere else is refused.
pub const PARAM_ACCEPTED_PASSPORT_COUNTRIES: &str =
    "accepted_passport_countries";

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

/// Shipped only by the `ovcs` branch's Keycloak image.
pub const SCANOVATE_AUTHENTICATOR: &str = "scanovate-authenticator";
pub const CONDITIONAL_AUTH_NOTE: &str = "conditional-auth-note";
pub const DENY_ACCESS_AUTHENTICATOR: &str = "deny-access-authenticator";
pub const LOOKUP_AND_UPDATE_USER: &str = "lookup-and-update-user";

pub const BIOMETRIC_REGISTRATION_FLOW: &str = "biometric-registration";
pub const BIOMETRIC_REGISTRATION_FORM_FLOW: &str =
    "biometric-registration registration form";
pub const PASSPORT_COUNTRY_CHECK_FLOW: &str = "passport-country-check";

/// The alias a deployed realm already keeps its liveness secret under, so the
/// merge by alias leaves that secret in place.
pub const SCANOVATE_CONFIG_ALIAS: &str = "scanovate-registration";
pub const SCANOVATE_LIVENESS_SECRET: &str = "liveness-secret";
pub const SCANOVATE_LIVENESS_URL: &str = "liveness-url";
pub const SCANOVATE_OCR_URL_DEFAULT: &str = "http://scanovate-ocr:5040";
pub const SCANOVATE_FACE_MATCH_URL_DEFAULT: &str =
    "http://scanovate-face-match:3000";
pub const BIOMETRIC_OTP_CONFIG: &str = "biometric-registration-otp";
pub const BIOMETRIC_USER_CREATION_CONFIG: &str = "biometric-registration-user";
pub const BIOMETRIC_LOOKUP_CONFIG: &str = "biometric-lookup-register";
pub const PASSPORT_COUNTRY_CONDITION_CONFIG: &str =
    "passport-country-not-accepted";
pub const PASSPORT_COUNTRY_DENIED_CONFIG: &str = "passport-country-denied";

/// The auth note the Scanovate step stores the passport's issuing country in.
const ISSUING_COUNTRY_NOTE: &str = "issuingCountry";
const PASSPORT_COUNTRY_NOT_ACCEPTED: &str = "passportCountryNotAccepted";

/// Names the preset that leaves the realm alone, whatever the document declares.
///
/// Useful while a client has not supplied what a preset needs — the SEIU document,
/// for instance, declares SAML but leaves the IdP metadata URL blank pending their
/// identity provider.
pub const NONE: &str = "none";

/// Which collection of the realm a requirement is about.
///
/// A `&str` before, matched with a `_ =>` arm that meant "authenticator" — so a
/// misspelled kind was checked against the wrong collection and reported nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString)]
pub enum RequirementKind {
    #[strum(serialize = "flow")]
    Flow,
    #[strum(serialize = "authenticator")]
    Authenticator,
    #[strum(serialize = "authenticator_config")]
    AuthenticatorConfig,
}

/// Something a preset needs the target realm to already have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requirement {
    pub kind: RequirementKind,
    pub name: &'static str,
    pub why: &'static str,
}

/// An authenticator config value a preset needs and must never write.
///
/// Set in the target realm by whoever deploys it. The build reports a realm that
/// lacks it, rather than carrying the secret through a plan or a bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfigSecret {
    pub config_alias: &'static str,
    pub key: &'static str,
}

/// An authenticator config value that depends on where the realm is deployed.
///
/// Written from `parameter` when the plan gives one; otherwise the target realm's
/// own value is kept, and the build reports a realm that has none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeploymentSetting {
    pub config_alias: &'static str,
    pub key: &'static str,
    pub parameter: &'static str,
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

    pub secrets: &'static [ConfigSecret],
    pub deployment_settings: &'static [DeploymentSetting],

    build: fn(&PresetInput) -> RealmPatch,
}

impl AuthPreset {
    /// Turn the resolved parameters into what the realm needs.
    pub fn build(&self, input: &PresetInput) -> RealmPatch {
        let mut result = (self.build)(input);
        for setting in self.deployment_settings {
            let value = input.text(setting.parameter, "");
            if value.is_empty() {
                continue;
            }
            let configs = result
                .patch
                .get_mut("authenticatorConfig")
                .and_then(Value::as_array_mut)
                .into_iter()
                .flatten()
                .filter(|config| {
                    config.get("alias").and_then(Value::as_str)
                        == Some(setting.config_alias)
                });
            for config in configs {
                if let Some(entries) =
                    config.get_mut("config").and_then(Value::as_object_mut)
                {
                    entries.insert(
                        setting.key.to_string(),
                        Value::String(value.clone()),
                    );
                }
            }
        }
        result
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

// -- biometric_authentication ----------------------------------------------

/// One execution of a flow, spelled the way a working realm export spells it.
///
/// `autheticatorFlow` is Keycloak's own legacy misspelling, still written by its
/// exporter beside the correct key.
fn execution(
    step: &str,
    requirement: &str,
    priority: u32,
    config: Option<&str>,
) -> Value {
    let mut execution = json!({
        "authenticator": step,
        "authenticatorFlow": false,
        "autheticatorFlow": false,
        "priority": priority,
        "requirement": requirement,
        "userSetupAllowed": false,
    });
    if let (Some(config), Some(execution)) = (config, execution.as_object_mut())
    {
        execution.insert("authenticatorConfig".to_string(), json!(config));
    }
    execution
}

fn subflow(alias: &str, requirement: &str, priority: u32) -> Value {
    json!({
        "authenticatorFlow": true,
        "autheticatorFlow": true,
        "flowAlias": alias,
        "priority": priority,
        "requirement": requirement,
        "userSetupAllowed": false,
    })
}

/// Self-registration verified by Scanovate: an enrollment form, a one-time code,
/// a passport read and matched to the voter's face, and a census lookup.
///
/// Transcribed from the registration flow running on OVCS. A passport issued
/// outside the accepted countries is refused by a conditional subflow rather than
/// a Scanovate validation rule, so the voter is told why. The liveness secret is
/// left out: see [`ConfigSecret`].
fn biometric_patch(input: &PresetInput) -> RealmPatch {
    let countries =
        input.text(PARAM_ACCEPTED_PASSPORT_COUNTRIES, "ESP|PHL|ISR|NLD");
    let similarity =
        input.text(PARAM_SCANOVATE_FACE_MATCH_MIN_SIMILARITY, "0.67");

    // Keys in byte order: these are serialized into strings, and serde_json
    // keeps insertion order only when a crate in the build enables
    // `preserve_order` (windmill does, the WASM build does not). Sorted, both
    // write the same bytes.
    let validate = json!({
        "philippinePassport": [{
            "attributePath": "/date_of_expiry",
            "errorMsg": "scanovateAttributesError",
            "isBeforeDateValue": "now",
            "process": "ocr",
            "sourceDateFormat": "yyyy-MM-dd",
            "type": "isBeforeDateValue",
        }],
    });
    let store = json!({
        "philippinePassport": [
            {
                "UserAttribute": "firstName",
                "attributePath": "/first_name_english",
                "process": "ocr",
                "type": "text",
            },
            {
                "UserAttribute": "lastName",
                "attributePath": "/last_name_english",
                "process": "ocr",
                "type": "text",
            },
            {
                "UserAttribute": "sequent.read-only.id-card-number",
                "attributePath": "/document_number",
                "process": "ocr",
                "type": "text",
            },
            {
                "UserAttribute": "dateOfBirth",
                "attributePath": "/date_of_birth",
                "process": "ocr",
                "sourceDateFormat": "yyyy-MM-dd",
                "storeDateFormat": "yyyy-MM-dd",
                "type": "date",
            },
            {
                "UserAttribute": ISSUING_COUNTRY_NOTE,
                "attributePath": "/issuing_country_code",
                "process": "ocr",
                "type": "text",
            },
        ],
    });

    let patch = json!({
        "registrationFlow": BIOMETRIC_REGISTRATION_FLOW,
        "loginTheme": "sequent-ui-voting",
        "authenticationFlows": [
            {
                "alias": BIOMETRIC_REGISTRATION_FLOW,
                "description": "registration flow",
                "providerId": "basic-flow",
                "topLevel": true,
                "builtIn": false,
                "authenticationExecutions": [
                    {
                        "authenticator": "registration-page-form",
                        "authenticatorFlow": true,
                        "autheticatorFlow": true,
                        "flowAlias": BIOMETRIC_REGISTRATION_FORM_FLOW,
                        "priority": 10,
                        "requirement": "REQUIRED",
                        "userSetupAllowed": false,
                    },
                    execution(
                        OTP_AUTHENTICATOR,
                        "REQUIRED",
                        17,
                        Some(BIOMETRIC_OTP_CONFIG),
                    ),
                    execution(
                        SCANOVATE_AUTHENTICATOR,
                        "REQUIRED",
                        18,
                        Some(SCANOVATE_CONFIG_ALIAS),
                    ),
                    subflow(PASSPORT_COUNTRY_CHECK_FLOW, "CONDITIONAL", 19),
                    execution(
                        LOOKUP_AND_UPDATE_USER,
                        "REQUIRED",
                        20,
                        Some(BIOMETRIC_LOOKUP_CONFIG),
                    ),
                ],
            },
            {
                "alias": BIOMETRIC_REGISTRATION_FORM_FLOW,
                "description": "registration form",
                "providerId": "form-flow",
                "topLevel": false,
                "builtIn": false,
                "authenticationExecutions": [
                    execution("enrollment-window-check", "REQUIRED", 0, None),
                    execution(
                        "deferred-registration-user-creation",
                        "REQUIRED",
                        10,
                        Some(BIOMETRIC_USER_CREATION_CONFIG),
                    ),
                ],
            },
            {
                "alias": PASSPORT_COUNTRY_CHECK_FLOW,
                "description": "refuse a passport issued outside the accepted countries",
                "providerId": "basic-flow",
                "topLevel": false,
                "builtIn": false,
                "authenticationExecutions": [
                    execution(
                        CONDITIONAL_AUTH_NOTE,
                        "REQUIRED",
                        0,
                        Some(PASSPORT_COUNTRY_CONDITION_CONFIG),
                    ),
                    execution(
                        DENY_ACCESS_AUTHENTICATOR,
                        "REQUIRED",
                        10,
                        Some(PASSPORT_COUNTRY_DENIED_CONFIG),
                    ),
                ],
            },
        ],
        "authenticatorConfig": [
            {
                "alias": BIOMETRIC_OTP_CONFIG,
                "config": {
                    "code-progress-policy": "SHOW",
                    "messageCourierAttribute": "CHOSEN",
                    "telUserAttribute": "sequent.read-only.mobile-number",
                    // The voter does not exist yet: the code goes to the address
                    // typed on the enrollment form.
                    "deferredUserAttribute": "true",
                    "one-time-link": "false",
                    "length": input.text(PARAM_OTP_LENGTH, "6"),
                    "ttl": input.text(PARAM_OTP_TTL_SECONDS, "300"),
                    "resendCoudActivationTimer": "60",
                    "senderId": input.text(PARAM_OTP_SENDER_ID, "Sequent"),
                },
            },
            {
                "alias": SCANOVATE_CONFIG_ALIAS,
                "config": {
                    "ocr-url": input.text(
                        PARAM_SCANOVATE_OCR_URL,
                        SCANOVATE_OCR_URL_DEFAULT,
                    ),
                    "ocr-types": "{\"philippinePassport\": \"passport\"}",
                    "capture-sides": "{\"philippinePassport\": [\"front\"]}",
                    "face-match-url": input.text(
                        PARAM_SCANOVATE_FACE_MATCH_URL,
                        SCANOVATE_FACE_MATCH_URL_DEFAULT,
                    ),
                    "face-match-min-similarity":
                        format!("{{\"philippinePassport\": {similarity}}}"),
                    "doc-id-type": "sequent.read-only.id-card-type",
                    "user-status": "sequent.read-only.id-card-number-validated",
                    "attributes-to-validate": validate.to_string(),
                    "attributes-to-store": store.to_string(),
                    "max-attempts": "3",
                },
            },
            {
                "alias": BIOMETRIC_USER_CREATION_CONFIG,
                "config": {
                    "user-status": "sequent.read-only.id-card-number-validated",
                    "channel-choice-policy": "OTP_CHANNELS",
                },
            },
            {
                "alias": BIOMETRIC_LOOKUP_CONFIG,
                "config": {
                    "auto-login": "false",
                    "auto-2fa": "true",
                    "messageCourierAttribute": "CHOSEN",
                    "telUserAttribute": "sequent.read-only.mobile-number",
                    "search-attributes": "firstName,middleName,lastName,dateOfBirth",
                    "update-attributes":
                        "sequent.read-only.id-card-number-validated,email,\
                         sequent.read-only.mobile-number,\
                         sequent.read-only.id-card-number,\
                         sequent.read-only.id-card-type",
                    "unset-attributes":
                        "email,sequent.read-only.mobile-number,\
                         sequent.read-only.id-card-number-validated",
                    // A voter the census does not match is held for an
                    // administrator rather than refused.
                    "no-matching-voter-policy": "PENDING_APPROVAL",
                    "reply-by-hours": "72",
                },
            },
            {
                "alias": PASSPORT_COUNTRY_CONDITION_CONFIG,
                "config": {
                    "conditional-auth-note-key": ISSUING_COUNTRY_NOTE,
                    "conditional-auth-note-value":
                        format!(r"(?i)\s*({countries})\s*"),
                    "value-is-regex": "true",
                    "negate": "true",
                },
            },
            {
                "alias": PASSPORT_COUNTRY_DENIED_CONFIG,
                "config": {"denyErrorMessage": PASSPORT_COUNTRY_NOT_ACCEPTED},
            },
        ],
        "localizationTexts": {
            "en": {
                "philippinePassport": "Passport",
                ISSUING_COUNTRY_NOTE: "Issuing country",
                PASSPORT_COUNTRY_NOT_ACCEPTED:
                    "Passports issued by this country can't be used to enroll.",
            },
            "tl": {
                "philippinePassport": "Pasaporte",
                ISSUING_COUNTRY_NOTE: "Bansang nag-isyu",
                PASSPORT_COUNTRY_NOT_ACCEPTED:
                    "Hindi maaaring gamitin sa pagpapatala ang pasaporteng \
                     inisyu ng bansang ito.",
            },
        },
    });

    RealmPatch {
        patch: object(patch),
        ..RealmPatch::default()
    }
}

/// Every preset, in the order a caller should see them listed.
pub const PRESETS: &[AuthPreset] = &[
    AuthPreset {
        name: "biometric_authentication",
        summary: "Biometric Authentication: the voter enrolls with their passport \
                  and a live selfie verified by Scanovate, after a one-time code, \
                  and is matched against the census.",
        uses_otp: true,
        build: biometric_patch,
        required_parameters: &[],
        optional_parameters: &[
            PARAM_SCANOVATE_OCR_URL,
            PARAM_SCANOVATE_LIVENESS_URL,
            PARAM_SCANOVATE_FACE_MATCH_URL,
            PARAM_ACCEPTED_PASSPORT_COUNTRIES,
            PARAM_SCANOVATE_FACE_MATCH_MIN_SIMILARITY,
            PARAM_OTP_SENDER_ID,
            PARAM_OTP_LENGTH,
            PARAM_OTP_TTL_SECONDS,
        ],
        // What the census lookup searches by, and what the flow writes back.
        profile_attributes: &[
            "middleName",
            "dateOfBirth",
            "mobile",
            "sequent.read-only.id-card-type",
            "sequent.read-only.id-card-number",
            "sequent.read-only.id-card-number-validated",
        ],
        secrets: &[ConfigSecret {
            config_alias: SCANOVATE_CONFIG_ALIAS,
            key: SCANOVATE_LIVENESS_SECRET,
        }],
        // The voter's browser calls it on the realm's own host.
        deployment_settings: &[DeploymentSetting {
            config_alias: SCANOVATE_CONFIG_ALIAS,
            key: SCANOVATE_LIVENESS_URL,
            parameter: PARAM_SCANOVATE_LIVENESS_URL,
        }],
        requires: &[
            Requirement {
                kind: RequirementKind::Authenticator,
                name: SCANOVATE_AUTHENTICATOR,
                why: "the passport and selfie are verified by this provider, \
                      which only the ovcs Keycloak image ships",
            },
            Requirement {
                kind: RequirementKind::Authenticator,
                name: CONDITIONAL_AUTH_NOTE,
                why: "the passport's issuing country is checked by this condition",
            },
            Requirement {
                kind: RequirementKind::Authenticator,
                name: DENY_ACCESS_AUTHENTICATOR,
                why: "a passport from a country that is not accepted is refused \
                      by this step",
            },
        ],
    },
    AuthPreset {
        name: "digital_certificates",
        summary: "The voter presents a digital certificate, brokered through the \
                  platform's digital-certificates provider.",
        uses_otp: false,
        secrets: &[],
        deployment_settings: &[],
        build: certificates_patch,
        required_parameters: &[],
        optional_parameters: &[],
        // The certificate carries the identity; nothing extra is read off the voter.
        profile_attributes: &[],
        requires: &[Requirement {
            kind: RequirementKind::Flow,
            name: CERTIFICATE_FIRST_LOGIN_FLOW,
            why: "a first-time certificate login is handed to this flow",
        }],
    },
    AuthPreset {
        name: "otp_email_or_sms",
        summary: "The voter types their member id, then a one-time code sent to \
                  whichever of email and mobile the census holds for them.",
        uses_otp: true,
        secrets: &[],
        deployment_settings: &[],
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
            kind: RequirementKind::Authenticator,
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
        secrets: &[],
        deployment_settings: &[],
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
            kind: RequirementKind::Flow,
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
        secrets: &[],
        deployment_settings: &[],
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
                kind: RequirementKind::Authenticator,
                name: OTP_AUTHENTICATOR,
                why: "the one-time code step is bound to this authenticator's \
                      config",
            },
            Requirement {
                kind: RequirementKind::AuthenticatorConfig,
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
                "biometric_authentication",
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
        assert!(get("biometric_authentication").unwrap().uses_otp);
    }

    fn biometric(values: &[(&str, &str)]) -> RealmPatch {
        let mut given = vec![
            (PARAM_SCANOVATE_OCR_URL, "https://ocr.example.org"),
            (PARAM_SCANOVATE_LIVENESS_URL, "https://liveness.example.org"),
            (PARAM_SCANOVATE_FACE_MATCH_URL, "https://match.example.org"),
        ];
        given.extend_from_slice(values);
        get("biometric_authentication")
            .unwrap()
            .build(&input(&given))
    }

    fn flow<'a>(patch: &'a RealmPatch, alias: &str) -> &'a Value {
        patch.patch["authenticationFlows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|flow| flow["alias"] == json!(alias))
            .unwrap_or_else(|| panic!("no flow '{alias}'"))
    }

    fn config<'a>(patch: &'a RealmPatch, alias: &str) -> &'a Value {
        patch.patch["authenticatorConfig"]
            .as_array()
            .unwrap()
            .iter()
            .find(|config| config["alias"] == json!(alias))
            .map(|config| &config["config"])
            .unwrap_or_else(|| panic!("no authenticator config '{alias}'"))
    }

    fn steps(flow: &Value) -> Vec<&str> {
        flow["authenticationExecutions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|execution| {
                execution["authenticator"]
                    .as_str()
                    .or_else(|| execution["flowAlias"].as_str())
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn biometric_registration_runs_its_steps_in_order() {
        let patch = biometric(&[]);
        assert_eq!(
            patch.patch["registrationFlow"],
            json!(BIOMETRIC_REGISTRATION_FLOW)
        );
        assert_eq!(
            steps(flow(&patch, BIOMETRIC_REGISTRATION_FLOW)),
            [
                "registration-page-form",
                OTP_AUTHENTICATOR,
                SCANOVATE_AUTHENTICATOR,
                PASSPORT_COUNTRY_CHECK_FLOW,
                LOOKUP_AND_UPDATE_USER,
            ]
        );
        assert_eq!(
            steps(flow(&patch, BIOMETRIC_REGISTRATION_FORM_FLOW)),
            [
                "enrollment-window-check",
                "deferred-registration-user-creation"
            ]
        );
    }

    #[test]
    fn a_passport_from_another_country_is_denied() {
        let patch = biometric(&[]);
        let check = flow(&patch, PASSPORT_COUNTRY_CHECK_FLOW);
        assert_eq!(check["topLevel"], json!(false));
        assert_eq!(
            steps(check),
            [CONDITIONAL_AUTH_NOTE, DENY_ACCESS_AUTHENTICATOR]
        );

        let registration = flow(&patch, BIOMETRIC_REGISTRATION_FLOW);
        let subflow = registration["authenticationExecutions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| {
                step["flowAlias"] == json!(PASSPORT_COUNTRY_CHECK_FLOW)
            })
            .unwrap();
        assert_eq!(subflow["requirement"], json!("CONDITIONAL"));
        assert_eq!(subflow["authenticatorFlow"], json!(true));

        let condition = config(&patch, PASSPORT_COUNTRY_CONDITION_CONFIG);
        assert_eq!(
            condition["conditional-auth-note-key"],
            json!("issuingCountry")
        );
        assert_eq!(
            condition["conditional-auth-note-value"],
            json!(r"(?i)\s*(ESP|PHL|ISR|NLD)\s*")
        );
        assert_eq!(condition["value-is-regex"], json!("true"));
        assert_eq!(condition["negate"], json!("true"));
        assert_eq!(
            config(&patch, PASSPORT_COUNTRY_DENIED_CONFIG)["denyErrorMessage"],
            json!("passportCountryNotAccepted")
        );
    }

    #[test]
    fn the_accepted_passport_countries_can_be_set() {
        let patch =
            biometric(&[(PARAM_ACCEPTED_PASSPORT_COUNTRIES, "PHL|JPN")]);
        assert_eq!(
            config(&patch, PASSPORT_COUNTRY_CONDITION_CONFIG)
                ["conditional-auth-note-value"],
            json!(r"(?i)\s*(PHL|JPN)\s*")
        );
    }

    #[test]
    fn the_scanovate_step_reads_a_passport_and_stores_its_country() {
        let patch = biometric(&[]);
        let scanovate = config(&patch, SCANOVATE_CONFIG_ALIAS);
        assert_eq!(scanovate["ocr-url"], json!("https://ocr.example.org"));
        assert_eq!(
            scanovate["liveness-url"],
            json!("https://liveness.example.org")
        );
        assert_eq!(
            scanovate["face-match-url"],
            json!("https://match.example.org")
        );
        assert_eq!(
            scanovate["ocr-types"],
            json!(r#"{"philippinePassport": "passport"}"#)
        );

        let rules: Value = serde_json::from_str(
            scanovate["attributes-to-validate"].as_str().unwrap(),
        )
        .unwrap();
        let rules = rules["philippinePassport"].as_array().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0]["type"], json!("isBeforeDateValue"));
        assert_eq!(rules[0]["isBeforeDateValue"], json!("now"));
        assert_eq!(rules[0]["attributePath"], json!("/date_of_expiry"));

        let stored: Value = serde_json::from_str(
            scanovate["attributes-to-store"].as_str().unwrap(),
        )
        .unwrap();
        assert!(stored["philippinePassport"].as_array().unwrap().iter().any(
            |attribute| attribute["UserAttribute"] == json!("issuingCountry")
                && attribute["attributePath"] == json!("/issuing_country_code")
        ));

        let registration = flow(&patch, BIOMETRIC_REGISTRATION_FLOW);
        let step = registration["authenticationExecutions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|step| {
                step["authenticator"] == json!(SCANOVATE_AUTHENTICATOR)
            })
            .unwrap();
        assert_eq!(step["authenticatorConfig"], json!(SCANOVATE_CONFIG_ALIAS));
    }

    #[test]
    fn the_liveness_secret_is_never_written() {
        // Left out rather than blanked, so a base realm's secret survives the
        // merge by alias.
        let patch = biometric(&[]);
        assert!(config(&patch, SCANOVATE_CONFIG_ALIAS)
            .get(SCANOVATE_LIVENESS_SECRET)
            .is_none());
        assert_eq!(
            get("biometric_authentication").unwrap().secrets,
            [ConfigSecret {
                config_alias: SCANOVATE_CONFIG_ALIAS,
                key: SCANOVATE_LIVENESS_SECRET,
            }]
        );
    }

    #[test]
    fn the_internal_services_default_to_their_deployed_names() {
        let patch = get("biometric_authentication").unwrap().build(&input(&[]));
        let scanovate = config(&patch, SCANOVATE_CONFIG_ALIAS);
        assert_eq!(scanovate["ocr-url"], json!(SCANOVATE_OCR_URL_DEFAULT));
        assert_eq!(
            scanovate["face-match-url"],
            json!(SCANOVATE_FACE_MATCH_URL_DEFAULT)
        );
    }

    #[test]
    fn the_liveness_url_is_left_to_the_realm_unless_given() {
        // The voter's browser reaches it on the realm's own host, which a plan
        // does not know, so a base realm's value survives the merge by alias.
        let preset = get("biometric_authentication").unwrap();
        let patch = preset.build(&input(&[]));
        assert!(config(&patch, SCANOVATE_CONFIG_ALIAS)
            .get(SCANOVATE_LIVENESS_URL)
            .is_none());
        assert_eq!(
            preset.deployment_settings,
            [DeploymentSetting {
                config_alias: SCANOVATE_CONFIG_ALIAS,
                key: SCANOVATE_LIVENESS_URL,
                parameter: PARAM_SCANOVATE_LIVENESS_URL,
            }]
        );
    }

    #[test]
    fn an_unmatched_biometric_enrollment_waits_for_approval() {
        let patch = biometric(&[]);
        let lookup = config(&patch, BIOMETRIC_LOOKUP_CONFIG);
        assert_eq!(
            lookup["no-matching-voter-policy"],
            json!("PENDING_APPROVAL")
        );
        assert_eq!(
            lookup["update-attributes"],
            json!(
                "sequent.read-only.id-card-number-validated,email,\
                 sequent.read-only.mobile-number,sequent.read-only.id-card-number,\
                 sequent.read-only.id-card-type"
            )
        );
        assert_eq!(patch.patch["loginTheme"], json!("sequent-ui-voting"));
    }

    #[test]
    fn the_biometric_otp_settings_can_be_set() {
        let patch = biometric(&[
            (PARAM_OTP_SENDER_ID, "COMELEC"),
            (PARAM_OTP_LENGTH, "8"),
            (PARAM_OTP_TTL_SECONDS, "120"),
        ]);
        let otp = config(&patch, BIOMETRIC_OTP_CONFIG);
        assert_eq!(otp["senderId"], json!("COMELEC"));
        assert_eq!(otp["length"], json!("8"));
        assert_eq!(otp["ttl"], json!("120"));
        // Bound in the flow itself: binding every OTP execution in the realm
        // would rewire the login flow too.
        assert!(patch.bind_authenticator_config.is_none());
    }

    #[test]
    fn the_new_messages_are_translated() {
        let patch = biometric(&[]);
        for locale in ["en", "tl"] {
            let texts = &patch.patch["localizationTexts"][locale];
            for key in [
                "philippinePassport",
                "issuingCountry",
                "passportCountryNotAccepted",
            ] {
                assert!(texts[key].is_string(), "{locale}.{key}");
            }
        }
        assert_eq!(
            patch.patch["localizationTexts"]["en"]["philippinePassport"],
            json!("Passport")
        );
    }

    #[test]
    fn biometric_authentication_needs_no_parameter_to_build() {
        // The wizard has no field for one, so a required parameter would make
        // the preset impossible to choose there.
        let preset = get("biometric_authentication").unwrap();
        assert!(preset.uses_otp);
        assert!(preset.required_parameters.is_empty());
        for parameter in [
            PARAM_SCANOVATE_OCR_URL,
            PARAM_SCANOVATE_LIVENESS_URL,
            PARAM_SCANOVATE_FACE_MATCH_URL,
        ] {
            assert!(
                preset.optional_parameters.contains(&parameter),
                "{parameter}"
            );
        }
        for need in [
            SCANOVATE_AUTHENTICATOR,
            CONDITIONAL_AUTH_NOTE,
            DENY_ACCESS_AUTHENTICATOR,
        ] {
            assert!(
                preset.requires.iter().any(|requirement| requirement.kind
                    == RequirementKind::Authenticator
                    && requirement.name == need),
                "{need}"
            );
        }
        for attribute in [
            "mobile",
            "sequent.read-only.id-card-type",
            "sequent.read-only.id-card-number",
        ] {
            assert!(preset.profile_attributes.contains(&attribute));
        }
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

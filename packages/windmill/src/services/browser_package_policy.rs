// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Explicit opt-in admission of configuration packages signed with
//! officials' browser security keys (WebAuthn), for one tenant of an
//! deployment. Preview and normal-site opt-ins are independent.
//!
//! What a tenant trusts here comes from the deployment alone, never from a
//! package, an imported tenant record or the tenant's settings: an explicit
//! switch, the one tenant it applies to, a roster file and its SHA-256 pin,
//! and the approvals required ([`SWITCH`] and the names beside it).
//!
//! * Switch unset (the default): nothing changes. Every package is
//!   admitted exactly as before, and a browser-signed one is refused by the
//!   certificate verifier itself.
//! * Switch set to anything but [`SWITCH_VALUE`], or set with its tenant,
//!   roster, pin or threshold missing or unusable: every package is refused,
//!   signed or not, for every tenant ([`BrowserPolicy::Unusable`]). Half a
//!   configuration never falls back to the legacy rules.
//! * Switch set for another tenant: this tenant is as before.
//! * Switch set for this tenant: this tenant imports browser-signed packages
//!   only, checked against the pinned roster ([`BrowserPolicy::BrowserOnly`]).
//!   A certificate-signed or unsigned file is refused: no downgrade.
//!
//! The policy is read when an import or application holds the tenant's
//! lock, and read again under the import lock before each section of an
//! application is applied ([`recheck`]).

use std::path::Path;

use chrono::{DateTime, Utc};
use sequent_core::election_config::package_verify::{
    admit_for_tenant_at, Admission, ConfigurationSigning, VerifiedPackage,
};
use sequent_core::election_config::problem::{Code, Problem, Report};
use sequent_core::election_config::webauthn_package::{
    is_browser_member, is_browser_verified, verify_browser_package, ENVELOPE_MEMBER,
};
use sequent_core::election_config::webauthn_proof::PinnedRoster;

/// The switch. Only [`SWITCH_VALUE`] turns the preview on.
pub const SWITCH: &str = "WINDMILL_WEBAUTHN_PACKAGES_PREVIEW";
pub const SWITCH_VALUE: &str = "nonproduction-isolated-test";
/// Independent normal-site opt-in. Never inferred from tenant settings or imports.
pub const NORMAL_SWITCH: &str = "WINDMILL_WEBAUTHN_PACKAGES";
pub const NORMAL_SWITCH_VALUE: &str = "site-pinned";
/// The one tenant (its id) that imports browser-signed packages only.
pub const TENANT: &str = "WINDMILL_WEBAUTHN_PACKAGES_TENANT_ID";
/// The roster file, as the deployment provisions it.
pub const ROSTER_FILE: &str = "WINDMILL_WEBAUTHN_PACKAGES_ROSTER_FILE";
/// The roster file's SHA-256, lowercase hex, provisioned apart from it.
pub const ROSTER_SHA256: &str = "WINDMILL_WEBAUTHN_PACKAGES_ROSTER_SHA256";
/// How many approvals by distinct approvers a package needs: at least one.
pub const REQUIRED_APPROVALS: &str = "WINDMILL_WEBAUTHN_PACKAGES_REQUIRED_APPROVALS";

/// What this deployment says of browser-signed packages for one tenant.
#[derive(Debug, Clone)]
pub enum BrowserPolicy {
    /// No preview: everything as before.
    Off,
    /// The preview is for another tenant: this one is as before.
    OtherTenant,
    /// The preview is switched on but can't be used: nothing is admitted.
    Unusable(String),
    /// This tenant imports browser-signed packages only, on this roster.
    BrowserOnly(PinnedRoster),
}

impl BrowserPolicy {
    /// The deployment's policy for `tenant_id`, from its environment and
    /// the roster file it names.
    pub fn load(tenant_id: &str) -> Self {
        Self::from_sources(
            tenant_id,
            |name| {
                std::env::var_os(name).map(|value| {
                    value
                        .into_string()
                        .unwrap_or_else(|_| "<non-Unicode>".into())
                })
            },
            |path| std::fs::read_to_string(path).map_err(|error| error.to_string()),
        )
    }

    /// [`BrowserPolicy::load`] from `var` and `read`, for a test to supply.
    pub fn from_sources(
        tenant_id: &str,
        var: impl Fn(&str) -> Option<String>,
        read: impl Fn(&Path) -> Result<String, String>,
    ) -> Self {
        let set = |name: &str| var(name).filter(|value| !value.trim().is_empty());
        match (var(SWITCH), var(NORMAL_SWITCH)) {
            (None, None) => {
                if [TENANT, ROSTER_FILE, ROSTER_SHA256, REQUIRED_APPROVALS]
                    .iter()
                    .any(|name| var(name).is_some())
                {
                    return BrowserPolicy::Unusable(
                        "browser package trust is configured without an explicit mode".into(),
                    );
                }
                return BrowserPolicy::Off;
            }
            (Some(value), None) if value == SWITCH_VALUE => {}
            (None, Some(value)) if value == NORMAL_SWITCH_VALUE => {}
            _ => {
                return BrowserPolicy::Unusable(format!(
                    "set only {SWITCH}={SWITCH_VALUE} or {NORMAL_SWITCH}={NORMAL_SWITCH_VALUE}"
                ))
            }
        }
        let Some(tenant) = set(TENANT) else {
            return BrowserPolicy::Unusable(format!("browser package mode is set without {TENANT}"));
        };
        let configured_tenant =
            match sequent_core::services::uuid_validation::parse_uuid_v4(tenant.trim()) {
                Ok(tenant) => tenant,
                Err(_) => return BrowserPolicy::Unusable(format!("{TENANT} is not a v4 UUID")),
            };
        let attempted_tenant =
            match sequent_core::services::uuid_validation::parse_uuid_v4(tenant_id.trim()) {
                Ok(tenant) => tenant,
                Err(_) => {
                    return BrowserPolicy::Unusable(
                        "the destination tenant is not a v4 UUID".into(),
                    )
                }
            };
        if configured_tenant != attempted_tenant {
            return BrowserPolicy::OtherTenant;
        }
        let canonical_tenant = configured_tenant.hyphenated().to_string();
        let (Some(file), Some(pin), Some(required)) = (
            set(ROSTER_FILE),
            set(ROSTER_SHA256),
            set(REQUIRED_APPROVALS),
        ) else {
            return BrowserPolicy::Unusable(format!(
                "{ROSTER_FILE}, {ROSTER_SHA256} and {REQUIRED_APPROVALS} are all required"
            ));
        };
        let required = match required.trim().parse::<usize>() {
            Ok(required) if required > 0 => required,
            _ => {
                return BrowserPolicy::Unusable(format!(
                    "{REQUIRED_APPROVALS} is not a number of approvals of at least one"
                ))
            }
        };
        let text = match read(Path::new(file.trim())) {
            Ok(text) => text,
            Err(error) => {
                return BrowserPolicy::Unusable(format!("the roster file can't be read: {error}"))
            }
        };
        match PinnedRoster::load(&text, pin.trim(), &canonical_tenant, required) {
            Ok(pinned) => BrowserPolicy::BrowserOnly(pinned),
            Err(error) => BrowserPolicy::Unusable(error.to_string()),
        }
    }
}

fn unusable(why: &str) -> Problem {
    Problem::error(
        Code::UntrustedSigner,
        ENVELOPE_MEMBER,
        format!(
            "this deployment's security-key package policy can't be used ({why}), so no \
             package is imported"
        ),
    )
    .id("package.webauthn-policy-unusable")
    .detail("reason", why)
}

fn browser_only() -> Problem {
    Problem::error(
        Code::Unsigned,
        ENVELOPE_MEMBER,
        "this tenant imports only packages signed with officials' security keys, and this \
         file is not one",
    )
    .id("package.webauthn-required")
}

/// The admission of `bytes` for `tenant_id` under `policy`: the legacy
/// admission unchanged where the preview doesn't apply, nothing where it
/// can't be used, and only a browser-signed package verified against the
/// pinned roster where it does.
pub fn admit_with_policy(
    bytes: &[u8],
    setting: Option<&ConfigurationSigning>,
    tenant_id: &str,
    revocation_lists: Vec<Vec<u8>>,
    now: DateTime<Utc>,
    policy: &BrowserPolicy,
) -> Result<Admission, Report> {
    match policy {
        BrowserPolicy::Off | BrowserPolicy::OtherTenant => {
            admit_for_tenant_at(bytes, setting, tenant_id, revocation_lists, now)
        }
        BrowserPolicy::Unusable(why) => Err(Report::from_problem(unusable(why))),
        BrowserPolicy::BrowserOnly(pinned) => {
            // A malformed stored setting still stops everything, as it does
            // for every other file.
            if let Some(setting) = setting {
                setting.check_stored()?;
            }
            let names = sequent_core::election_config::manifest::member_names(bytes)
                .map_err(Report::from_problem)?;
            if !names.iter().any(|name| is_browser_member(name)) {
                return Err(Report::from_problem(browser_only()));
            }
            verify_browser_package(bytes, pinned, Some(now))
                .map(|verified| Admission::Verified(Box::new(verified.into_verified_package())))
        }
    }
}

/// What an application does before each section, given the policy now and
/// the package it admitted when it started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recheck {
    /// As before: the legacy checks only.
    Proceed,
    /// Admit the package again, under the tenant's import lock, and go on
    /// only if it is admitted as it was.
    Readmit,
    /// Stop: the policy no longer admits what this application admitted.
    Block(String),
}

pub fn recheck(policy: &BrowserPolicy, admitted: Option<&VerifiedPackage>) -> Recheck {
    let browser = admitted.is_some_and(is_browser_verified);
    match (policy, browser) {
        (BrowserPolicy::Off | BrowserPolicy::OtherTenant, false) => Recheck::Proceed,
        (BrowserPolicy::Off | BrowserPolicy::OtherTenant, true) => Recheck::Block(
            "security-key packages are no longer admitted for this tenant".to_string(),
        ),
        (BrowserPolicy::Unusable(why), _) => Recheck::Block(unusable(why).message),
        (BrowserPolicy::BrowserOnly(_), false) => Recheck::Block(browser_only().message),
        (BrowserPolicy::BrowserOnly(_), true) => Recheck::Readmit,
    }
}

/// Whether a package admitted again is the one admitted before: the same
/// manifest, signed by the same people on the same pinned roster.
pub fn admitted_as_before(before: &VerifiedPackage, again: &VerifiedPackage) -> bool {
    before.scheme == again.scheme
        && before.manifest_sha256 == again.manifest_sha256
        && before.signer == again.signer
        && before.approvers == again.approvers
}

/// Scheme-specific persisted-chain check. The caller reads the last import
/// while holding the tenant import lock.
pub fn freshness(
    package: &VerifiedPackage,
    last: Option<&sequent_core::election_config::package_verify::LastImport>,
) -> Result<sequent_core::election_config::package_verify::Freshness, Problem> {
    if is_browser_verified(package) {
        sequent_core::election_config::webauthn_package::browser_freshness(package, last)
    } else {
        sequent_core::election_config::package_verify::freshness(package, last)
    }
}

#[cfg(test)]
#[path = "browser_package_policy_tests.rs"]
mod browser_package_policy_tests;

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use strum_macros::{Display, EnumString};

/// Whether the pages a voter sees carry the platform's own branding: its
/// logo and favicon where the election event sets none, the "powered by"
/// footer and the version label.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    JsonSchema,
    EnumString,
    Display,
)]
pub enum PlatformBrandingPolicy {
    #[default]
    #[strum(serialize = "show")]
    #[serde(rename = "show")]
    Show,
    #[strum(serialize = "hide")]
    #[serde(rename = "hide")]
    Hide,
}

/// Reads an optional policy, taking a code this version does not know as no
/// policy, so that one unknown value cannot make a whole election event
/// presentation unreadable.
pub fn deserialize_lenient_platform_branding_policy<'de, D>(
    deserializer: D,
) -> Result<Option<PlatformBrandingPolicy>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    Ok(value.and_then(|value| serde_json::from_value(value).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    #[test]
    fn every_policy_keeps_its_code() {
        for (policy, code) in [
            (PlatformBrandingPolicy::Show, "show"),
            (PlatformBrandingPolicy::Hide, "hide"),
        ] {
            assert_eq!(policy.to_string(), code);
            assert_eq!(PlatformBrandingPolicy::from_str(code), Ok(policy));
            assert_eq!(serde_json::to_value(policy).unwrap(), json!(code));
        }
    }

    #[test]
    fn the_platform_branding_shows_unless_told_otherwise() {
        assert_eq!(
            PlatformBrandingPolicy::default(),
            PlatformBrandingPolicy::Show
        );
    }
}

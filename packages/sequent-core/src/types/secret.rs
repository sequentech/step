// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::fmt;

/// A secret such as a password. It serializes as the plain string, so request
/// bodies and task arguments keep their format, but `Debug` never shows it:
/// derived `Debug` output and `#[instrument]` spans print `<redacted>`.
#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SecretString(String);

impl SecretString {
    pub fn new(secret: String) -> Self {
        Self(secret)
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<String> for SecretString {
    fn from(secret: String) -> Self {
        Self(secret)
    }
}

impl From<&str> for SecretString {
    fn from(secret: &str) -> Self {
        Self(secret.to_string())
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

#[cfg(test)]
mod tests {
    use super::SecretString;

    #[derive(Debug, serde::Serialize, serde::Deserialize)]
    struct Body {
        password: SecretString,
        hint: Option<SecretString>,
    }

    #[test]
    fn debug_output_never_contains_the_secret() {
        let body = Body {
            password: "hunter2".into(),
            hint: Some("also-secret".into()),
        };
        let debug = format!("{body:?}");
        assert!(!debug.contains("hunter2"), "{debug}");
        assert!(!debug.contains("also-secret"), "{debug}");
        assert!(debug.contains("<redacted>"), "{debug}");
    }

    #[test]
    fn serializes_as_the_plain_string() {
        let json = serde_json::json!({"password": "hunter2", "hint": null});
        let body: Body = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(body.password.expose_secret(), "hunter2");
        assert_eq!(serde_json::to_value(&body).unwrap(), json);
    }
}

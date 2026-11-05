// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The election authority's contact (specs/policy.md, section 3, rule 19):
//! how voters reach it, for example to report an audit mismatch. The admin
//! types it in the Transparency tab; the draft policy carries it as `contact`
//! lines, and voters' apps show it from the pinned policy only.

use anyhow::{anyhow, bail, ensure, Result};
use sequent_core::ballot::AuthorityContactPresentation;
use serde_json::Value;
use tracing::warn;

use super::config::check_url;

/// The presentation's key for the contact.
pub const PRESENTATION_AUTHORITY_CONTACT: &str = "authority_contact";

const MAX_NAME: usize = 128;
const MAX_EMAIL: usize = 254;
const MAX_LOCAL: usize = 64;
const MAX_LABEL: usize = 63;

/// What the draft carries: each kind that is set, at most once.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuthorityContact {
    pub name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub url: Option<String>,
}

/// The Unicode 16.0 format characters that `text` excludes (specs/entry.md,
/// section 3).
fn is_format_char(c: char) -> bool {
    matches!(u32::from(c),
        0x00AD | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x0890..=0x0891
        | 0x08E2 | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064
        | 0x2066..=0x206F | 0xFEFF | 0xFFF9..=0xFFFB | 0x110BD | 0x110CD
        | 0x13430..=0x1343F | 0x1BCA0..=0x1BCA3 | 0x1D173..=0x1D17A | 0xE0001
        | 0xE0020..=0xE007F)
}

/// A contact name: 1 to 128 bytes of text on one line, with single spaces
/// only.
pub fn check_name(name: &str) -> Result<()> {
    ensure!(
        !name.is_empty() && name.len() <= MAX_NAME,
        "the contact name is 1 to {MAX_NAME} bytes"
    );
    ensure!(
        !name.chars().any(|c| c.is_control() || is_format_char(c)),
        "the contact name has a control or format character"
    );
    ensure!(
        !name.starts_with(' ') && !name.ends_with(' ') && !name.contains("  "),
        "the contact name has other spaces than single ones between words"
    );
    Ok(())
}

/// A contact email: ASCII `local@domain` of at most 254 bytes.
pub fn check_email(email: &str) -> Result<()> {
    ensure!(
        email.len() <= MAX_EMAIL && email.is_ascii(),
        "the contact email isn't ASCII of at most {MAX_EMAIL} bytes"
    );
    let (local, domain) = email
        .split_once('@')
        .filter(|(_, domain)| !domain.contains('@'))
        .ok_or_else(|| anyhow!("the contact email isn't local@domain"))?;
    ensure!(
        (1..=MAX_LOCAL).contains(&local.len())
            && local
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".%_+-".contains(&b))
            && !local.starts_with('.')
            && !local.ends_with('.')
            && !local.contains(".."),
        "the contact email's local part isn't valid"
    );
    let labels: Vec<&str> = domain.split('.').collect();
    ensure!(
        labels.len() >= 2
            && labels.iter().all(|label| {
                (1..=MAX_LABEL).contains(&label.len())
                    && label
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                    && !label.starts_with('-')
                    && !label.ends_with('-')
            }),
        "the contact email's domain isn't valid"
    );
    Ok(())
}

/// A contact phone: `+`, then 7 to 15 digits, the first not `0`.
pub fn check_phone(phone: &str) -> Result<()> {
    let digits = phone
        .strip_prefix('+')
        .ok_or_else(|| anyhow!("the contact phone starts with +"))?;
    ensure!(
        (7..=15).contains(&digits.len())
            && digits.bytes().all(|b| b.is_ascii_digit())
            && !digits.starts_with('0'),
        "the contact phone is + and 7 to 15 digits, the first not 0"
    );
    Ok(())
}

/// A contact URL: absolute and `https`, not even `http` on localhost.
pub fn check_contact_url(url: &str) -> Result<()> {
    if !url.starts_with("https://") {
        bail!("the contact URL isn't an https URL");
    }
    check_url("the contact URL", url)
}

impl AuthorityContact {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.email.is_none() && self.phone.is_none() && self.url.is_none()
    }

    /// Whether every kind that is set follows its syntax.
    pub fn check(&self) -> Result<()> {
        if let Some(name) = &self.name {
            check_name(name)?;
        }
        if let Some(email) = &self.email {
            check_email(email)?;
        }
        if let Some(phone) = &self.phone {
            check_phone(phone)?;
        }
        if let Some(url) = &self.url {
            check_contact_url(url)?;
        }
        Ok(())
    }

    /// The policy's `contact` lines: name, email, phone, url.
    pub fn lines(&self) -> Vec<[&str; 3]> {
        [
            ("name", &self.name),
            ("email", &self.email),
            ("phone", &self.phone),
            ("url", &self.url),
        ]
        .into_iter()
        .filter_map(|(kind, value)| value.as_deref().map(|value| ["contact", kind, value]))
        .collect()
    }

    /// The contact the admin typed, as the presentation keeps it. A kind that
    /// is empty after trimming is not set, and one that breaks its syntax is
    /// left out with a warning, so that the draft stays one the Board can
    /// seal; the tab validates what it saves, so this is a guard for what
    /// was stored by other means.
    pub fn from_presentation(presentation: Option<&Value>) -> Self {
        let Some(value) = presentation
            .and_then(|p| p.get(PRESENTATION_AUTHORITY_CONTACT))
            .filter(|v| v.is_object())
        else {
            return Self::default();
        };
        let Ok(stored) = serde_json::from_value::<AuthorityContactPresentation>(value.clone())
        else {
            warn!("The event's authority contact isn't readable: left out of the policy");
            return Self::default();
        };
        let clean = |kind: &str, value: Option<String>, check: fn(&str) -> Result<()>| {
            let value = value
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty())?;
            match check(&value) {
                Ok(()) => Some(value),
                Err(error) => {
                    warn!("The event's contact {kind} is left out of the policy: {error}");
                    None
                }
            }
        };
        Self {
            name: clean("name", stored.name, check_name),
            email: clean("email", stored.email, check_email),
            phone: clean("phone", stored.phone, check_phone),
            url: clean("url", stored.url, check_contact_url),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn names_follow_the_text_rules() {
        assert!(check_name("Electoral Board of Acme").is_ok());
        assert!(check_name("Junta Electoral de Castellón").is_ok());
        assert!(check_name(&"a".repeat(128)).is_ok());
        assert!(check_name(&"a".repeat(129)).is_err());
        // 64 two-byte characters are 128 bytes; one more is over.
        assert!(check_name(&"ñ".repeat(64)).is_ok());
        assert!(check_name(&"ñ".repeat(65)).is_err());
        for bad in [
            "",
            " Board",
            "Board ",
            "Electoral  Board",
            "Electoral\nBoard",
            "Electoral\tBoard",
            "Board\u{200b}",
            "Board\u{202e}",
            "Board\u{feff}",
            "Board\u{85}",
        ] {
            assert!(check_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn emails_are_plain_ascii_addresses() {
        for good in [
            "elections@acme.example",
            "a.b+c_d%e-f@sub.acme-x.example",
            "x@a.b",
        ] {
            assert!(check_email(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "elections",
            "elections@acme",
            "@acme.example",
            ".a@acme.example",
            "a.@acme.example",
            "a..b@acme.example",
            "a b@acme.example",
            "a@@acme.example",
            "a@b@acme.example",
            "a@-acme.example",
            "a@acme-.example",
            "a@acme..example",
            "José@acme.example",
            "a@acmé.example",
            "a@acme.example.",
            "\"a b\"@acme.example",
        ] {
            assert!(check_email(bad).is_err(), "{bad:?}");
        }
        let long_local = format!("{}@acme.example", "a".repeat(65));
        assert!(check_email(&long_local).is_err());
        let long_label = format!("a@{}.example", "a".repeat(64));
        assert!(check_email(&long_label).is_err());
        let long = format!("a@{}.example", vec!["a".repeat(63); 4].join("."));
        assert!(long.len() > 254 && check_email(&long).is_err());
    }

    #[test]
    fn phones_are_a_plus_and_seven_to_fifteen_digits() {
        for good in ["+34900000000", "+1234567", "+123456789012345"] {
            assert!(check_phone(good).is_ok(), "{good}");
        }
        for bad in [
            "",
            "34900000000",
            "+",
            "+123456",
            "+1234567890123456",
            "+04900000000",
            "+34 900 000 000",
            "+34-900-000-000",
            "+3490000000a",
        ] {
            assert!(check_phone(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn urls_are_https_only() {
        assert!(check_contact_url("https://elections.acme.example/contact").is_ok());
        for bad in [
            "",
            "http://elections.acme.example",
            "http://localhost:8080/contact",
            "ftp://elections.acme.example",
            "https://user@elections.acme.example",
            "https://",
            "elections.acme.example",
            "https://elections.acme.example/a b",
        ] {
            assert!(check_contact_url(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_lines_come_in_the_order_of_the_kinds() {
        let contact = AuthorityContact {
            name: Some("Electoral Board of Acme".into()),
            email: Some("elections@acme.example".into()),
            phone: Some("+34900000000".into()),
            url: Some("https://elections.acme.example/contact".into()),
        };
        assert!(contact.check().is_ok());
        assert_eq!(
            contact
                .lines()
                .iter()
                .map(|l| l.join(" "))
                .collect::<Vec<_>>(),
            [
                "contact name Electoral Board of Acme",
                "contact email elections@acme.example",
                "contact phone +34900000000",
                "contact url https://elections.acme.example/contact",
            ]
        );
        assert!(AuthorityContact::default().lines().is_empty());
        assert!(AuthorityContact::default().is_empty());
    }

    #[test]
    fn a_stored_contact_is_read_trimmed_and_checked_per_kind() {
        let stored = json!({
            "authority_contact": {
                "name": "  Electoral Board  ",
                "email": "not an address",
                "phone": "",
                "url": "https://elections.acme.example/contact",
            }
        });
        let contact = AuthorityContact::from_presentation(Some(&stored));
        assert_eq!(contact.name.as_deref(), Some("Electoral Board"));
        assert_eq!(contact.email, None, "an invalid kind is left out");
        assert_eq!(contact.phone, None, "an empty kind is not set");
        assert_eq!(
            contact.url.as_deref(),
            Some("https://elections.acme.example/contact")
        );
        assert!(contact.check().is_ok());
    }

    #[test]
    fn a_missing_or_unreadable_contact_is_none() {
        assert!(AuthorityContact::from_presentation(None).is_empty());
        assert!(AuthorityContact::from_presentation(Some(&json!({}))).is_empty());
        assert!(AuthorityContact::from_presentation(Some(&json!({
            "authority_contact": "Board"
        })))
        .is_empty());
        assert!(AuthorityContact::from_presentation(Some(&json!({
            "authority_contact": {"name": 7}
        })))
        .is_empty());
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Recipient addresses: validation, the masked form kept in records, and a
//! keyed digest that correlates records without storing the address.

use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use sequent_core::types::dial_codes;
use sequent_core::types::messaging::{MessageChannel, RecipientKind};
use sha2::Sha256;

/// A validated recipient address in canonical form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub kind: RecipientKind,
    value: String,
}

impl Destination {
    pub fn parse(channel: MessageChannel, raw: &str) -> Result<Destination> {
        let kind = channel.recipient_kind();
        let value = match kind {
            RecipientKind::PHONE_NUMBER => normalize_phone(raw)?,
            RecipientKind::EMAIL_ADDRESS => normalize_email(raw)?,
            RecipientKind::PAGE_SCOPED_ID => normalize_page_scoped_id(raw)?,
        };
        Ok(Destination { kind, value })
    }

    /// The address to hand to the provider.
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Shown in records, logs and screens instead of the address: the
    /// country code and the last three digits (`+966 ••• ••• 567`), the
    /// first two characters of an email (`ma••••••••@example.com`; fewer for
    /// a local part of one or two, never all of it), or the
    /// last three digits of a Page-scoped ID (`••• 321`). The masked parts
    /// have a fixed length, so they don't reveal the address's length.
    pub fn masked(&self) -> String {
        const HIDDEN: &str = "\u{2022}\u{2022}\u{2022}";
        let last_three = |value: &str| value[value.len().saturating_sub(3)..].to_string();
        match self.kind {
            RecipientKind::PHONE_NUMBER => {
                let code = self.calling_code().unwrap_or_default();
                let national = &self.value[1 + code.len()..];
                format!("+{code} {HIDDEN} {HIDDEN} {}", last_three(national))
            }
            RecipientKind::EMAIL_ADDRESS => {
                let (local, domain) = self.value.split_once('@').unwrap_or((&self.value, ""));
                // Never the whole local part, however short it is.
                let shown = 2.min(local.chars().count().saturating_sub(1));
                let first: String = local.chars().take(shown).collect();
                format!("{first}{HIDDEN}{HIDDEN}\u{2022}\u{2022}@{domain}")
            }
            RecipientKind::PAGE_SCOPED_ID => format!("{HIDDEN} {}", last_three(&self.value)),
        }
    }

    /// The ISO 3166-1 alpha-2 country of a phone number.
    pub fn country(&self) -> Option<&'static str> {
        match self.kind {
            RecipientKind::PHONE_NUMBER => dial_codes::iso_country(&self.value),
            _ => None,
        }
    }

    /// Correlates records of the same recipient within a tenant. Keyed, so
    /// it cannot be reversed by enumerating phone numbers.
    pub fn digest(&self, key: &[u8]) -> Result<String> {
        let mut mac = Hmac::<Sha256>::new_from_slice(key)
            .map_err(|error| anyhow!("invalid destination digest key: {error}"))?;
        mac.update(self.kind.to_string().as_bytes());
        mac.update(b":");
        mac.update(self.value.as_bytes());
        Ok(hex::encode(mac.finalize().into_bytes()))
    }

    /// Country calling code of a phone number, as digits.
    pub fn calling_code(&self) -> Option<&'static str> {
        if self.kind != RecipientKind::PHONE_NUMBER {
            return None;
        }
        dial_codes::calling_code(&self.value)
    }
}

fn normalize_phone(raw: &str) -> Result<String> {
    let compact: String = raw
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '(' | ')' | '.'))
        .collect();
    let Some(digits) = compact.strip_prefix('+') else {
        return Err(anyhow!("phone numbers must be in E.164 format"));
    };
    if !(8..=15).contains(&digits.len()) || !digits.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("phone numbers must be in E.164 format"));
    }
    let normalized = format!("+{digits}");
    let destination = Destination {
        kind: RecipientKind::PHONE_NUMBER,
        value: normalized.clone(),
    };
    if destination.calling_code().is_none() {
        return Err(anyhow!("unknown country calling code"));
    }
    Ok(normalized)
}

fn normalize_email(raw: &str) -> Result<String> {
    let email = raw.trim().to_lowercase();
    match email.split_once('@') {
        Some((local, domain))
            if !local.is_empty()
                && domain.contains('.')
                && !domain.starts_with('.')
                && !email.chars().any(char::is_whitespace) =>
        {
            Ok(email)
        }
        _ => Err(anyhow!("invalid email address")),
    }
}

fn normalize_page_scoped_id(raw: &str) -> Result<String> {
    let id = raw.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
        return Err(anyhow!("invalid Page-scoped ID"));
    }
    Ok(id.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phone_numbers_are_normalized_to_e164() {
        let destination =
            Destination::parse(MessageChannel::WHATSAPP, "+63 (917) 123-4567").expect("phone");
        assert_eq!(destination.as_str(), "+639171234567");
        assert_eq!(destination.calling_code(), Some("63"));
        assert_eq!(
            destination.masked(),
            "+63 \u{2022}\u{2022}\u{2022} \u{2022}\u{2022}\u{2022} 567"
        );
        assert_eq!(destination.country(), Some("PH"));
    }

    #[test]
    fn phone_numbers_without_a_country_code_are_rejected() {
        assert!(Destination::parse(MessageChannel::SMS, "09171234567").is_err());
        assert!(Destination::parse(MessageChannel::SMS, "+63abc").is_err());
        assert!(Destination::parse(MessageChannel::SMS, "+1234").is_err());
        assert!(Destination::parse(MessageChannel::VIBER, "+0123456789").is_err());
    }

    #[test]
    fn calling_codes_are_matched_by_length() {
        let code = |raw| {
            Destination::parse(MessageChannel::SMS, raw)
                .expect("phone")
                .calling_code()
                .map(str::to_string)
        };
        assert_eq!(code("+15550123456"), Some("1".to_string()));
        assert_eq!(code("+447700900123"), Some("44".to_string()));
        assert_eq!(code("+971501234567"), Some("971".to_string()));
    }

    #[test]
    fn emails_are_lowercased_and_masked() {
        let destination =
            Destination::parse(MessageChannel::EMAIL, " Voter@Example.ORG ").expect("email");
        assert_eq!(destination.as_str(), "voter@example.org");
        assert_eq!(
            destination.masked(),
            "vo\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}@example.org"
        );
        assert_eq!(destination.country(), None);
        // A short local part is never shown whole.
        let hidden = "\u{2022}".repeat(8);
        for (address, shown) in [("ab@example.org", "a"), ("a@example.org", "")] {
            assert_eq!(
                Destination::parse(MessageChannel::EMAIL, address)
                    .expect("email")
                    .masked(),
                format!("{shown}{hidden}@example.org")
            );
        }
        assert!(Destination::parse(MessageChannel::EMAIL, "voter").is_err());
        assert!(Destination::parse(MessageChannel::EMAIL, "a b@example.org").is_err());
    }

    #[test]
    fn messenger_recipients_are_page_scoped_ids() {
        let destination =
            Destination::parse(MessageChannel::MESSENGER, "6543210987654321").expect("psid");
        assert_eq!(destination.masked(), "\u{2022}\u{2022}\u{2022} 321");
        assert!(Destination::parse(MessageChannel::MESSENGER, "+639171234567").is_err());
    }

    #[test]
    fn digests_depend_on_the_key_and_not_on_formatting() {
        let a = Destination::parse(MessageChannel::SMS, "+63 917 123 4567").expect("a");
        let b = Destination::parse(MessageChannel::WHATSAPP, "+639171234567").expect("b");
        assert_eq!(a.digest(b"key-1").unwrap(), b.digest(b"key-1").unwrap());
        assert_ne!(a.digest(b"key-1").unwrap(), a.digest(b"key-2").unwrap());
        assert!(!a.digest(b"key-1").unwrap().contains("9171234567"));
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Recipient addresses: validation, the masked form kept in records, and a
//! keyed digest that correlates records without storing the address.

use anyhow::{anyhow, Result};
use hmac::{Hmac, Mac};
use sequent_core::types::messaging::{MessageChannel, RecipientKind};
use sha2::Sha256;

/// ITU-T E.164 country calling codes. They are prefix-free, so the first
/// match by length is the code.
const CALLING_CODES: &[&str] = &[
    "1", "7", "20", "27", "30", "31", "32", "33", "34", "36", "39", "40", "41", "43", "44", "45",
    "46", "47", "48", "49", "51", "52", "53", "54", "55", "56", "57", "58", "60", "61", "62", "63",
    "64", "65", "66", "81", "82", "84", "86", "90", "91", "92", "93", "94", "95", "98", "211",
    "212", "213", "216", "218", "220", "221", "222", "223", "224", "225", "226", "227", "228",
    "229", "230", "231", "232", "233", "234", "235", "236", "237", "238", "239", "240", "241",
    "242", "243", "244", "245", "246", "247", "248", "249", "250", "251", "252", "253", "254",
    "255", "256", "257", "258", "260", "261", "262", "263", "264", "265", "266", "267", "268",
    "269", "290", "291", "297", "298", "299", "350", "351", "352", "353", "354", "355", "356",
    "357", "358", "359", "370", "371", "372", "373", "374", "375", "376", "377", "378", "379",
    "380", "381", "382", "383", "385", "386", "387", "389", "420", "421", "423", "500", "501",
    "502", "503", "504", "505", "506", "507", "508", "509", "590", "591", "592", "593", "594",
    "595", "596", "597", "598", "599", "670", "672", "673", "674", "675", "676", "677", "678",
    "679", "680", "681", "682", "683", "685", "686", "687", "688", "689", "690", "691", "692",
    "850", "852", "853", "855", "856", "870", "880", "886", "960", "961", "962", "963", "964",
    "965", "966", "967", "968", "970", "971", "972", "973", "974", "975", "976", "977", "992",
    "993", "994", "995", "996", "998",
];

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

    /// Shown in records and screens instead of the address.
    pub fn masked(&self) -> String {
        match self.kind {
            RecipientKind::PHONE_NUMBER => {
                let code = self.calling_code().unwrap_or_default();
                let national = &self.value[1 + code.len()..];
                let visible = national.len().min(4);
                format!(
                    "+{code}{}{}",
                    "*".repeat(national.len() - visible),
                    &national[national.len() - visible..]
                )
            }
            RecipientKind::EMAIL_ADDRESS => {
                let (local, domain) = self.value.split_once('@').unwrap_or((&self.value, ""));
                let first = local.chars().next().map(String::from).unwrap_or_default();
                format!("{first}***@{domain}")
            }
            RecipientKind::PAGE_SCOPED_ID => {
                let visible = self.value.len().min(4);
                format!("***{}", &self.value[self.value.len() - visible..])
            }
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
    pub fn calling_code(&self) -> Option<&str> {
        if self.kind != RecipientKind::PHONE_NUMBER {
            return None;
        }
        let digits = &self.value[1..];
        (1..=3)
            .filter_map(|length| digits.get(..length))
            .find(|prefix| CALLING_CODES.contains(prefix))
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
        assert_eq!(destination.masked(), "+63******4567");
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
        assert_eq!(destination.masked(), "v***@example.org");
        assert!(Destination::parse(MessageChannel::EMAIL, "voter").is_err());
        assert!(Destination::parse(MessageChannel::EMAIL, "a b@example.org").is_err());
    }

    #[test]
    fn messenger_recipients_are_page_scoped_ids() {
        let destination =
            Destination::parse(MessageChannel::MESSENGER, "6543210987654321").expect("psid");
        assert_eq!(destination.masked(), "***4321");
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

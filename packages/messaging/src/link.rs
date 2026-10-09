// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! One-time Messenger link references. Only keyed digests are stored, so a
//! database read does not reveal a usable reference. A link is completed
//! only by opening it (Get Started or a referral): there is no word to type.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use sequent_core::types::messaging::{MessengerLinkState, MessengerPage};
use sha2::Sha256;

/// 256 random bits, hex encoded. Fits the `m.me` `ref` parameter.
pub fn new_reference() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn digest(key: &[u8], value: &str) -> Result<String> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key)
        .map_err(|error| anyhow!("invalid link digest key: {error}"))?;
    mac.update(value.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// The `m.me` link that opens the Page's conversation with the reference.
pub fn messenger_link(page: &MessengerPage, reference: &str) -> String {
    let target = page.username.as_deref().unwrap_or(page.page_id.as_str());
    format!(
        "https://m.me/{}?ref={}",
        urlencoding::encode(target),
        urlencoding::encode(reference)
    )
}

/// What a stored link allows for a request presenting `auth_session` and
/// `challenge` digests at `now`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkCheck {
    Valid,
    /// Another session or another code: refused without revealing why.
    Mismatch,
    Expired,
    /// Already confirmed, replaced or expired earlier.
    Finished(MessengerLinkState),
}

pub fn check_link(
    state: MessengerLinkState,
    expires_at: DateTime<Utc>,
    stored_session: &str,
    stored_challenge: &str,
    auth_session: &str,
    challenge: &str,
    now: DateTime<Utc>,
) -> LinkCheck {
    if stored_session != auth_session || stored_challenge != challenge {
        return LinkCheck::Mismatch;
    }
    match state {
        MessengerLinkState::PENDING | MessengerLinkState::CODE_SENT => {
            if expires_at <= now {
                LinkCheck::Expired
            } else {
                LinkCheck::Valid
            }
        }
        finished => LinkCheck::Finished(finished),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use std::collections::HashSet;

    #[test]
    fn references_are_random_and_well_formed() {
        let references: HashSet<String> = (0..100).map(|_| new_reference()).collect();
        assert_eq!(references.len(), 100);
        assert!(references.iter().all(|r| r.len() == 64));
    }

    #[test]
    fn digests_are_keyed() {
        assert_eq!(digest(b"k", "ref").unwrap(), digest(b"k", "ref").unwrap());
        assert_ne!(
            digest(b"k", "ref").unwrap(),
            digest(b"other", "ref").unwrap()
        );
    }

    #[test]
    fn links_use_the_username_or_the_page_id() {
        let mut page = MessengerPage {
            page_id: "1234".to_string(),
            username: Some("comelec".to_string()),
            name: None,
        };
        assert_eq!(messenger_link(&page, "abc"), "https://m.me/comelec?ref=abc");
        page.username = None;
        assert_eq!(messenger_link(&page, "abc"), "https://m.me/1234?ref=abc");
    }

    #[test]
    fn links_are_bound_to_the_session_challenge_and_lifetime() {
        let now = Utc::now();
        let later = now + Duration::minutes(5);
        let check = |state, expires_at, session, challenge| {
            check_link(state, expires_at, "s1", "c1", session, challenge, now)
        };
        assert_eq!(
            check(MessengerLinkState::PENDING, later, "s1", "c1"),
            LinkCheck::Valid
        );
        assert_eq!(
            check(MessengerLinkState::CODE_SENT, later, "s1", "c1"),
            LinkCheck::Valid
        );
        assert_eq!(
            check(MessengerLinkState::PENDING, later, "s2", "c1"),
            LinkCheck::Mismatch
        );
        assert_eq!(
            check(MessengerLinkState::PENDING, later, "s1", "c2"),
            LinkCheck::Mismatch
        );
        assert_eq!(
            check(MessengerLinkState::PENDING, now, "s1", "c1"),
            LinkCheck::Expired
        );
        assert_eq!(
            check(MessengerLinkState::REPLACED, later, "s1", "c1"),
            LinkCheck::Finished(MessengerLinkState::REPLACED)
        );
        assert_eq!(
            check(MessengerLinkState::CONFIRMED, later, "s1", "c1"),
            LinkCheck::Finished(MessengerLinkState::CONFIRMED)
        );
    }
}

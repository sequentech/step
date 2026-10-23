// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The statements step signs into an event's `events` log (Vitrine's
//! `specs/entry.md`, sections 5.2 and 5.3), and the hashes the booth compares
//! with them.

use anyhow::{ensure, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use strum_macros::{AsRefStr, Display, EnumString};

use super::entry::{check_id, Hash};

pub const CONFIG_TYPE: &str = "vitrine/config/v1";
pub const MILESTONE_TYPE: &str = "vitrine/milestone/v1";
const POLICY_ID_PREFIX: &str = "sha256:";

/// The `transparency` line of a config statement, as the policy's
/// `visibility`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

/// The `demo` line: a demonstration event says so on every Vitrine view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum Demo {
    Yes,
    No,
}

/// A `release <package> <version>` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub package: String,
    pub version: String,
}

fn check_token(what: &str, value: &str) -> Result<()> {
    ensure!(
        !value.is_empty() && value.bytes().all(|b| b.is_ascii_graphic()),
        "{what} is one word of visible ASCII: {value:?}"
    );
    Ok(())
}

/// A policy ID, `sha256:` and 64 lowercase hex digits.
pub fn check_policy_id(id: &str) -> Result<()> {
    let hex = id.strip_prefix(POLICY_ID_PREFIX).unwrap_or_default();
    ensure!(
        hex.len() == 64
            && hex
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "a policy ID is sha256: and 64 lowercase hex digits"
    );
    Ok(())
}

/// `key`: SHA-256 over the election public key's bytes as the booth encrypts
/// to it.
pub fn election_key_hash(public_key: &[u8]) -> Hash {
    Sha256::digest(public_key).into()
}

/// The digest of one ballot style: SHA-256 over the exact ballot-style text
/// the voting portal receives.
pub fn style_digest(style: &str) -> Hash {
    Sha256::digest(style.as_bytes()).into()
}

/// `styles`: SHA-256 over the concatenation of the sorted style digests.
pub fn styles_hash(digests: &[Hash]) -> Hash {
    let mut sorted = digests.to_vec();
    sorted.sort_unstable();
    let mut h = Sha256::new();
    for digest in &sorted {
        h.update(digest);
    }
    h.finalize().into()
}

/// A `vitrine/config/v1` statement: what one election's booth must check
/// before it encrypts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConfigStatement {
    pub event_id: String,
    pub election_id: String,
    pub key: Hash,
    pub styles: Hash,
    pub opens: u64,
    pub closes: u64,
    pub policy_id: String,
    pub releases: Vec<Release>,
    pub transparency: Visibility,
    pub demo: Demo,
    pub time: u64,
}

impl ConfigStatement {
    /// The note text, before signing.
    pub fn text(&self) -> Result<String> {
        check_id(&self.event_id)?;
        check_id(&self.election_id)?;
        check_policy_id(&self.policy_id)?;
        ensure!(self.opens < self.closes, "a config opens before it closes");
        let mut t = format!(
            "{CONFIG_TYPE}\nevent {}\nelection {}\nkey {}\nstyles {}\nopens {}\ncloses {}\npolicy {}\n",
            self.event_id,
            self.election_id,
            STANDARD.encode(self.key),
            STANDARD.encode(self.styles),
            self.opens,
            self.closes,
            self.policy_id,
        );
        for r in &self.releases {
            check_token("a release package", &r.package)?;
            check_token("a release version", &r.version)?;
            writeln!(t, "release {} {}", r.package, r.version)?;
        }
        writeln!(t, "transparency {}", self.transparency)?;
        writeln!(t, "demo {}", self.demo)?;
        writeln!(t, "time {}", self.time)?;
        Ok(t)
    }
}

/// The `kind` line of a milestone statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MilestoneKind {
    VotingOpened,
    VotingClosed,
    /// A custom milestone, named by a token of `a-z 0-9 -`.
    Other(String),
}

impl MilestoneKind {
    pub const VOTING_OPENED: &'static str = "voting-opened";
    pub const VOTING_CLOSED: &'static str = "voting-closed";

    fn token(&self) -> Result<&str> {
        Ok(match self {
            MilestoneKind::VotingOpened => Self::VOTING_OPENED,
            MilestoneKind::VotingClosed => Self::VOTING_CLOSED,
            MilestoneKind::Other(token) => {
                ensure!(
                    token.starts_with(|c: char| c.is_ascii_lowercase())
                        && token
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                        && token != Self::VOTING_OPENED
                        && token != Self::VOTING_CLOSED,
                    "a custom milestone is a token of a-z 0-9 -: {token:?}"
                );
                token
            }
        })
    }
}

/// A `vitrine/milestone/v1` statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MilestoneStatement {
    pub event_id: String,
    pub kind: MilestoneKind,
    pub time: u64,
    /// `config <election id> <leaf hash>`, for `voting-opened` only.
    pub configs: Vec<(String, Hash)>,
    /// `count <election id> <ballots>`, for `voting-closed` only.
    pub counts: Vec<(String, u64)>,
}

impl MilestoneStatement {
    pub fn text(&self) -> Result<String> {
        check_id(&self.event_id)?;
        ensure!(
            self.configs.is_empty() || self.kind == MilestoneKind::VotingOpened,
            "only voting-opened has config lines"
        );
        ensure!(
            self.counts.is_empty() || self.kind == MilestoneKind::VotingClosed,
            "only voting-closed has count lines"
        );
        let mut t = format!(
            "{MILESTONE_TYPE}\nevent {}\nkind {}\ntime {}\n",
            self.event_id,
            self.kind.token()?,
            self.time
        );
        for (election, leaf) in &self.configs {
            check_id(election)?;
            writeln!(t, "config {election} {}", STANDARD.encode(leaf))?;
        }
        for (election, count) in &self.counts {
            check_id(election)?;
            writeln!(t, "count {election} {count}")?;
        }
        Ok(t)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ConfigStatement {
        ConfigStatement {
            event_id: "muni-2026".into(),
            election_id: "mayor".into(),
            key: [1; 32],
            styles: [2; 32],
            opens: 1_793_001_600,
            closes: 1_793_044_800,
            policy_id: format!("sha256:{}", "ab".repeat(32)),
            releases: vec![Release {
                package: "sequent/voting-portal".into(),
                version: "dev".into(),
            }],
            transparency: Visibility::Public,
            demo: Demo::No,
            time: 1_792_994_400,
        }
    }

    #[test]
    fn config_lines_are_in_the_spec_order() {
        let text = config().text().unwrap();
        let keys: Vec<&str> = text.lines().map(|l| l.split(' ').next().unwrap()).collect();
        assert_eq!(
            keys,
            [
                CONFIG_TYPE,
                "event",
                "election",
                "key",
                "styles",
                "opens",
                "closes",
                "policy",
                "release",
                "transparency",
                "demo",
                "time"
            ]
        );
        assert!(text.contains("\nrelease sequent/voting-portal dev\n"));
    }

    #[test]
    fn config_refuses_bad_values() {
        let mut c = config();
        c.policy_id = "sha256:AB".into();
        assert!(c.text().is_err());
        let mut c = config();
        c.releases[0].version = "1 2".into();
        assert!(c.text().is_err());
        let mut c = config();
        c.closes = c.opens;
        assert!(c.text().is_err());
    }

    #[test]
    fn milestones_carry_lines_only_for_their_kind() {
        let opened = MilestoneStatement {
            event_id: "e".into(),
            kind: MilestoneKind::VotingOpened,
            time: 5,
            configs: vec![("mayor".into(), [0; 32])],
            counts: vec![],
        };
        assert_eq!(
            opened.text().unwrap(),
            format!(
                "{MILESTONE_TYPE}\nevent e\nkind voting-opened\ntime 5\nconfig mayor {}\n",
                STANDARD.encode([0u8; 32])
            )
        );
        let mut wrong = opened.clone();
        wrong.kind = MilestoneKind::VotingClosed;
        assert!(wrong.text().is_err());
        let custom = MilestoneStatement {
            kind: MilestoneKind::Other("voting-opened".into()),
            configs: vec![],
            ..opened.clone()
        };
        assert!(custom.text().is_err());
        let custom = MilestoneStatement {
            kind: MilestoneKind::Other("tally-completed".into()),
            configs: vec![],
            ..opened
        };
        assert!(custom.text().unwrap().contains("\nkind tally-completed\n"));
    }

    #[test]
    fn hashes_follow_the_contract() {
        assert_eq!(
            hex::encode(election_key_hash(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let a = style_digest("{\"area\":\"a\"}");
        let b = style_digest("{\"area\":\"b\"}");
        assert_eq!(styles_hash(&[a, b]), styles_hash(&[b, a]));
        let mut concat = Vec::new();
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        concat.extend_from_slice(&lo);
        concat.extend_from_slice(&hi);
        assert_eq!(styles_hash(&[b, a]), <Hash>::from(Sha256::digest(&concat)));
        assert_eq!(
            hex::encode(styles_hash(&[])),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}

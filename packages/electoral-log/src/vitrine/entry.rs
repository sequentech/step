// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Vitrine entries (Vitrine's `specs/entry.md`, sections 1 to 4): the
//! envelope, the field types step uses, and the `vitrine/election/ballots/v1` and
//! `vitrine/election/events/v1` layouts.

use anyhow::{anyhow, bail, ensure, Context, Result};
use sha2::{Digest, Sha256};
use std::str::FromStr;
use strum_macros::{AsRefStr, Display, EnumString};

use super::note;

/// Entry bundles prefix each entry with a `uint16` length.
pub const MAX_ENTRY_SIZE: usize = 65_535;
pub const ELECTION_TAG: &str = "vitrine/election/ballots/v1";
pub const EVENTS_TAG: &str = "vitrine/election/events/v1";
/// Reviewed Vitrine core meanings for Step's two election logs.
pub const ELECTION_MEANING_HASH: &str =
    "sha256:b77e087d44cf4621cb2ca2cdf982abb5c46f3b964ce79f9ff2b04ef645624ab1";
pub const EVENTS_MEANING_HASH: &str =
    "sha256:a76f20437d3846a9cb930a379fe545c218e9e80f35c186fff6dd79e870b5ba51";

pub const HASH_SIZE: usize = 32;
pub type Hash = [u8; HASH_SIZE];

const KIND_PUBLIC: u8 = 0;
const KIND_COMMITMENT: u8 = 1;
const MAX_TAG: usize = 255;
const MAX_ID: usize = 255;
const TIME_SIZE: usize = 8;

const ELECTION_FIELDS: usize = 5;
const EVENTS_FIELDS: usize = 5;

const SUBJECT_NOTHING: u8 = 0;
const SUBJECT_HASH: u8 = 1;
const SUBJECT_ORIGIN: u8 = 2;
const SUBJECT_REF: u8 = 3;

/// One positional field of an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field {
    Public(Vec<u8>),
    Commitment(Hash),
}

/// The RFC 6962 leaf hash, `SHA-256(0x00 || entry)`.
pub fn leaf_hash(entry: &[u8]) -> Hash {
    let mut h = Sha256::new();
    h.update([0u8]);
    h.update(entry);
    h.finalize().into()
}

fn check_tag(tag: &str) -> Result<()> {
    let b = tag.as_bytes();
    ensure!(
        !b.is_empty() && b.len() <= MAX_TAG,
        "a tag has 1 to 255 bytes"
    );
    ensure!(
        b[0].is_ascii_lowercase(),
        "a tag starts with a lowercase letter"
    );
    ensure!(
        b.iter().all(|c| c.is_ascii_lowercase()
            || c.is_ascii_digit()
            || matches!(c, b'.' | b'_' | b'-' | b'/')),
        "a tag has only a-z 0-9 . _ - /"
    );
    Ok(())
}

/// An `id` field: 1 to 255 bytes of `A-Z a-z 0-9 . _ - : / @ +`.
pub fn check_id(id: &str) -> Result<()> {
    let b = id.as_bytes();
    ensure!(
        !b.is_empty() && b.len() <= MAX_ID,
        "an id has 1 to 255 bytes"
    );
    ensure!(
        b.iter().all(|c| c.is_ascii_alphanumeric()
            || matches!(c, b'.' | b'_' | b'-' | b':' | b'/' | b'@' | b'+')),
        "an id has only A-Z a-z 0-9 . _ - : / @ +: {id:?}"
    );
    Ok(())
}

pub fn encode_entry(tag: &str, fields: &[Field]) -> Result<Vec<u8>> {
    check_tag(tag)?;
    let mut body = Vec::new();
    for field in fields {
        match field {
            Field::Public(value) => {
                let len = u16::try_from(value.len())
                    .map_err(|_| anyhow!("a public field has at most 65535 bytes"))?;
                body.push(KIND_PUBLIC);
                body.extend_from_slice(&len.to_be_bytes());
                body.extend_from_slice(value);
            }
            Field::Commitment(digest) => {
                body.push(KIND_COMMITMENT);
                body.extend_from_slice(digest);
            }
        }
    }
    let body_len =
        u16::try_from(body.len()).map_err(|_| anyhow!("the fields have at most 65535 bytes"))?;
    let mut out = Vec::with_capacity(1 + tag.len() + 2 + body.len());
    out.push(tag.len() as u8);
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(&body_len.to_be_bytes());
    out.extend_from_slice(&body);
    ensure!(
        out.len() <= MAX_ENTRY_SIZE,
        "an entry has at most 65535 bytes"
    );
    Ok(out)
}

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        ensure!(self.0.len() >= n, "truncated entry");
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<usize> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]) as usize)
    }

    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// Splits an entry into its tag and fields. Rejects trailing bytes, unknown
/// field kinds, a field vector whose length doesn't match its contents and a
/// tag outside the alphabet.
pub fn decode_entry(entry: &[u8]) -> Result<(String, Vec<Field>)> {
    ensure!(
        entry.len() <= MAX_ENTRY_SIZE,
        "an entry has at most 65535 bytes"
    );
    let mut r = Reader(entry);
    let tag_len = r.u8()? as usize;
    let tag = std::str::from_utf8(r.take(tag_len)?).context("the tag isn't ASCII")?;
    check_tag(tag)?;
    let fields_len = r.u16()?;
    let mut f = Reader(r.take(fields_len)?);
    ensure!(r.is_empty(), "trailing bytes after the fields");
    let mut fields = Vec::new();
    while !f.is_empty() {
        match f.u8()? {
            KIND_PUBLIC => {
                let len = f.u16()?;
                fields.push(Field::Public(f.take(len)?.to_vec()));
            }
            KIND_COMMITMENT => {
                let digest: Hash = f.take(HASH_SIZE)?.try_into()?;
                fields.push(Field::Commitment(digest));
            }
            kind => bail!("unknown field kind {kind}"),
        }
    }
    Ok((tag.to_string(), fields))
}

fn decode_layout(entry: &[u8], tag: &str, count: usize) -> Result<Vec<Field>> {
    let (found, fields) = decode_entry(entry)?;
    ensure!(found == tag, "the tag is {found}, not {tag}");
    ensure!(
        fields.len() == count,
        "{tag} has {count} fields, not {}",
        fields.len()
    );
    Ok(fields)
}

fn public(field: &Field) -> Result<&[u8]> {
    match field {
        Field::Public(value) => Ok(value),
        Field::Commitment(_) => bail!("a public field holds a commitment"),
    }
}

fn id_field(field: &Field) -> Result<String> {
    let id = std::str::from_utf8(public(field)?).context("an id is ASCII")?;
    check_id(id)?;
    Ok(id.to_string())
}

fn hash_field(field: &Field) -> Result<Hash> {
    public(field)?
        .try_into()
        .map_err(|_| anyhow!("a hash has 32 bytes"))
}

fn time_field(field: &Field) -> Result<u64> {
    let b: [u8; TIME_SIZE] = public(field)?
        .try_into()
        .map_err(|_| anyhow!("a time has 8 bytes"))?;
    Ok(u64::from_be_bytes(b))
}

/// The stage of a ballot an `ballots/v1` entry records.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum Stage {
    /// The ballot box stored the encrypted ballot.
    Received,
    /// The voter cast it.
    Cast,
    /// The voter audited it instead, which spoils it.
    Audited,
}

/// A `vitrine/election/ballots/v1` entry: one stage of a ballot. A ballot has
/// one entry per stage it reaches, all with the same `ballot_id`: its
/// Tracking ID digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ElectionEntry {
    pub event_id: String,
    pub election_id: String,
    pub stage: Stage,
    pub ballot_id: Hash,
    pub accepted_at: u64,
}

impl ElectionEntry {
    pub fn encode(&self) -> Result<Vec<u8>> {
        check_id(&self.event_id)?;
        check_id(&self.election_id)?;
        encode_entry(
            ELECTION_TAG,
            &[
                Field::Public(self.event_id.as_bytes().to_vec()),
                Field::Public(self.election_id.as_bytes().to_vec()),
                Field::Public(self.stage.as_ref().as_bytes().to_vec()),
                Field::Public(self.ballot_id.to_vec()),
                Field::Public(self.accepted_at.to_be_bytes().to_vec()),
            ],
        )
    }

    pub fn decode(entry: &[u8]) -> Result<Self> {
        let f = decode_layout(entry, ELECTION_TAG, ELECTION_FIELDS)?;
        let stage = std::str::from_utf8(public(&f[2])?).context("a stage is ASCII")?;
        Ok(ElectionEntry {
            event_id: id_field(&f[0])?,
            election_id: id_field(&f[1])?,
            stage: Stage::from_str(stage).map_err(|_| anyhow!("unknown stage {stage:?}"))?,
            ballot_id: hash_field(&f[3])?,
            accepted_at: time_field(&f[4])?,
        })
    }
}

/// The `kind` of an `events/v1` entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "kebab-case")]
pub enum EventKind {
    Policy,
    Config,
    VotingOpened,
    VotingClosed,
    Checkpoint,
    TallyCommitment,
    Results,
    Notice,
    Redaction,
    BallotVoid,
    ReadAccess,
    Milestone,
}

/// The `subject` field type: what an `events/v1` entry is about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Subject {
    Nothing,
    Hash(Hash),
    Origin(String),
    Ref { origin: String, index: u64 },
}

impl Subject {
    fn encode(&self) -> Result<Vec<u8>> {
        Ok(match self {
            Subject::Nothing => vec![SUBJECT_NOTHING],
            Subject::Hash(h) => {
                let mut v = vec![SUBJECT_HASH];
                v.extend_from_slice(h);
                v
            }
            Subject::Origin(origin) => {
                check_id(origin)?;
                let mut v = vec![SUBJECT_ORIGIN];
                v.extend_from_slice(origin.as_bytes());
                v
            }
            Subject::Ref { origin, index } => {
                ensure!(
                    !origin.is_empty() && origin.len() <= MAX_ID,
                    "a ref's origin has 1 to 255 bytes"
                );
                let mut v = vec![SUBJECT_REF, origin.len() as u8];
                v.extend_from_slice(origin.as_bytes());
                v.extend_from_slice(&index.to_be_bytes());
                v
            }
        })
    }

    fn decode(value: &[u8]) -> Result<Self> {
        let mut r = Reader(value);
        let subject = match r.u8().context("a subject has a type byte")? {
            SUBJECT_NOTHING => Subject::Nothing,
            SUBJECT_HASH => Subject::Hash(r.take(HASH_SIZE)?.try_into()?),
            SUBJECT_ORIGIN => {
                let origin = std::str::from_utf8(r.take(r.0.len())?)?.to_string();
                check_id(&origin)?;
                Subject::Origin(origin)
            }
            SUBJECT_REF => {
                let len = r.u8()? as usize;
                ensure!(len > 0, "a ref's origin has 1 to 255 bytes");
                let origin = std::str::from_utf8(r.take(len)?)?.to_string();
                let index: [u8; 8] = r.take(8)?.try_into()?;
                Subject::Ref {
                    origin,
                    index: u64::from_be_bytes(index),
                }
            }
            other => bail!("unknown subject type {other}"),
        };
        ensure!(r.is_empty(), "trailing bytes in the subject");
        Ok(subject)
    }
}

/// A `vitrine/election/events/v1` entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventsEntry {
    pub kind: EventKind,
    pub time: u64,
    pub subject: Subject,
    pub statement: Option<String>,
    pub details: Option<Hash>,
}

impl EventsEntry {
    pub fn encode(&self) -> Result<Vec<u8>> {
        if let Some(statement) = &self.statement {
            note::check_note(statement)?;
        }
        encode_entry(
            EVENTS_TAG,
            &[
                Field::Public(self.kind.as_ref().as_bytes().to_vec()),
                Field::Public(self.time.to_be_bytes().to_vec()),
                Field::Public(self.subject.encode()?),
                Field::Public(
                    self.statement
                        .as_deref()
                        .map(|s| s.as_bytes().to_vec())
                        .unwrap_or_default(),
                ),
                match self.details {
                    Some(digest) => Field::Commitment(digest),
                    None => Field::Public(Vec::new()),
                },
            ],
        )
    }

    pub fn decode(entry: &[u8]) -> Result<Self> {
        let f = decode_layout(entry, EVENTS_TAG, EVENTS_FIELDS)?;
        let kind = std::str::from_utf8(public(&f[0])?).context("a kind is ASCII")?;
        let kind = EventKind::from_str(kind).map_err(|_| anyhow!("unknown kind {kind:?}"))?;
        let statement = match public(&f[3])? {
            [] => None,
            bytes => {
                let s = std::str::from_utf8(bytes).context("a statement is UTF-8")?;
                note::check_note(s)?;
                Some(s.to_string())
            }
        };
        let details = match &f[4] {
            Field::Commitment(digest) => Some(*digest),
            Field::Public(value) if value.is_empty() => None,
            Field::Public(_) => bail!("details are a commitment"),
        };
        Ok(EventsEntry {
            kind,
            time: time_field(&f[1])?,
            subject: Subject::decode(public(&f[2])?)?,
            statement,
            details,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry() -> ElectionEntry {
        ElectionEntry {
            event_id: "muni-2026".into(),
            election_id: "mayor".into(),
            stage: Stage::Cast,
            ballot_id: [7u8; 32],
            accepted_at: 1_793_001_660,
        }
    }

    #[test]
    fn election_entries_round_trip() {
        let e = entry();
        assert_eq!(ElectionEntry::decode(&e.encode().unwrap()).unwrap(), e);
    }

    #[test]
    fn the_leaf_hash_is_rfc_6962() {
        let mut h = Sha256::new();
        h.update([0u8]);
        let empty: Hash = h.finalize().into();
        assert_eq!(leaf_hash(&[]), empty);
    }

    #[test]
    fn trailing_bytes_and_unknown_kinds_are_rejected() {
        let mut bytes = entry().encode().unwrap();
        bytes.push(0);
        assert!(decode_entry(&bytes).is_err());
        assert!(decode_entry(&[1, b't', 0, 1, 2]).is_err());
        assert!(decode_entry(&[1, b't', 0, 2, 0]).is_err());
    }

    #[test]
    fn ids_and_tags_keep_to_their_alphabets() {
        assert!(check_id("a3e1c6b0-4c47-4bd8-9c4e-5f7f5f0bb5ab").is_ok());
        assert!(check_id("").is_err());
        assert!(check_id("with space").is_err());
        assert!(check_id(&"a".repeat(256)).is_err());
        assert!(encode_entry("Vitrine/x", &[]).is_err());
        assert!(encode_entry("1x", &[]).is_err());
        let mut e = entry();
        e.event_id = "muni\u{200b}2026".into();
        assert!(e.encode().is_err());
    }

    #[test]
    fn a_stage_is_one_of_three_tokens() {
        for (stage, token) in [
            (Stage::Received, "received"),
            (Stage::Cast, "cast"),
            (Stage::Audited, "audited"),
        ] {
            let mut e = entry();
            e.stage = stage;
            let bytes = e.encode().unwrap();
            assert!(bytes.windows(token.len()).any(|w| w == token.as_bytes()));
            assert_eq!(ElectionEntry::decode(&bytes).unwrap(), e);
        }
        let mut e = entry();
        e.stage = Stage::Received;
        let bytes = e.encode().unwrap();
        let other = String::from_utf8_lossy(&bytes).replace("received", "RECEIVED");
        assert!(ElectionEntry::decode(other.as_bytes()).is_err());
    }

    #[test]
    fn a_ballot_has_one_entry_per_stage_under_one_ballot_id() {
        let mut received = entry();
        received.stage = Stage::Received;
        let mut cast = received.clone();
        cast.stage = Stage::Cast;
        assert_ne!(received.encode().unwrap(), cast.encode().unwrap());
        assert_eq!(received.ballot_id, cast.ballot_id);
    }

    #[test]
    fn subjects_round_trip() {
        for subject in [
            Subject::Nothing,
            Subject::Hash([1; 32]),
            Subject::Origin("vote.example/muni/ballots".into()),
            Subject::Ref {
                origin: "vote.example/muni/events".into(),
                index: 4,
            },
        ] {
            assert_eq!(
                Subject::decode(&subject.encode().unwrap()).unwrap(),
                subject
            );
        }
        assert!(Subject::decode(&[]).is_err());
        assert!(Subject::decode(&[SUBJECT_NOTHING, 0]).is_err());
    }

    #[test]
    fn events_details_must_be_a_commitment() {
        let e = EventsEntry {
            kind: EventKind::Milestone,
            time: 1,
            subject: Subject::Nothing,
            statement: None,
            details: Some([3; 32]),
        };
        assert_eq!(EventsEntry::decode(&e.encode().unwrap()).unwrap(), e);
        let public_details = encode_entry(
            EVENTS_TAG,
            &[
                Field::Public(b"notice".to_vec()),
                Field::Public(1u64.to_be_bytes().to_vec()),
                Field::Public(vec![SUBJECT_NOTHING]),
                Field::Public(Vec::new()),
                Field::Public(b"text".to_vec()),
            ],
        )
        .unwrap();
        assert!(EventsEntry::decode(&public_details).is_err());
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Automatic checks of the voting stage, decided from the records the
//! platform already keeps of a live election event.
//!
//! Findings about a voter never carry a Ballot ID and findings about a ballot
//! never name its voter. In a pseudonymous election event the checks never
//! link a voter to a ballot at all: ballots are found by the Ballot IDs the
//! voters were shown, and voters by their participation records.
use super::{
    definition::Probe,
    ledger::{Finding, Subject},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use electoral_log::adapters::privacy::{normalize_privacy_code, BallotBoxPrivacy, BallotIdentity};
use sequent_core::{
    ballot::{HashableBallot, SignedHashableBallot},
    encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512},
    multi_ballot::{HashableMultiBallot, SignedHashableMultiBallot},
    types::{keycloak::Role, permissions::Permissions, vote_privacy},
};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voter {
    pub id: String,
    pub username: String,
    pub enabled: bool,
    pub area_id: Option<String>,
    pub area_name: Option<String>,
}

/// A Keycloak event of a voter in the event's log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserEvent {
    pub at: DateTime<Utc>,
    pub kind: String,
    pub error: bool,
}

/// A valid cast vote as stored in the ballot box.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredBallot {
    pub voter_id: String,
    pub ballot_id: String,
    /// The Ballot ID of the receipt the ballot box signed when it received the
    /// ballot, for ballots cast with signed receipts.
    pub receipt_ballot_id: Option<String>,
    pub content: String,
    pub cast_at: DateTime<Utc>,
}

impl StoredBallot {
    /// Whether a Ballot ID a voter was shown names this ballot.
    fn presented_as(&self, id: &str) -> bool {
        same_id(&self.ballot_id, id)
            || self
                .receipt_ballot_id
                .as_deref()
                .is_some_and(|receipt| same_id(receipt, id))
    }
}

/// A valid cast ballot of a pseudonymous election event, found by the Ballot
/// ID its voter was shown. It does not say who cast it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastBallot {
    /// The short Ballot ID of its receipt.
    pub ballot_id: String,
    /// Hex of the ballot hash the ballot box stored it under.
    pub ballot_hash: String,
    pub content: String,
    pub cast_at: DateTime<Utc>,
}

/// A cast vote statement in the event's log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoggedCast {
    pub ballot_id: String,
    /// Hex of the cast vote hash the ballot box signed.
    pub vote_hash: String,
}

/// The privacy settings of the election event's presentation; unset values
/// keep the platform's defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivacyPolicy {
    pub ballot_identity: Option<vote_privacy::BallotIdentity>,
    pub network_data_policy: Option<vote_privacy::NetworkDataPolicy>,
}

/// What the checks read. The Hasura implementation is in `super::hasura`.
pub trait VotingRecords {
    fn voter(&self, event: &str, username: &str) -> Result<Option<Voter>>;
    fn user_events(&self, event: &str, voter_id: &str) -> Result<Vec<UserEvent>>;
    fn published_areas(&self, event: &str, areas: &[String]) -> Result<BTreeSet<String>>;
    fn ballot_identity(&self, event: &str) -> Result<BallotIdentity>;
    /// Voter-id events only.
    fn stored_ballots(&self, event: &str, voter_ids: &[String]) -> Result<Vec<StoredBallot>>;
    /// Voter-id events only.
    fn logged_casts(&self, event: &str, voter_id: &str) -> Result<Vec<LoggedCast>>;
    /// Pseudonymous events: which of the voters voted.
    fn participants(&self, event: &str, voter_ids: &[String]) -> Result<BTreeSet<String>>;
    /// Pseudonymous events: the valid cast ballots with these Ballot IDs.
    fn cast_ballots(&self, event: &str, ballot_ids: &[String]) -> Result<Vec<CastBallot>>;
    /// Pseudonymous events: Ballot IDs of the cast vote statements in the
    /// event's log since `since`.
    fn private_casts(&self, event: &str, since: DateTime<Utc>) -> Result<BTreeSet<String>>;
    fn privacy_policy(&self, event: &str) -> Result<PrivacyPolicy>;
    /// Pseudonymous events: counts of the ballot-box rows that break pseudonymity.
    fn ballot_box_privacy(&self, event: &str) -> Result<BallotBoxPrivacy>;
    /// The tenant's roles with their permissions.
    fn roles(&self) -> Result<Vec<Role>>;
}

/// Keycloak's event for a completed sign-in.
const SIGN_IN_EVENT: &str = "LOGIN";

#[derive(Debug, Default)]
pub struct VoterFacts {
    pub username: String,
    pub voter: Option<Voter>,
    pub events: Vec<UserEvent>,
    pub ballots: Vec<StoredBallot>,
    pub logged: Vec<LoggedCast>,
    /// Pseudonymous events: whether the voter has voted in the event.
    pub participated: bool,
}

/// Everything the probes need, read once so all checks judge the same state.
#[derive(Debug)]
pub struct Facts {
    pub since: DateTime<Utc>,
    pub identity: BallotIdentity,
    pub voters: Vec<VoterFacts>,
    pub published_areas: BTreeSet<String>,
    /// Ballot IDs read from the voters' confirmation screens or receipts.
    pub presented_ballot_ids: Vec<String>,
    /// Pseudonymous events: the presented ballots cast since `since`.
    pub cast_ballots: Vec<CastBallot>,
    /// Pseudonymous events: Ballot IDs with a cast vote statement in the log.
    pub private_casts: BTreeSet<String>,
    /// Read only for runs with a privacy check.
    pub privacy: Option<PrivacyFacts>,
}

/// What the privacy checks read: settings and counts, never a voter or a ballot.
#[derive(Debug)]
pub struct PrivacyFacts {
    pub policy: PrivacyPolicy,
    /// Not read for voter-id events, whose ballots name their voters anyway.
    pub ballot_box: Option<BallotBoxPrivacy>,
    /// Why the roles could not be read, if they could not.
    pub roles: Result<Vec<Role>, String>,
}

impl PrivacyFacts {
    pub fn gather(
        records: &impl VotingRecords,
        event: &str,
        identity: BallotIdentity,
    ) -> Result<Self> {
        Ok(Self {
            policy: records.privacy_policy(event)?,
            ballot_box: match identity {
                BallotIdentity::Pseudonym => Some(records.ballot_box_privacy(event)?),
                BallotIdentity::VoterId => None,
            },
            roles: records.roles().map_err(|error| format!("{error:#}")),
        })
    }
}

impl Facts {
    /// Records older than `since` belong to an earlier run and are left out.
    pub fn gather(
        records: &impl VotingRecords,
        event: &str,
        usernames: &[String],
        presented_ballot_ids: Vec<String>,
        since: DateTime<Utc>,
    ) -> Result<Self> {
        let identity = records.ballot_identity(event)?;
        let mut voters = Vec::new();
        for username in usernames {
            let voter = records.voter(event, username)?;
            let (events, logged) = match &voter {
                Some(voter) => (
                    records.user_events(event, &voter.id)?,
                    match identity {
                        BallotIdentity::VoterId => records.logged_casts(event, &voter.id)?,
                        BallotIdentity::Pseudonym => vec![],
                    },
                ),
                None => Default::default(),
            };
            voters.push(VoterFacts {
                username: username.clone(),
                voter,
                events: events.into_iter().filter(|e| e.at >= since).collect(),
                ballots: vec![],
                logged,
                participated: false,
            });
        }
        let found = |facts: &VoterFacts| facts.voter.clone();
        let ids: Vec<String> = voters.iter().filter_map(found).map(|v| v.id).collect();
        let areas: BTreeSet<String> = voters
            .iter()
            .filter_map(found)
            .filter_map(|voter| voter.area_id)
            .collect();
        let areas: Vec<String> = areas.into_iter().collect();
        let published_areas = if areas.is_empty() {
            BTreeSet::new()
        } else {
            records.published_areas(event, &areas)?
        };
        let mut cast_ballots = vec![];
        let mut private_casts = BTreeSet::new();
        match identity {
            BallotIdentity::VoterId if !ids.is_empty() => {
                for ballot in records.stored_ballots(event, &ids)? {
                    if ballot.cast_at < since {
                        continue;
                    }
                    if let Some(facts) = voters.iter_mut().find(|facts| {
                        facts
                            .voter
                            .as_ref()
                            .is_some_and(|voter| voter.id == ballot.voter_id)
                    }) {
                        facts.ballots.push(ballot);
                    }
                }
            }
            BallotIdentity::VoterId => {}
            BallotIdentity::Pseudonym => {
                if !ids.is_empty() {
                    let participants = records.participants(event, &ids)?;
                    for facts in &mut voters {
                        facts.participated = facts
                            .voter
                            .as_ref()
                            .is_some_and(|voter| participants.contains(&voter.id));
                    }
                }
                let codes: BTreeSet<String> = presented_ballot_ids
                    .iter()
                    .map(|id| privacy_code(id))
                    .collect();
                if !codes.is_empty() {
                    let codes: Vec<String> = codes.into_iter().collect();
                    cast_ballots = records
                        .cast_ballots(event, &codes)?
                        .into_iter()
                        .filter(|ballot| ballot.cast_at >= since)
                        .collect();
                    private_casts = records.private_casts(event, since)?;
                }
            }
        }
        Ok(Self {
            since,
            identity,
            voters,
            published_areas,
            presented_ballot_ids,
            cast_ballots,
            private_casts,
            privacy: None,
        })
    }

    /// Pseudonymous events: the cast ballot a presented Ballot ID names.
    fn cast_ballot(&self, presented: &str) -> Option<&CastBallot> {
        let code = privacy_code(presented);
        self.cast_ballots
            .iter()
            .find(|ballot| same_id(&ballot.ballot_id, &code))
    }

    fn ballots(&self) -> impl Iterator<Item = (&VoterFacts, &StoredBallot)> {
        self.voters
            .iter()
            .flat_map(|facts| facts.ballots.iter().map(move |ballot| (facts, ballot)))
    }
}

/// The findings of one check, or why it cannot be decided with what the operator gave.
pub fn probe(probe: Probe, facts: &Facts) -> Result<Vec<Finding>, String> {
    if probe.needs_voters() && facts.voters.is_empty() {
        return Err("no voters were given".into());
    }
    let privacy = || {
        facts
            .privacy
            .as_ref()
            .ok_or_else(|| "the privacy settings were not read".to_string())
    };
    match (probe, facts.identity) {
        (Probe::PseudonymousBallots, identity) => Ok(pseudonymous_ballots(identity, privacy()?)),
        (Probe::NetworkDataNotStored, identity) => {
            Ok(network_data_not_stored(identity, privacy()?))
        }
        (Probe::BallotBoxUnlinked, _) => Ok(ballot_box_unlinked(privacy()?)),
        (Probe::PrivilegedRoles, _) => privileged_roles(privacy()?),
        (Probe::VoterAuthenticated, _) => Ok(voter_authenticated(facts)),
        (Probe::BallotPublished, _) => Ok(ballot_published(facts)),
        (Probe::BallotCast, BallotIdentity::VoterId) => Ok(ballot_cast(facts)),
        (Probe::BallotCast, BallotIdentity::Pseudonym) => Ok(participation(facts)),
        (Probe::ReceiptProduced, BallotIdentity::VoterId) => receipt_produced(facts),
        (Probe::ReceiptProduced, BallotIdentity::Pseudonym) => receipt_by_ballot_id(facts),
        (Probe::BallotStored, BallotIdentity::VoterId) => Ok(ballot_stored(facts)),
        (Probe::BallotStored, BallotIdentity::Pseudonym) => ballot_stored_by_ballot_id(facts),
    }
}

const NOT_A_VOTER: &str = "is not a voter of this election event";

const NO_BALLOT_IDS: &str = "no Ballot IDs from confirmation screens or receipts were given";

const NO_CAST_BALLOTS: &str = "the checked voters have no cast ballots";

/// A presented Ballot ID in the form pseudonymous receipts show it, or as
/// given when it is not one.
fn privacy_code(id: &str) -> String {
    normalize_privacy_code(id).unwrap_or_else(|| id.trim().to_owned())
}

/// Name of findings about the voters of a run as a whole.
const CHECKED_VOTERS: &str = "checked voters";

fn voter_authenticated(facts: &Facts) -> Vec<Finding> {
    facts
        .voters
        .iter()
        .map(|facts_of| {
            let name = facts_of.username.as_str();
            let Some(voter) = &facts_of.voter else {
                return Finding::fail(Subject::Voter, name, NOT_A_VOTER);
            };
            if !voter.enabled {
                return Finding::fail(Subject::Voter, name, "is disabled");
            }
            let sign_in = facts_of
                .events
                .iter()
                .filter(|event| event.kind == SIGN_IN_EVENT && !event.error)
                .map(|event| event.at)
                .min();
            match sign_in {
                Some(at) => Finding::pass(
                    Subject::Voter,
                    name,
                    format!("signed in at {}", at.to_rfc3339()),
                ),
                None => {
                    let errors = facts_of.events.iter().filter(|event| event.error).count();
                    Finding::fail(
                        Subject::Voter,
                        name,
                        format!(
                            "no successful sign-in since {}; failed authentication events: {errors}",
                            facts.since.to_rfc3339()
                        ),
                    )
                }
            }
        })
        .collect()
}

fn ballot_published(facts: &Facts) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut seen = BTreeSet::new();
    for facts_of in &facts.voters {
        let name = facts_of.username.as_str();
        let Some(voter) = &facts_of.voter else {
            findings.push(Finding::fail(Subject::Voter, name, NOT_A_VOTER));
            continue;
        };
        let Some(area) = &voter.area_id else {
            findings.push(Finding::fail(Subject::Voter, name, "has no area"));
            continue;
        };
        if !seen.insert(area) {
            continue;
        }
        let label = voter.area_name.as_deref().unwrap_or(area);
        findings.push(if facts.published_areas.contains(area) {
            Finding::pass(Subject::Area, label, "has a published ballot style")
        } else {
            Finding::fail(Subject::Area, label, "has no published ballot style")
        });
    }
    findings
}

fn ballot_cast(facts: &Facts) -> Vec<Finding> {
    facts
        .voters
        .iter()
        .map(|facts_of| {
            let name = facts_of.username.as_str();
            if facts_of.voter.is_none() {
                return Finding::fail(Subject::Voter, name, NOT_A_VOTER);
            }
            if facts_of.ballots.is_empty() {
                Finding::fail(
                    Subject::Voter,
                    name,
                    format!("no valid cast vote since {}", facts.since.to_rfc3339()),
                )
            } else {
                Finding::pass(
                    Subject::Voter,
                    name,
                    format!("valid cast votes: {}", facts_of.ballots.len()),
                )
            }
        })
        .collect()
}

fn same_id(left: &str, right: &str) -> bool {
    left.trim().eq_ignore_ascii_case(right.trim())
}

fn receipt_produced(facts: &Facts) -> Result<Vec<Finding>, String> {
    let presented = &facts.presented_ballot_ids;
    if presented.is_empty() {
        return Err(NO_BALLOT_IDS.into());
    }
    let mut findings: Vec<Finding> = presented
        .iter()
        .map(|id| {
            if facts.ballots().any(|(_, ballot)| ballot.presented_as(id)) {
                Finding::pass(
                    Subject::Ballot,
                    id,
                    "is a stored ballot of the checked voters",
                )
            } else {
                Finding::fail(
                    Subject::Ballot,
                    id,
                    "is not a stored ballot of the checked voters",
                )
            }
        })
        .collect();
    let stored = facts.ballots().count();
    let missing = facts
        .ballots()
        .filter(|(_, ballot)| !presented.iter().any(|id| ballot.presented_as(id)))
        .count();
    findings.push(if stored == 0 {
        Finding::fail(Subject::Stage, CHECKED_VOTERS, NO_CAST_BALLOTS)
    } else if missing > 0 {
        Finding::fail(
            Subject::Stage,
            CHECKED_VOTERS,
            format!("cast ballots without a presented Ballot ID: {missing} of {stored}"),
        )
    } else {
        Finding::pass(
            Subject::Stage,
            CHECKED_VOTERS,
            format!("the Ballot ID of every cast ballot was presented: {stored}"),
        )
    });
    Ok(findings)
}

/// Pseudonymous events: whether each voter voted, from the participation
/// records. They carry no time, so a vote before `since` counts too.
fn participation(facts: &Facts) -> Vec<Finding> {
    facts
        .voters
        .iter()
        .map(|facts_of| {
            let name = facts_of.username.as_str();
            if facts_of.voter.is_none() {
                Finding::fail(Subject::Voter, name, NOT_A_VOTER)
            } else if facts_of.participated {
                Finding::pass(Subject::Voter, name, "has voted in this election event")
            } else {
                Finding::fail(Subject::Voter, name, "has not voted in this election event")
            }
        })
        .collect()
}

/// Pseudonymous events: every presented Ballot ID names a ballot cast in this
/// run, and there are at least as many of them as checked voters who voted.
/// Without linking voters to ballots, which ballot is whose is not known.
fn receipt_by_ballot_id(facts: &Facts) -> Result<Vec<Finding>, String> {
    let presented = &facts.presented_ballot_ids;
    if presented.is_empty() {
        return Err(NO_BALLOT_IDS.into());
    }
    let mut matched = BTreeSet::new();
    let mut findings: Vec<Finding> = presented
        .iter()
        .map(|id| match facts.cast_ballot(id) {
            Some(ballot) => {
                matched.insert(ballot.ballot_id.as_str());
                Finding::pass(
                    Subject::Ballot,
                    id,
                    "is a ballot cast in this election event",
                )
            }
            None => Finding::fail(
                Subject::Ballot,
                id,
                format!(
                    "is not a ballot cast in this election event since {}",
                    facts.since.to_rfc3339()
                ),
            ),
        })
        .collect();
    let voted = facts.voters.iter().filter(|f| f.participated).count();
    let matched = matched.len();
    findings.push(if voted == 0 {
        Finding::fail(Subject::Stage, CHECKED_VOTERS, NO_CAST_BALLOTS)
    } else if matched < voted {
        Finding::fail(
            Subject::Stage,
            CHECKED_VOTERS,
            format!("presented Ballot IDs of cast ballots: {matched}, fewer than the checked voters who voted: {voted}"),
        )
    } else {
        Finding::pass(
            Subject::Stage,
            CHECKED_VOTERS,
            format!("presented Ballot IDs of cast ballots: {matched}; checked voters who voted: {voted}"),
        )
    });
    Ok(findings)
}

/// SHA-512 of a stored ballot, as the ballot box hashes it when it is cast.
/// The content does not say whether it holds one ballot per contest or one
/// for all contests, so every reading that parses is returned.
pub fn content_hashes(content: &str) -> Vec<String> {
    let mut hashes = Vec::new();
    if let Ok(signed) = serde_json::from_str::<SignedHashableBallot>(content) {
        if let Some(hash) = HashableBallot::try_from(&signed)
            .ok()
            .and_then(|ballot| hash_ballot_sha512(&ballot).ok())
        {
            hashes.push(hex::encode(hash));
        }
    }
    if let Ok(signed) = serde_json::from_str::<SignedHashableMultiBallot>(content) {
        if let Some(hash) = HashableMultiBallot::try_from(&signed)
            .ok()
            .and_then(|ballot| hash_multi_ballot_sha512(&ballot).ok())
        {
            hashes.push(hex::encode(hash));
        }
    }
    hashes
}

fn ballot_stored(facts: &Facts) -> Vec<Finding> {
    let mut findings: Vec<Finding> = facts
        .ballots()
        .map(|(facts_of, ballot)| {
            let id = ballot.ballot_id.as_str();
            let hashes = content_hashes(&ballot.content);
            if hashes.is_empty() {
                return Finding::fail(Subject::Ballot, id, "the stored content cannot be read");
            }
            let mut logged = facts_of
                .logged
                .iter()
                .filter(|cast| same_id(&cast.ballot_id, id))
                .peekable();
            if logged.peek().is_none() {
                return Finding::fail(
                    Subject::Ballot,
                    id,
                    "the election event's log has no cast vote entry for this Ballot ID",
                );
            }
            if logged.any(|cast| hashes.iter().any(|hash| same_id(hash, &cast.vote_hash))) {
                Finding::pass(
                    Subject::Ballot,
                    id,
                    "the stored content matches the hash in the election event's log",
                )
            } else {
                Finding::fail(
                    Subject::Ballot,
                    id,
                    "the stored content does not match the hash in the election event's log",
                )
            }
        })
        .collect();
    if findings.is_empty() {
        findings.push(Finding::fail(
            Subject::Stage,
            CHECKED_VOTERS,
            NO_CAST_BALLOTS,
        ));
    }
    findings
}

/// Pseudonymous events: each presented cast ballot is stored under the hash
/// of its content and its cast vote statement is in the event's log.
fn ballot_stored_by_ballot_id(facts: &Facts) -> Result<Vec<Finding>, String> {
    if facts.presented_ballot_ids.is_empty() {
        return Err(NO_BALLOT_IDS.into());
    }
    let mut findings: Vec<Finding> = facts
        .cast_ballots
        .iter()
        .map(|ballot| {
            let id = ballot.ballot_id.as_str();
            let hashes = content_hashes(&ballot.content);
            if hashes.is_empty() {
                return Finding::fail(Subject::Ballot, id, "the stored content cannot be read");
            }
            if !hashes.iter().any(|hash| names_hash(&ballot.ballot_hash, hash)) {
                return Finding::fail(
                    Subject::Ballot,
                    id,
                    "the stored content does not match the ballot hash it is stored under",
                );
            }
            if !facts.private_casts.iter().any(|logged| same_id(logged, id)) {
                return Finding::fail(
                    Subject::Ballot,
                    id,
                    "the election event's log has no cast vote entry for this Ballot ID",
                );
            }
            Finding::pass(
                Subject::Ballot,
                id,
                "the stored content matches its ballot hash and the election event's log has its cast vote entry",
            )
        })
        .collect();
    if findings.is_empty() {
        findings.push(Finding::fail(
            Subject::Stage,
            CHECKED_VOTERS,
            "no presented Ballot ID names a cast ballot",
        ));
    }
    Ok(findings)
}

/// Name of findings about the election event's settings.
const ELECTION_EVENT: &str = "election event";

/// Name of findings about the ballot box as a whole.
const BALLOT_BOX: &str = "ballot box";

const VOTER_ID_BALLOTS: &str =
    "the ballot box stores ballots under voter ids (ballot identity voter-id), so each ballot names its voter";

fn identity_name(identity: Option<vote_privacy::BallotIdentity>) -> &'static str {
    match identity {
        Some(vote_privacy::BallotIdentity::Pseudonym) => "pseudonym",
        Some(vote_privacy::BallotIdentity::VoterId) => "voter-id",
        None => "not set (voter-id)",
    }
}

/// The ballot box and the event's presentation agree that ballots carry pseudonyms.
fn pseudonymous_ballots(identity: BallotIdentity, privacy: &PrivacyFacts) -> Vec<Finding> {
    let presented = privacy.policy.ballot_identity;
    let detail = format!(
        "presentation.ballot_identity is {}",
        identity_name(presented)
    );
    vec![match identity {
        BallotIdentity::VoterId => Finding::fail(
            Subject::Stage,
            ELECTION_EVENT,
            format!("{VOTER_ID_BALLOTS}; {detail}"),
        ),
        BallotIdentity::Pseudonym if presented != Some(vote_privacy::BallotIdentity::Pseudonym) => {
            Finding::fail(
                Subject::Stage,
                ELECTION_EVENT,
                format!("the ballot box stores ballots under pseudonyms but {detail}"),
            )
        }
        BallotIdentity::Pseudonym => Finding::pass(
            Subject::Stage,
            ELECTION_EVENT,
            format!("the ballot box stores ballots under pseudonyms and {detail}"),
        ),
    }]
}

/// Under `forensic-store` the IP address and country of each cast are kept in
/// the forensic tables next to its pseudonym and Ballot ID, so a cast can be
/// traced to a network and, through it, to a person.
fn network_data_not_stored(identity: BallotIdentity, privacy: &PrivacyFacts) -> Vec<Finding> {
    use vote_privacy::NetworkDataPolicy;
    let policy = privacy.policy.network_data_policy;
    vec![match (identity, policy.unwrap_or_default()) {
        (BallotIdentity::VoterId, _) => Finding::fail(
            Subject::Stage,
            ELECTION_EVENT,
            format!("{VOTER_ID_BALLOTS}, with its IP address and country"),
        ),
        (BallotIdentity::Pseudonym, NetworkDataPolicy::ForensicStore) => Finding::fail(
            Subject::Stage,
            ELECTION_EVENT,
            "presentation.network_data_policy is forensic-store: the IP address and country of each cast are kept in the forensic tables with its pseudonym and Ballot ID",
        ),
        (BallotIdentity::Pseudonym, NetworkDataPolicy::DoNotStore) => Finding::pass(
            Subject::Stage,
            ELECTION_EVENT,
            match policy {
                Some(_) => "presentation.network_data_policy is do-not-store",
                None => "presentation.network_data_policy is not set (do-not-store)",
            },
        ),
    }]
}

fn ballot_box_unlinked(privacy: &PrivacyFacts) -> Vec<Finding> {
    let Some(counts) = &privacy.ballot_box else {
        return vec![Finding::fail(Subject::Stage, BALLOT_BOX, VOTER_ID_BALLOTS)];
    };
    let checks = [
        (
            counts.ballots_with_network_data,
            counts.ballots,
            "ballots with an IP address or country",
        ),
        (
            counts.ballots_with_username,
            counts.ballots,
            "ballots with a username other than their pseudonym",
        ),
        (
            counts.ballots_without_pseudonym,
            counts.ballots,
            "ballots whose voter id is not an allocated pseudonym",
        ),
        (
            counts.received_without_pseudonym,
            counts.received,
            "received ballots whose voter id is not an allocated pseudonym",
        ),
    ];
    checks
        .into_iter()
        .map(|(found, of, what)| {
            let detail = format!("{what}: {found} of {of}");
            if found == 0 {
                Finding::pass(Subject::Stage, BALLOT_BOX, detail)
            } else {
                Finding::fail(Subject::Stage, BALLOT_BOX, detail)
            }
        })
        .collect()
}

/// Informational: who holds the roles is not in the event's records.
fn privileged_roles(privacy: &PrivacyFacts) -> Result<Vec<Finding>, String> {
    let roles = privacy
        .roles
        .as_ref()
        .map_err(|error| format!("the tenant's roles cannot be read: {error}"))?;
    Ok([
        Permissions::VOTER_PSEUDONYM_RESOLVE,
        Permissions::FORENSIC_READ,
    ]
    .iter()
    .map(|permission| {
        let permission = permission.to_string();
        let holders: Vec<&str> = roles
            .iter()
            .filter(|role| {
                role.permissions
                    .as_ref()
                    .is_some_and(|granted| granted.contains(&permission))
            })
            .filter_map(|role| role.name.as_deref())
            .collect();
        let detail = if holders.is_empty() {
            "no role holds it".to_string()
        } else {
            format!("held by roles: {}", holders.join(", "))
        };
        Finding::pass(Subject::Stage, &permission, detail)
    })
    .collect())
}

/// Whether a ballot hash, SHA-512 truncated to 256 bits, is that of the full
/// `sha512` hex digest.
fn names_hash(ballot_hash: &str, sha512: &str) -> bool {
    ballot_hash.len() == sha512.len() / 2
        && sha512
            .get(..ballot_hash.len())
            .is_some_and(|prefix| same_id(prefix, ballot_hash))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::acceptance::ledger::Outcome;
    use chrono::{Duration, TimeZone};
    use sequent_core::{
        encrypt::encrypt_decoded_contest,
        fixtures::ballot_codec::{get_writein_ballot_style, get_writein_plaintext},
    };
    use std::collections::BTreeMap;
    use voting_crypto::RistrettoCtx;

    pub fn since() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2028, 1, 10, 9, 0, 0).unwrap()
    }

    /// A freshly encrypted ballot: (Ballot ID, content, logged hash).
    pub fn cast_ballot() -> (String, String, String) {
        let style = get_writein_ballot_style();
        let choices = vec![get_writein_plaintext()];
        let ballot = encrypt_decoded_contest(&RistrettoCtx, &choices, &style).unwrap();
        let signed = SignedHashableBallot::try_from(&ballot).unwrap();
        let hashable = HashableBallot::try_from(&signed).unwrap();
        (
            ballot.ballot_hash.clone(),
            serde_json::to_string(&signed).unwrap(),
            hex::encode(hash_ballot_sha512(&hashable).unwrap()),
        )
    }

    /// In-memory election event: two voters of one area who signed in and voted.
    #[derive(Clone)]
    pub struct Event {
        pub voters: Vec<Voter>,
        pub events: BTreeMap<String, Vec<UserEvent>>,
        pub published: BTreeSet<String>,
        pub ballots: Vec<StoredBallot>,
        pub logged: BTreeMap<String, Vec<LoggedCast>>,
        /// Ballots carry pseudonyms; each one's receipt Ballot ID is its code.
        pub pseudonymous: bool,
        pub private_casts: BTreeSet<String>,
        pub policy: PrivacyPolicy,
        pub ballot_box: BallotBoxPrivacy,
        /// The tenant's roles, or why they cannot be read.
        pub roles: Result<Vec<Role>, String>,
    }

    pub fn role(name: &str, permissions: &[&str]) -> Role {
        Role {
            id: Some(format!("id-{name}")),
            name: Some(name.into()),
            permissions: Some(permissions.iter().map(|p| (*p).into()).collect()),
            access: None,
            attributes: None,
            client_roles: None,
        }
    }

    impl Default for Event {
        fn default() -> Self {
            Self {
                voters: vec![],
                events: BTreeMap::new(),
                published: BTreeSet::new(),
                ballots: vec![],
                logged: BTreeMap::new(),
                pseudonymous: false,
                private_casts: BTreeSet::new(),
                policy: PrivacyPolicy::default(),
                ballot_box: BallotBoxPrivacy::default(),
                roles: Ok(vec![
                    role("admin", &["voter-pseudonym-resolve", "forensic-read"]),
                    role("auditor", &["forensic-read"]),
                    role("support", &["user-read"]),
                ]),
            }
        }
    }

    impl Event {
        pub fn voted(usernames: &[&str]) -> Self {
            let mut event = Self::default();
            event.published.insert("area-1".into());
            for username in usernames {
                let id = format!("id-{username}");
                event.voters.push(Voter {
                    id: id.clone(),
                    username: (*username).into(),
                    enabled: true,
                    area_id: Some("area-1".into()),
                    area_name: Some("Area One".into()),
                });
                event.events.insert(
                    id.clone(),
                    vec![UserEvent {
                        at: since() + Duration::minutes(1),
                        kind: SIGN_IN_EVENT.into(),
                        error: false,
                    }],
                );
                let (ballot_id, content, vote_hash) = cast_ballot();
                event.ballots.push(StoredBallot {
                    voter_id: id.clone(),
                    ballot_id: ballot_id.clone(),
                    receipt_ballot_id: None,
                    content,
                    cast_at: since() + Duration::minutes(2),
                });
                event.logged.insert(
                    id,
                    vec![LoggedCast {
                        ballot_id,
                        vote_hash,
                    }],
                );
            }
            event
        }

        /// The same voters in a pseudonymous election event.
        pub fn voted_under_pseudonyms(usernames: &[&str]) -> Self {
            let mut event = Self::voted(usernames);
            event.pseudonymous = true;
            event.policy = PrivacyPolicy {
                ballot_identity: Some(vote_privacy::BallotIdentity::Pseudonym),
                network_data_policy: Some(vote_privacy::NetworkDataPolicy::DoNotStore),
            };
            event.ballot_box = BallotBoxPrivacy {
                ballots: usernames.len() as i64,
                received: usernames.len() as i64,
                ..Default::default()
            };
            event.logged.clear();
            for (index, ballot) in event.ballots.iter_mut().enumerate() {
                let code = format!("ABCD-{index:04}");
                event.private_casts.insert(code.clone());
                ballot.receipt_ballot_id = Some(code);
            }
            event
        }

        pub fn ballot_ids(&self) -> Vec<String> {
            self.ballots
                .iter()
                .map(|b| match &b.receipt_ballot_id {
                    Some(code) if self.pseudonymous => code.clone(),
                    _ => b.ballot_id.clone(),
                })
                .collect()
        }

        fn pseudonym_only(&self) -> Result<()> {
            if !self.pseudonymous {
                anyhow::bail!("This event does not use pseudonyms");
            }
            Ok(())
        }

        /// Facts for a run of the privacy checks as well.
        pub fn privacy_facts(&self, usernames: &[&str], presented: Vec<String>) -> Facts {
            let mut facts = self.facts(usernames, presented);
            facts.privacy = Some(PrivacyFacts::gather(self, "event", facts.identity).unwrap());
            facts
        }

        fn voter_id_only(&self) -> Result<()> {
            if self.pseudonymous {
                anyhow::bail!("Identity-based acceptance is unsupported for pseudonym events");
            }
            Ok(())
        }

        pub fn facts(&self, usernames: &[&str], presented: Vec<String>) -> Facts {
            let usernames: Vec<String> = usernames.iter().map(|u| (*u).into()).collect();
            Facts::gather(self, "event", &usernames, presented, since()).unwrap()
        }
    }

    impl VotingRecords for Event {
        fn voter(&self, _: &str, username: &str) -> Result<Option<Voter>> {
            Ok(self.voters.iter().find(|v| v.username == username).cloned())
        }
        fn user_events(&self, _: &str, voter_id: &str) -> Result<Vec<UserEvent>> {
            Ok(self.events.get(voter_id).cloned().unwrap_or_default())
        }
        fn published_areas(&self, _: &str, areas: &[String]) -> Result<BTreeSet<String>> {
            Ok(areas
                .iter()
                .filter(|area| self.published.contains(*area))
                .cloned()
                .collect())
        }
        fn ballot_identity(&self, _: &str) -> Result<BallotIdentity> {
            Ok(if self.pseudonymous {
                BallotIdentity::Pseudonym
            } else {
                BallotIdentity::VoterId
            })
        }
        fn stored_ballots(&self, _: &str, voter_ids: &[String]) -> Result<Vec<StoredBallot>> {
            self.voter_id_only()?;
            Ok(self
                .ballots
                .iter()
                .filter(|ballot| voter_ids.contains(&ballot.voter_id))
                .cloned()
                .collect())
        }
        fn logged_casts(&self, _: &str, voter_id: &str) -> Result<Vec<LoggedCast>> {
            self.voter_id_only()?;
            Ok(self.logged.get(voter_id).cloned().unwrap_or_default())
        }
        fn participants(&self, _: &str, voter_ids: &[String]) -> Result<BTreeSet<String>> {
            Ok(self
                .ballots
                .iter()
                .map(|ballot| ballot.voter_id.clone())
                .filter(|voter| voter_ids.contains(voter))
                .collect())
        }
        fn cast_ballots(&self, _: &str, ballot_ids: &[String]) -> Result<Vec<CastBallot>> {
            Ok(self
                .ballots
                .iter()
                .filter_map(|ballot| {
                    let code = ballot.receipt_ballot_id.clone()?;
                    ballot_ids.contains(&code).then(|| CastBallot {
                        ballot_id: code,
                        ballot_hash: ballot.ballot_id.clone(),
                        content: ballot.content.clone(),
                        cast_at: ballot.cast_at,
                    })
                })
                .collect())
        }
        fn private_casts(&self, _: &str, _: DateTime<Utc>) -> Result<BTreeSet<String>> {
            Ok(self.private_casts.clone())
        }
        fn privacy_policy(&self, _: &str) -> Result<PrivacyPolicy> {
            Ok(self.policy.clone())
        }
        fn ballot_box_privacy(&self, _: &str) -> Result<BallotBoxPrivacy> {
            self.pseudonym_only()?;
            Ok(self.ballot_box)
        }
        fn roles(&self) -> Result<Vec<Role>> {
            self.roles.clone().map_err(anyhow::Error::msg)
        }
    }

    const ALL: [Probe; 5] = [
        Probe::VoterAuthenticated,
        Probe::BallotPublished,
        Probe::BallotCast,
        Probe::ReceiptProduced,
        Probe::BallotStored,
    ];

    fn failures(findings: &[Finding]) -> Vec<String> {
        findings
            .iter()
            .filter(|finding| finding.outcome == Outcome::Fail)
            .map(|finding| format!("{} {}: {}", finding.subject, finding.name, finding.detail))
            .collect()
    }

    fn failed(which: Probe, facts: &Facts) -> Vec<String> {
        failures(&probe(which, facts).unwrap())
    }

    #[test]
    fn voters_who_signed_in_and_voted_pass_every_check() {
        let event = Event::voted(&["ana", "ben"]);
        let facts = event.facts(&["ana", "ben"], event.ballot_ids());
        for which in ALL {
            let findings = probe(which, &facts).unwrap();
            assert!(!findings.is_empty());
            assert_eq!(failures(&findings), Vec::<String>::new(), "{which}");
        }
        assert_eq!(probe(Probe::BallotPublished, &facts).unwrap().len(), 1);
    }

    #[test]
    fn no_finding_holds_a_voter_and_a_ballot_id_together() {
        let event = Event::voted(&["ana", "ben"]);
        let facts = event.facts(&["ana", "ben"], event.ballot_ids());
        for which in ALL {
            for finding in probe(which, &facts).unwrap() {
                let text = format!("{} {}", finding.name, finding.detail);
                let voter = ["ana", "ben", "id-ana", "id-ben"]
                    .iter()
                    .any(|name| text.split_whitespace().any(|word| word == *name));
                let ballot = event.ballot_ids().iter().any(|id| text.contains(id));
                assert!(!(voter && ballot), "{which}: {text}");
                match finding.subject {
                    Subject::Voter => assert!(!ballot, "{which}: {text}"),
                    Subject::Ballot => assert!(!voter, "{which}: {text}"),
                    _ => assert!(!voter && !ballot, "{which}: {text}"),
                }
            }
        }
    }

    #[test]
    fn a_voter_who_never_signed_in_fails_authentication() {
        let mut event = Event::voted(&["ana", "ben"]);
        event.events.insert(
            "id-ben".into(),
            vec![
                UserEvent {
                    at: since() - Duration::minutes(5),
                    kind: SIGN_IN_EVENT.into(),
                    error: false,
                },
                UserEvent {
                    at: since() + Duration::minutes(1),
                    kind: "LOGIN_ERROR".into(),
                    error: true,
                },
                UserEvent {
                    at: since() + Duration::minutes(1),
                    kind: "CODE_TO_TOKEN".into(),
                    error: false,
                },
            ],
        );
        let facts = event.facts(&["ana", "ben"], vec![]);
        assert_eq!(
            failed(Probe::VoterAuthenticated, &facts),
            ["voter ben: no successful sign-in since 2028-01-10T09:00:00+00:00; failed authentication events: 1"]
        );
    }

    #[test]
    fn unknown_and_disabled_voters_fail() {
        let mut event = Event::voted(&["ana", "ben"]);
        event.voters[1].enabled = false;
        let facts = event.facts(&["ana", "ben", "carl"], vec![]);
        assert_eq!(
            failed(Probe::VoterAuthenticated, &facts),
            [
                "voter ben: is disabled",
                "voter carl: is not a voter of this election event"
            ]
        );
        assert_eq!(
            failed(Probe::BallotCast, &facts),
            ["voter carl: is not a voter of this election event"]
        );
        assert_eq!(
            failed(Probe::BallotPublished, &facts),
            ["voter carl: is not a voter of this election event"]
        );
    }

    #[test]
    fn an_area_without_a_published_ballot_style_fails() {
        let mut event = Event::voted(&["ana", "ben"]);
        event.voters[1].area_id = Some("area-2".into());
        event.voters[1].area_name = None;
        let facts = event.facts(&["ana", "ben"], vec![]);
        assert_eq!(
            failed(Probe::BallotPublished, &facts),
            ["area area-2: has no published ballot style"]
        );

        event.voters[1].area_id = None;
        let facts = event.facts(&["ana", "ben"], vec![]);
        assert_eq!(
            failed(Probe::BallotPublished, &facts),
            ["voter ben: has no area"]
        );
    }

    #[test]
    fn a_voter_without_a_cast_vote_in_this_run_fails() {
        let mut event = Event::voted(&["ana", "ben"]);
        event.ballots[1].cast_at = since() - Duration::seconds(1);
        let facts = event.facts(&["ana", "ben"], vec![]);
        assert_eq!(
            failed(Probe::BallotCast, &facts),
            ["voter ben: no valid cast vote since 2028-01-10T09:00:00+00:00"]
        );
    }

    #[test]
    fn receipts_need_every_presented_id_stored_and_every_ballot_presented() {
        let event = Event::voted(&["ana", "ben"]);
        let ids = event.ballot_ids();

        let facts = event.facts(&["ana", "ben"], vec![]);
        assert!(probe(Probe::ReceiptProduced, &facts).is_err());

        let facts = event.facts(&["ana", "ben"], vec![ids[0].to_uppercase()]);
        assert_eq!(
            failed(Probe::ReceiptProduced, &facts),
            ["stage checked voters: cast ballots without a presented Ballot ID: 1 of 2"]
        );

        let facts = event.facts(
            &["ana", "ben"],
            vec![ids[0].clone(), ids[1].clone(), "f00".into()],
        );
        assert_eq!(
            failed(Probe::ReceiptProduced, &facts),
            ["ballot f00: is not a stored ballot of the checked voters"]
        );

        let facts = event.facts(&["ana"], ids.clone());
        assert_eq!(
            failed(Probe::ReceiptProduced, &facts),
            [format!(
                "ballot {}: is not a stored ballot of the checked voters",
                ids[1]
            )]
        );
    }

    #[test]
    fn a_ballot_cast_with_a_signed_receipt_is_presented_by_the_receipts_ballot_id() {
        let mut event = Event::voted(&["ana"]);
        event.ballots[0].receipt_ballot_id = Some("FTBE-MHRX".into());

        let facts = event.facts(&["ana"], vec!["ftbe-mhrx".into()]);
        assert!(failed(Probe::ReceiptProduced, &facts).is_empty());
        let facts = event.facts(&["ana"], event.ballot_ids());
        assert!(failed(Probe::ReceiptProduced, &facts).is_empty());
    }

    #[test]
    fn a_stored_ballot_must_match_the_log() {
        let event = Event::voted(&["ana", "ben"]);
        let ids = event.ballot_ids();

        let mut changed = event.clone();
        changed.ballots[0].content = changed.ballots[1].content.clone();
        assert_eq!(
            failed(Probe::BallotStored, &changed.facts(&["ana", "ben"], vec![])),
            [format!(
                "ballot {}: the stored content does not match the hash in the election event's log",
                ids[0]
            )]
        );

        let mut unlogged = event.clone();
        unlogged.logged.remove("id-ben");
        assert_eq!(
            failed(
                Probe::BallotStored,
                &unlogged.facts(&["ana", "ben"], vec![])
            ),
            [format!(
                "ballot {}: the election event's log has no cast vote entry for this Ballot ID",
                ids[1]
            )]
        );

        let mut unreadable = event.clone();
        unreadable.ballots[0].content = "{}".into();
        assert_eq!(
            failed(
                Probe::BallotStored,
                &unreadable.facts(&["ana", "ben"], vec![])
            ),
            [format!(
                "ballot {}: the stored content cannot be read",
                ids[0]
            )]
        );

        let mut empty = event;
        empty.ballots.clear();
        assert_eq!(
            failed(Probe::BallotStored, &empty.facts(&["ana", "ben"], vec![])),
            ["stage checked voters: the checked voters have no cast ballots"]
        );
    }

    #[test]
    fn checks_are_not_decided_without_voters() {
        let event = Event::voted(&["ana"]);
        let facts = event.facts(&[], event.ballot_ids());
        for which in ALL {
            assert!(probe(which, &facts).is_err());
        }
    }

    #[test]
    fn pseudonymous_voters_who_signed_in_and_voted_pass_every_check() {
        let event = Event::voted_under_pseudonyms(&["ana", "ben"]);
        let facts = event.facts(&["ana", "ben"], event.ballot_ids());
        assert_eq!(facts.identity, BallotIdentity::Pseudonym);
        assert!(facts.voters.iter().all(|voter| voter.ballots.is_empty()));
        for which in ALL {
            let findings = probe(which, &facts).unwrap();
            assert!(!findings.is_empty());
            assert_eq!(failures(&findings), Vec::<String>::new(), "{which}");
        }
        assert_eq!(
            probe(Probe::ReceiptProduced, &facts)
                .unwrap()
                .last()
                .unwrap()
                .detail,
            "presented Ballot IDs of cast ballots: 2; checked voters who voted: 2"
        );
    }

    #[test]
    fn pseudonymous_findings_never_hold_a_voter_and_a_ballot_together() {
        let event = Event::voted_under_pseudonyms(&["ana", "ben"]);
        let facts = event.facts(&["ana", "ben"], event.ballot_ids());
        let hashes: Vec<String> = event.ballots.iter().map(|b| b.ballot_id.clone()).collect();
        for which in ALL {
            for finding in probe(which, &facts).unwrap() {
                let text = format!("{} {}", finding.name, finding.detail);
                let voter = ["ana", "ben", "id-ana", "id-ben"]
                    .iter()
                    .any(|name| text.split_whitespace().any(|word| word == *name));
                let ballot = event
                    .ballot_ids()
                    .iter()
                    .chain(&hashes)
                    .any(|id| text.contains(id.as_str()));
                assert!(!(voter && ballot), "{which}: {text}");
            }
        }
    }

    #[test]
    fn a_pseudonymous_voter_without_participation_fails_ballot_cast() {
        let mut event = Event::voted_under_pseudonyms(&["ana", "ben"]);
        event.ballots.pop();
        let facts = event.facts(&["ana", "ben", "carl"], vec![]);
        assert_eq!(
            failed(Probe::BallotCast, &facts),
            [
                "voter ben: has not voted in this election event",
                "voter carl: is not a voter of this election event"
            ]
        );
    }

    #[test]
    fn pseudonymous_receipts_compare_presented_ballot_ids_with_voters_who_voted() {
        let event = Event::voted_under_pseudonyms(&["ana", "ben"]);
        let ids = event.ballot_ids();

        let facts = event.facts(&["ana", "ben"], vec![]);
        assert!(probe(Probe::ReceiptProduced, &facts).is_err());
        assert!(probe(Probe::BallotStored, &facts).is_err());

        let facts = event.facts(
            &["ana", "ben"],
            vec![ids[0].to_lowercase().replace('-', "")],
        );
        assert_eq!(
            failed(Probe::ReceiptProduced, &facts),
            ["stage checked voters: presented Ballot IDs of cast ballots: 1, fewer than the checked voters who voted: 2"]
        );

        let facts = event.facts(
            &["ana", "ben"],
            vec![
                ids[0].clone(),
                ids[1].clone(),
                ids[1].clone(),
                "ZZZZ-ZZZZ".into(),
            ],
        );
        assert_eq!(
            failed(Probe::ReceiptProduced, &facts),
            ["ballot ZZZZ-ZZZZ: is not a ballot cast in this election event since 2028-01-10T09:00:00+00:00"]
        );

        let mut early = event.clone();
        early.ballots[1].cast_at = since() - Duration::seconds(1);
        assert_eq!(
            failed(Probe::ReceiptProduced, &early.facts(&["ana", "ben"], ids.clone())),
            [
                format!(
                    "ballot {}: is not a ballot cast in this election event since 2028-01-10T09:00:00+00:00",
                    ids[1]
                ),
                "stage checked voters: presented Ballot IDs of cast ballots: 1, fewer than the checked voters who voted: 2".into()
            ]
        );

        let mut none = event;
        none.ballots.clear();
        assert_eq!(
            failed(
                Probe::ReceiptProduced,
                &none.facts(&["ana", "ben"], ids.clone())
            )
            .last()
            .unwrap(),
            "stage checked voters: the checked voters have no cast ballots"
        );
    }

    #[test]
    fn a_pseudonymous_stored_ballot_must_match_its_hash_and_the_log() {
        let event = Event::voted_under_pseudonyms(&["ana", "ben"]);
        let ids = event.ballot_ids();

        let mut changed = event.clone();
        changed.ballots[0].content = changed.ballots[1].content.clone();
        assert_eq!(
            failed(
                Probe::BallotStored,
                &changed.facts(&["ana", "ben"], ids.clone())
            ),
            [format!(
                "ballot {}: the stored content does not match the ballot hash it is stored under",
                ids[0]
            )]
        );

        let mut unlogged = event.clone();
        unlogged.private_casts.remove(&ids[1]);
        assert_eq!(
            failed(
                Probe::BallotStored,
                &unlogged.facts(&["ana", "ben"], ids.clone())
            ),
            [format!(
                "ballot {}: the election event's log has no cast vote entry for this Ballot ID",
                ids[1]
            )]
        );

        let mut unreadable = event.clone();
        unreadable.ballots[0].content = "{}".into();
        assert_eq!(
            failed(
                Probe::BallotStored,
                &unreadable.facts(&["ana", "ben"], ids.clone())
            ),
            [format!(
                "ballot {}: the stored content cannot be read",
                ids[0]
            )]
        );

        assert_eq!(
            failed(
                Probe::BallotStored,
                &event.facts(&["ana", "ben"], vec!["ZZZZ-ZZZZ".into()])
            ),
            ["stage checked voters: no presented Ballot ID names a cast ballot"]
        );
    }

    const PRIVACY: [Probe; 4] = [
        Probe::PseudonymousBallots,
        Probe::NetworkDataNotStored,
        Probe::BallotBoxUnlinked,
        Probe::PrivilegedRoles,
    ];

    #[test]
    fn a_pseudonymous_event_passes_the_privacy_checks_without_voters() {
        let event = Event::voted_under_pseudonyms(&["ana", "ben"]);
        let facts = event.privacy_facts(&[], vec![]);
        for which in PRIVACY {
            let findings = probe(which, &facts).unwrap();
            assert!(!findings.is_empty());
            assert_eq!(failures(&findings), Vec::<String>::new(), "{which}");
            for finding in &findings {
                assert_eq!(finding.subject, Subject::Stage, "{which}");
            }
        }
        assert_eq!(
            probe(Probe::BallotCast, &facts).unwrap_err(),
            "no voters were given"
        );
        let unread = event.facts(&[], vec![]);
        assert!(probe(Probe::PseudonymousBallots, &unread).is_err());
    }

    #[test]
    fn privileged_roles_are_listed_by_permission() {
        let event = Event::voted_under_pseudonyms(&["ana"]);
        let facts = event.privacy_facts(&[], vec![]);
        let details: Vec<String> = probe(Probe::PrivilegedRoles, &facts)
            .unwrap()
            .iter()
            .map(|finding| format!("{}: {}", finding.name, finding.detail))
            .collect();
        assert_eq!(
            details,
            [
                "voter-pseudonym-resolve: held by roles: admin",
                "forensic-read: held by roles: admin, auditor"
            ]
        );

        let mut nobody = event.clone();
        nobody.roles = Ok(vec![role("support", &[])]);
        let findings = probe(Probe::PrivilegedRoles, &nobody.privacy_facts(&[], vec![])).unwrap();
        assert!(findings.iter().all(|f| f.detail == "no role holds it"));

        let mut unreadable = event.clone();
        unreadable.roles = Err("not allowed".into());
        let reason = probe(
            Probe::PrivilegedRoles,
            &unreadable.privacy_facts(&[], vec![]),
        )
        .unwrap_err();
        assert!(reason.contains("not allowed"), "{reason}");
    }

    #[test]
    fn each_privacy_breach_fails_its_check() {
        type Break = fn(&mut Event);
        let cases: [(Probe, Break, &str); 7] = [
            (
                Probe::PseudonymousBallots,
                |event| event.policy.ballot_identity = None,
                "stage election event: the ballot box stores ballots under pseudonyms but presentation.ballot_identity is not set (voter-id)",
            ),
            (
                Probe::NetworkDataNotStored,
                |event| {
                    event.policy.network_data_policy =
                        Some(vote_privacy::NetworkDataPolicy::ForensicStore)
                },
                "stage election event: presentation.network_data_policy is forensic-store: the IP address and country of each cast are kept in the forensic tables with its pseudonym and Ballot ID",
            ),
            (
                Probe::BallotBoxUnlinked,
                |event| event.ballot_box.ballots_with_network_data = 1,
                "stage ballot box: ballots with an IP address or country: 1 of 2",
            ),
            (
                Probe::BallotBoxUnlinked,
                |event| event.ballot_box.ballots_with_username = 2,
                "stage ballot box: ballots with a username other than their pseudonym: 2 of 2",
            ),
            (
                Probe::BallotBoxUnlinked,
                |event| event.ballot_box.ballots_without_pseudonym = 1,
                "stage ballot box: ballots whose voter id is not an allocated pseudonym: 1 of 2",
            ),
            (
                Probe::BallotBoxUnlinked,
                |event| event.ballot_box.received_without_pseudonym = 1,
                "stage ballot box: received ballots whose voter id is not an allocated pseudonym: 1 of 2",
            ),
            (
                Probe::NetworkDataNotStored,
                |event| event.policy.network_data_policy = None,
                "",
            ),
        ];
        for (which, breakage, expected) in cases {
            let mut event = Event::voted_under_pseudonyms(&["ana", "ben"]);
            breakage(&mut event);
            let facts = event.privacy_facts(&[], vec![]);
            let expected: Vec<&str> = [expected].into_iter().filter(|e| !e.is_empty()).collect();
            assert_eq!(failed(which, &facts), expected, "{which}");
            for other in PRIVACY.iter().filter(|other| **other != which) {
                assert_eq!(failed(*other, &facts), Vec::<String>::new(), "{other}");
            }
        }
    }

    #[test]
    fn a_voter_id_event_fails_the_privacy_checks_cleanly() {
        let event = Event::voted(&["ana", "ben"]);
        let facts = event.privacy_facts(&["ana", "ben"], event.ballot_ids());
        assert!(facts.privacy.as_ref().unwrap().ballot_box.is_none());
        assert_eq!(
            failed(Probe::PseudonymousBallots, &facts),
            ["stage election event: the ballot box stores ballots under voter ids (ballot identity voter-id), so each ballot names its voter; presentation.ballot_identity is not set (voter-id)"]
        );
        assert_eq!(
            failed(Probe::NetworkDataNotStored, &facts),
            ["stage election event: the ballot box stores ballots under voter ids (ballot identity voter-id), so each ballot names its voter, with its IP address and country"]
        );
        assert_eq!(
            failed(Probe::BallotBoxUnlinked, &facts),
            ["stage ballot box: the ballot box stores ballots under voter ids (ballot identity voter-id), so each ballot names its voter"]
        );
        assert_eq!(failed(Probe::PrivilegedRoles, &facts), Vec::<String>::new());
    }
}

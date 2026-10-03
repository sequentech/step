// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Automatic checks of the voting stage, decided from the records the
//! platform already keeps of a live election event.
//!
//! Findings about a voter never carry a Ballot ID and findings about a ballot
//! never name its voter.
use super::{
    definition::Probe,
    ledger::{Finding, Subject},
};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sequent_core::{
    ballot::{HashableBallot, SignedHashableBallot},
    encrypt::{hash_ballot_sha512, hash_multi_ballot_sha512},
    multi_ballot::{HashableMultiBallot, SignedHashableMultiBallot},
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

/// What the checks read. The Hasura implementation is in `super::hasura`.
pub trait VotingRecords {
    fn voter(&self, event: &str, username: &str) -> Result<Option<Voter>>;
    fn user_events(&self, event: &str, voter_id: &str) -> Result<Vec<UserEvent>>;
    fn published_areas(&self, event: &str, areas: &[String]) -> Result<BTreeSet<String>>;
    fn stored_ballots(&self, event: &str, voter_ids: &[String]) -> Result<Vec<StoredBallot>>;
    fn logged_casts(&self, event: &str, voter_id: &str) -> Result<Vec<LoggedCast>>;
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
}

/// Everything the probes need, read once so all checks judge the same state.
#[derive(Debug)]
pub struct Facts {
    pub since: DateTime<Utc>,
    pub voters: Vec<VoterFacts>,
    pub published_areas: BTreeSet<String>,
    /// Ballot IDs read from the voters' confirmation screens or receipts.
    pub presented_ballot_ids: Vec<String>,
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
        let mut voters = Vec::new();
        for username in usernames {
            let voter = records.voter(event, username)?;
            let (events, logged) = match &voter {
                Some(voter) => (
                    records.user_events(event, &voter.id)?,
                    records.logged_casts(event, &voter.id)?,
                ),
                None => Default::default(),
            };
            voters.push(VoterFacts {
                username: username.clone(),
                voter,
                events: events.into_iter().filter(|e| e.at >= since).collect(),
                ballots: vec![],
                logged,
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
        if !ids.is_empty() {
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
        Ok(Self {
            since,
            voters,
            published_areas,
            presented_ballot_ids,
        })
    }

    fn ballots(&self) -> impl Iterator<Item = (&VoterFacts, &StoredBallot)> {
        self.voters
            .iter()
            .flat_map(|facts| facts.ballots.iter().map(move |ballot| (facts, ballot)))
    }
}

/// The findings of one check, or why it cannot be decided with what the operator gave.
pub fn probe(probe: Probe, facts: &Facts) -> Result<Vec<Finding>, String> {
    if facts.voters.is_empty() {
        return Err("no voters were given".into());
    }
    match probe {
        Probe::VoterAuthenticated => Ok(voter_authenticated(facts)),
        Probe::BallotPublished => Ok(ballot_published(facts)),
        Probe::BallotCast => Ok(ballot_cast(facts)),
        Probe::ReceiptProduced => receipt_produced(facts),
        Probe::BallotStored => Ok(ballot_stored(facts)),
    }
}

const NOT_A_VOTER: &str = "is not a voter of this election event";

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
        return Err("no Ballot IDs from confirmation screens or receipts were given".into());
    }
    let mut findings: Vec<Finding> = presented
        .iter()
        .map(|id| {
            if facts
                .ballots()
                .any(|(_, ballot)| same_id(&ballot.ballot_id, id))
            {
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
        .filter(|(_, ballot)| !presented.iter().any(|id| same_id(&ballot.ballot_id, id)))
        .count();
    findings.push(if stored == 0 {
        Finding::fail(
            Subject::Stage,
            CHECKED_VOTERS,
            "the checked voters have no cast ballots",
        )
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
            "the checked voters have no cast ballots",
        ));
    }
    findings
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
    use strand::backend::ristretto::RistrettoCtx;

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
    #[derive(Clone, Default)]
    pub struct Event {
        pub voters: Vec<Voter>,
        pub events: BTreeMap<String, Vec<UserEvent>>,
        pub published: BTreeSet<String>,
        pub ballots: Vec<StoredBallot>,
        pub logged: BTreeMap<String, Vec<LoggedCast>>,
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

        pub fn ballot_ids(&self) -> Vec<String> {
            self.ballots.iter().map(|b| b.ballot_id.clone()).collect()
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
        fn stored_ballots(&self, _: &str, voter_ids: &[String]) -> Result<Vec<StoredBallot>> {
            Ok(self
                .ballots
                .iter()
                .filter(|ballot| voter_ids.contains(&ballot.voter_id))
                .cloned()
                .collect())
        }
        fn logged_casts(&self, _: &str, voter_id: &str) -> Result<Vec<LoggedCast>> {
            Ok(self.logged.get(voter_id).cloned().unwrap_or_default())
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
}

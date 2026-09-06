// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum_macros::Display;

use crate::messages::newtypes::{CertificateAuthEventAction, *};

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
pub struct Statement {
    pub head: StatementHead,
    pub body: StatementBody,
}
impl Statement {
    pub fn new(head: StatementHead, body: StatementBody) -> Statement {
        Statement { head, body }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug, Clone)]
pub struct StatementHead {
    pub event: EventIdString,
    pub kind: StatementType,
    pub timestamp: Timestamp,
    pub event_type: StatementEventType,
    pub log_type: StatementLogType,
    pub description: String,
}
impl StatementHead {
    pub fn from_body(event: EventIdString, body: &StatementBody) -> Self {
        let timestamp = crate::timestamp();
        let default_head = StatementHead {
            event,
            kind: StatementType::Unknown,
            timestamp,
            event_type: StatementEventType::SYSTEM,
            log_type: StatementLogType::INFO,
            description: "".to_string(),
        };

        match body {
            StatementBody::CastVote(_, _, _, _, _) => StatementHead {
                kind: StatementType::CastVote,
                description: "Inserted cast vote.".to_string(),
                ..default_head
            },
            StatementBody::CastVoteWithChannel(_, _, _, _, _, channel) => StatementHead {
                kind: StatementType::CastVote,
                description: format!(
                    "Inserted cast vote. Voting channel: {channel}.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::CastVoteError(_, _, _, _, _) => StatementHead {
                kind: StatementType::CastVoteError,
                log_type: StatementLogType::ERROR,
                description: "Error inserting cast vote.".to_string(),
                ..default_head
            },
            StatementBody::ExternalApiRequest(_, _, direction, _, operation) => {
                // Keep the description short. The signed body retains the
                // cast-vote identifier, bounded outcome classification, and
                // template hash used for reconciliation.
                let outcome = operation
                    .rsplit_once("; ")
                    .map(|(_, outcome)| outcome)
                    .unwrap_or(operation)
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .split(" (")
                    .next()
                    .unwrap_or_default()
                    .trim();
                StatementHead {
                    kind: StatementType::ExternalApiRequest,
                    description: format!("{direction} request {outcome}."),
                    ..default_head
                }
            }
            StatementBody::ElectionPublish(_, _) => StatementHead {
                kind: StatementType::ElectionPublish,
                description: "Election published.".to_string(),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodOpen(_, channel) => StatementHead {
                kind: StatementType::ElectionVotingPeriodOpen,
                description: format!(
                    "Election voting period opened for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodPause(_, channel) => StatementHead {
                kind: StatementType::ElectionVotingPeriodPause,
                description: format!(
                    "Election voting period paused for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionVotingPeriodClose(_, channel) => StatementHead {
                kind: StatementType::ElectionVotingPeriodClose,
                description: format!(
                    "Election voting period closed for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodOpen(_, _, channel) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodOpen,
                description: format!(
                    "Election-event voting period opened for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodPause(_, channel) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodPause,
                description: format!(
                    "Election-event voting period paused for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::ElectionEventVotingPeriodClose(_, _, channel) => StatementHead {
                kind: StatementType::ElectionEventVotingPeriodClose,
                description: format!(
                    "Election-event voting period closed for {channel} channel.",
                    channel = channel.0
                ),
                ..default_head
            },
            StatementBody::KeyGeneration => StatementHead {
                kind: StatementType::KeyGeneration,
                description: "Creating keys ceremony.".to_string(),
                ..default_head
            },
            StatementBody::KeyInsertionStart => StatementHead {
                kind: StatementType::KeyInsertionStart,
                description: "Tally ceremony created.".to_string(),
                ..default_head
            },
            StatementBody::KeyInsertionCeremony(_) => StatementHead {
                kind: StatementType::KeyInsertionCeremony,
                description: "Trustees key restored.".to_string(),
                ..default_head
            },
            StatementBody::TallyOpen(_) => StatementHead {
                kind: StatementType::TallyOpen,
                description: "Tally session openned.".to_string(),
                ..default_head
            },
            StatementBody::TallyClose(_) => StatementHead {
                kind: StatementType::TallyClose,
                description: "Tally closed, session completed.".to_string(),
                ..default_head
            },
            StatementBody::TallyResumedWithResolution(_, _) => StatementHead {
                kind: StatementType::TallyResumedWithResolution,
                description: "Tally resumed after tie-break resolution.".to_string(),
                ..default_head
            },
            StatementBody::TallyPausedPendingResolution(_, _) => StatementHead {
                kind: StatementType::TallyPausedPendingResolution,
                description: "Tally paused pending tie-break resolution.".to_string(),
                ..default_head
            },
            StatementBody::TallyTieResolved(_, contest, _) => StatementHead {
                kind: StatementType::TallyTieResolved,
                event_type: StatementEventType::USER,
                description: format!("Tie-break resolved for contest {}.", contest.0),
                ..default_head
            },
            StatementBody::TallyTieResolutionUpdated(_, contest, _) => StatementHead {
                kind: StatementType::TallyTieResolutionUpdated,
                event_type: StatementEventType::USER,
                description: format!("Tie-break resolution updated for contest {}.", contest.0),
                ..default_head
            },
            StatementBody::SendTemplate => StatementHead {
                kind: StatementType::SendTemplate,
                description: "Template sent to user.".to_string(),
                ..default_head
            },
            StatementBody::SendCommunications(_) => StatementHead {
                kind: StatementType::SendCommunications,
                description: "Communication sent to user.".to_string(),
                ..default_head
            },
            StatementBody::KeycloakUserEvent(error_message_string, error_message_type) => {
                let mut description = error_message_type.0.to_string();
                let log_type = if error_message_type.0.contains("ERROR") {
                    // Leave the first word in error_message_string which should be the error code.
                    description = format!(
                        "{} {}",
                        description,
                        error_message_string
                            .0
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                    );
                    StatementLogType::ERROR
                } else {
                    // Remove ":" char from description if exists
                    description = description.replace(":", "");
                    StatementLogType::INFO
                };

                StatementHead {
                    kind: StatementType::KeycloakUserEvent,
                    event_type: StatementEventType::USER,
                    description,
                    log_type,
                    ..default_head
                }
            }
            StatementBody::VoterPublicKey(_, _, _, _) => StatementHead {
                kind: StatementType::VoterPublicKey,
                event_type: StatementEventType::USER,
                description: "Voter has public key.".to_string(),
                ..default_head
            },
            StatementBody::AdminPublicKey(_, _, _) => StatementHead {
                kind: StatementType::AdminPublicKey,
                description: "Admin has public key.".to_string(),
                ..default_head
            },
            StatementBody::CertificateAuthEvent(action, subjects) => {
                let action_str = match action {
                    CertificateAuthEventAction::Import => "imported",
                    CertificateAuthEventAction::Delete => "deleted",
                };
                let subjects_str = subjects.0.join("; ");
                let description = if subjects.0.len() == 1 {
                    format!("CA certificate {action_str}. Subject: {subjects_str}")
                } else {
                    format!("CA certificates {action_str}. Subjects: {subjects_str}")
                };
                StatementHead {
                    kind: StatementType::CertificateAuthEvent,
                    event_type: StatementEventType::USER,
                    description,
                    ..default_head
                }
            }
            StatementBody::PhoneBlacklistUpdated(number, action) => {
                let action_msg = match action {
                    PhoneBlacklistAction::CreateEntry => "added to",
                    PhoneBlacklistAction::DeleteEntry => "deleted from",
                };
                let description = format!("Phone {} {action_msg} the phone blacklist", number.0);
                StatementHead {
                    kind: StatementType::PhoneBlacklistUpdated,
                    description,
                    ..default_head
                }
            }
            StatementBody::ExternalReconciliation(
                _,
                kind,
                sequence,
                _,
                input_hash,
                output_hash,
            ) => {
                let action = match kind {
                    ExternalReconciliationKind::PatchGenerated => "External patch generated",
                    ExternalReconciliationKind::ChangesApplied => "Sequent-side changes applied",
                };
                StatementHead {
                    kind: StatementType::ExternalReconciliation,
                    event_type: StatementEventType::USER,
                    description: format!(
                        "{action} for reconciliation Sequence {} (input {}, output {}).",
                        sequence.0,
                        input_hash.0,
                        output_hash.0.as_deref().unwrap_or("none"),
                    ),
                    ..default_head
                }
            }
            StatementBody::ResultsPublicationAction(details) => {
                let action = match details.action {
                    ResultsPublicationAction::Publish => "published",
                    ResultsPublicationAction::Revoke => "revoked",
                };
                let route_election = details.route_election_id.0.as_deref().unwrap_or("event");
                StatementHead {
                    kind: StatementType::ResultsPublicationAction,
                    event_type: StatementEventType::USER,
                    description: format!(
                        "Results publication {} {action} for {} route ({route_election}), with {} access, {} visibility, and {} contests.",
                        details.publication_id.0,
                        details.route_scope.0,
                        details.access.0,
                        details.visibility_scope.0,
                        details.contest_ids.len(),
                    ),
                    ..default_head
                }
            }
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
pub enum StatementBody {
    // NOT IMPLEMENTED YET, but please feel free
    // "Emisión de voto (sólo como registro que el sistema almacenó correctamente el voto)
    CastVote(
        ElectionIdString,
        PseudonymHash,
        CastVoteHash,
        VoterIpString,
        VoterCountryString,
    ),
    // NOT IMPLEMENTED YET, but please feel free
    // "Errores en la emisión del voto."
    CastVoteError(
        ElectionIdString,
        PseudonymHash,
        CastVoteErrorString,
        VoterIpString,
        VoterCountryString,
    ),
    // /workspaces/step/packages/harvest/src/main.rs
    //    routes::ballot_publication::publish_ballot
    //
    // "Publicación, apertura y cierre de las elecciones"
    ElectionPublish(ElectionIdString, BallotPublicationIdString),
    // /workspaces/step/packages/harvest/src/main.rs
    //    routes::voting_status::update_event_status,
    //    routes::voting_status::update_election_status,
    //
    // "Publicación, apertura y cierre de las elecciones"
    ElectionVotingPeriodOpen(ElectionIdString, VotingChannelString),
    ElectionVotingPeriodPause(ElectionIdString, VotingChannelString),
    ElectionVotingPeriodClose(ElectionIdString, VotingChannelString),
    ElectionEventVotingPeriodOpen(EventIdString, ElectionsIdsString, VotingChannelString),
    ElectionEventVotingPeriodPause(EventIdString, VotingChannelString),
    ElectionEventVotingPeriodClose(EventIdString, ElectionsIdsString, VotingChannelString),
    // /workspaces/step/packages/windmill/src/celery_app.rs
    // create_keys
    //
    // "Creación de llave criptográfica"
    KeyGeneration,
    // /workspaces/step/packages/harvest/src/main.rs
    // routes::tally_ceremony::restore_private_key
    //
    // "Ingreso de los fragmentos de la llave privada"
    KeyInsertionStart,
    KeyInsertionCeremony(TrusteeNameString),
    // /workspaces/step/packages/windmill/src/celery_app.rs
    // tally_election_event
    //
    // "Apertura y cierre de la bóveda de votos"
    TallyOpen(ElectionIdString),
    // /workspaces/step/packages/windmill/src/celery_app.rs
    // execute_tally_session: falta que Felix ponga SUCCESS cuando se termine, creo, hablar con felix
    //
    // "Apertura y cierre de la bóveda de votos"
    TallyClose(ElectionIdString),
    TallyResumedWithResolution(ElectionIdString, ResolutionIdsString),
    TallyPausedPendingResolution(ElectionIdString, ResolutionIdsString),
    TallyTieResolved(ElectionIdString, ContestIdString, ResolutionIdsString),
    TallyTieResolutionUpdated(ElectionIdString, ContestIdString, ResolutionIdsString),

    SendTemplate,
    SendCommunications(Option<String>),
    KeycloakUserEvent(ErrorMessageString, KeycloakEventTypeString),
    /// Represents the assertion that
    ///     within the given tenant
    ///     within the given election event
    ///     the given user pseudonym hash
    ///     has as their public key the given public key (in der_b64 format)
    VoterPublicKey(
        TenantIdString,
        EventIdString,
        PseudonymHash,
        PublicKeyDerB64,
    ),
    /// Represents the assertion that
    ///     within the given tenant
    ///     the given admin user
    ///     hash has as their public key the given public key (in der_b64 format)
    AdminPublicKey(TenantIdString, Option<String>, PublicKeyDerB64),
    /// Records that one or more CA certificates were imported or deleted for an election event.
    /// Carries the action (Import/Delete) and the subject DNs of the affected certificates.
    CertificateAuthEvent(CertificateAuthEventAction, CertificateSubjectDnsString),
    PhoneBlacklistUpdated(PhoneE164String, PhoneBlacklistAction),
    ResultsPublicationAction(ResultsPublicationDetails),
    /// Records an external request and binds its subject to the signed
    /// statement rather than relying only on searchable message metadata.
    ExternalApiRequest(
        EventIdString,
        ExternalApiSubject,
        ExtApiRequestDirection,
        ExtApiName,
        String,
    ),
    /// Cast-vote statement carrying its source channel. This separate,
    /// append-only variant keeps existing Borsh-encoded `CastVote` messages
    /// deserializable.
    ///
    /// Rollout invariant: every electoral-log reader (including released
    /// `step-cli` and external auditors) must be upgraded before writers emit
    /// this variant. Older readers cannot decode a variant they do not know.
    CastVoteWithChannel(
        ElectionIdString,
        PseudonymHash,
        CastVoteHash,
        VoterIpString,
        VoterCountryString,
        VotingChannelString,
    ),
    /// Records a third-party voter registry reconciliation run event: either
    /// the external-side diff/patch being generated, or the Sequent-side diff
    /// being applied. Named for the general capability, not the specific
    /// integration (Datafix) that first needed it. Doesn't fit
    /// `ExternalApiRequest`'s shape: there is no HTTP call to the external
    /// system involved, since it is offline for the whole freeze period a
    /// reconciliation run happens during. The JSON of every applied voter's
    /// old/new values is carried in `Message.artifact` on the
    /// `ChangesApplied` entry — there is exactly one entry per phase per run,
    /// not one per voter.
    ExternalReconciliation(
        EventIdString,
        ExternalReconciliationKind,
        ExternalReconciliationSequenceString,
        ExternalReconciliationGeneratedAtString,
        ExternalReconciliationInputHashString,
        ExternalReconciliationOutputHashString,
    ),
}

// Note: When creating new variants, consider that the length limit STATEMENT_KIND_VARCHAR_LENGTH is 40.
#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug, Clone)]
pub enum StatementType {
    Unknown,
    CastVote,
    CastVoteError,
    ElectionPublish,
    ElectionVotingPeriodOpen,
    ElectionVotingPeriodClose,
    ElectionVotingPeriodPause,
    ElectionEventVotingPeriodOpen,
    ElectionEventVotingPeriodClose,
    ElectionEventVotingPeriodPause,
    KeyGeneration,
    KeyInsertionStart,
    KeyInsertionCeremony,
    TallyOpen,
    TallyClose,
    TallyResumedWithResolution,
    TallyPausedPendingResolution,
    TallyTieResolved,
    TallyTieResolutionUpdated,
    SendTemplate,
    SendCommunications,
    KeycloakUserEvent,
    VoterPublicKey,
    AdminPublicKey,
    CertificateAuthEvent,
    PhoneBlacklistUpdated,
    ResultsPublicationAction,
    ExternalApiRequest,
    ExternalReconciliation,
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug, Clone)]
pub enum StatementEventType {
    USER,
    SYSTEM,
}

#[cfg(test)]
mod statement_compatibility_tests {
    use super::*;

    fn external_api_request_description(operation: &str) -> String {
        let event_id = EventIdString("0609dd53-3c33-41cd-b2cd-0ffb39738d2d".to_string());
        let body = StatementBody::ExternalApiRequest(
            event_id.clone(),
            ExternalApiSubject {
                user_id: Some("voter-id".to_string()),
                username: Some("voter-name".to_string()),
            },
            ExtApiRequestDirection::Outbound,
            ExtApiName::Datafix,
            operation.to_string(),
        );
        StatementHead::from_body(event_id, &body).description
    }

    #[test]
    fn external_api_request_description_success() {
        assert_eq!(
            external_api_request_description("SetVoted Succeeded"),
            "Outbound request SetVoted Succeeded."
        );
    }

    #[test]
    fn external_api_request_description_failure_without_detail() {
        assert_eq!(
            external_api_request_description("SetNotVoted Failed"),
            "Outbound request SetNotVoted Failed."
        );
    }

    #[test]
    fn external_api_request_description_excludes_detail() {
        assert_eq!(
            external_api_request_description("SetNotVoted Failed: The voter has not voted."),
            "Outbound request SetNotVoted Failed."
        );
    }

    #[test]
    fn statement_body_borsh_discriminants_are_append_only() {
        let election_publish = StatementBody::ElectionPublish(
            ElectionIdString(None),
            BallotPublicationIdString(String::new()),
        );
        let certificate = StatementBody::CertificateAuthEvent(
            CertificateAuthEventAction::Import,
            CertificateSubjectDnsString(Vec::new()),
        );
        let phone = StatementBody::PhoneBlacklistUpdated(
            PhoneE164String(String::new()),
            PhoneBlacklistAction::CreateEntry,
        );
        let external = StatementBody::ExternalApiRequest(
            EventIdString(String::new()),
            ExternalApiSubject {
                user_id: None,
                username: None,
            },
            ExtApiRequestDirection::Outbound,
            ExtApiName::Datafix,
            String::new(),
        );
        let cast_vote_with_channel = StatementBody::CastVoteWithChannel(
            ElectionIdString(None),
            PseudonymHash::new([0; 64]),
            CastVoteHash::new([0; 64]),
            VoterIpString(String::new()),
            VoterCountryString(String::new()),
            VotingChannelString(String::new()),
        );
        let external_reconciliation = StatementBody::ExternalReconciliation(
            EventIdString(String::new()),
            ExternalReconciliationKind::PatchGenerated,
            ExternalReconciliationSequenceString(String::new()),
            ExternalReconciliationGeneratedAtString(String::new()),
            ExternalReconciliationInputHashString(String::new()),
            ExternalReconciliationOutputHashString(None),
        );

        assert_eq!(borsh::to_vec(&election_publish).unwrap()[0], 2);
        assert_eq!(borsh::to_vec(&certificate).unwrap()[0], 23);
        assert_eq!(borsh::to_vec(&phone).unwrap()[0], 24);
        // `ExternalApiRequest` follows `ResultsPublicationAction` in the
        // released enum. The previous expectation of 25 was left stale by the
        // rebase that introduced `ExternalApiRequest`.
        assert_eq!(borsh::to_vec(&external).unwrap()[0], 26);
        assert_eq!(borsh::to_vec(&cast_vote_with_channel).unwrap()[0], 27);
        assert_eq!(borsh::to_vec(&external_reconciliation).unwrap()[0], 28);
    }

    #[test]
    fn legacy_cast_vote_body_remains_deserializable() {
        let legacy = StatementBody::CastVote(
            ElectionIdString(Some("election-id".to_string())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            VoterIpString("ip".to_string()),
            VoterCountryString("country".to_string()),
        );

        let bytes = borsh::to_vec(&legacy).unwrap();
        assert_eq!(bytes[0], 0);
        let decoded: StatementBody = borsh::from_slice(&bytes).unwrap();
        assert!(matches!(decoded, StatementBody::CastVote(_, _, _, _, _)));
    }

    #[test]
    fn statement_type_borsh_discriminants_are_append_only() {
        assert_eq!(
            borsh::to_vec(&StatementType::ElectionPublish).unwrap()[0],
            3
        );
        assert_eq!(
            borsh::to_vec(&StatementType::CertificateAuthEvent).unwrap()[0],
            24
        );
        assert_eq!(
            borsh::to_vec(&StatementType::PhoneBlacklistUpdated).unwrap()[0],
            25
        );
        assert_eq!(
            borsh::to_vec(&StatementType::ResultsPublicationAction).unwrap()[0],
            26
        );
        assert_eq!(
            borsh::to_vec(&StatementType::ExternalApiRequest).unwrap()[0],
            27
        );
    }

    #[test]
    fn external_api_subject_is_part_of_borsh_payload() {
        let without_subject = StatementBody::ExternalApiRequest(
            EventIdString("event".to_string()),
            ExternalApiSubject {
                user_id: None,
                username: None,
            },
            ExtApiRequestDirection::Outbound,
            ExtApiName::Datafix,
            "SetVoted Succeeded".to_string(),
        );
        let with_subject = StatementBody::ExternalApiRequest(
            EventIdString("event".to_string()),
            ExternalApiSubject {
                user_id: Some("voter-id".to_string()),
                username: Some("voter-name".to_string()),
            },
            ExtApiRequestDirection::Outbound,
            ExtApiName::Datafix,
            "SetVoted Succeeded".to_string(),
        );

        assert_ne!(
            borsh::to_vec(&without_subject).unwrap(),
            borsh::to_vec(&with_subject).unwrap()
        );
    }
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug, Clone)]
pub enum StatementLogType {
    INFO,
    ERROR,
}

#[cfg(test)]
mod results_publication_tests {
    use super::*;

    #[test]
    fn results_publication_action_is_a_structured_user_event() {
        let body = StatementBody::ResultsPublicationAction(ResultsPublicationDetails {
            publication_id: ResultsPublicationIdString("publication-id".to_string()),
            action: ResultsPublicationAction::Publish,
            route_scope: ResultsPublicationRouteScopeString("election".to_string()),
            route_election_id: ElectionIdString(Some("election-id".to_string())),
            access: ResultsPublicationAccessString("public".to_string()),
            visibility_scope: ResultsPublicationVisibilityScopeString("full_event".to_string()),
            contest_ids: vec![
                ContestIdString("contest-1".to_string()),
                ContestIdString("contest-2".to_string()),
            ],
        });

        let head = StatementHead::from_body(EventIdString("event-id".to_string()), &body);

        assert!(matches!(head.kind, StatementType::ResultsPublicationAction));
        assert!(matches!(head.event_type, StatementEventType::USER));
        assert_eq!(
            head.description,
            "Results publication publication-id published for election route (election-id), with public access, full_event visibility, and 2 contests."
        );
    }
}

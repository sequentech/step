// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Every check verdict and outcome of `evaluate`, and the save notices.

use super::*;
use chrono::TimeZone;
use sequent_core::ballot::InitializationScope;

const OPEN: SigningAction = SigningAction::OpenVoting;
const CLOSE: SigningAction = SigningAction::CloseVoting;
use UnsignedScheduledClosePolicy::{REFUSE, RUN_AS_SYSTEM};

fn rule(required: bool, signatures: u32) -> RuleSnapshot {
    RuleSnapshot {
        required,
        signatures: required.then_some(signatures),
        digest: format!("{required}-{signatures}"),
    }
}

fn copy(required: bool, policy: UnsignedScheduledClosePolicy) -> CopyValues {
    CopyValues {
        rule: rule(required, 2),
        close_policy: policy,
    }
}

fn published(required: bool, policy: UnsignedScheduledClosePolicy) -> PublishedCopy {
    PublishedCopy::Snapshot {
        publication_id: Uuid::from_u128(7),
        published_at: Utc.with_ymd_and_hms(2028, 4, 1, 8, 0, 0).unwrap(),
        values: copy(required, policy),
    }
}

fn approval() -> AuthorizedBy {
    AuthorizedBy {
        request_id: Uuid::from_u128(9).to_string(),
        code: "K7Q-2M".to_owned(),
        signers: vec!["Ana".to_owned(), "Bo".to_owned()],
    }
}

fn inputs(
    action: SigningAction,
    current: CopyValues,
    published: PublishedCopy,
    coverage: Coverage,
) -> Inputs {
    Inputs {
        action,
        current,
        published,
        coverage,
        approval_required: true,
        moment: Moment::Prediction,
    }
}

fn check<'a>(explanation: &'a Explanation, id: CheckId) -> Option<&'a Check> {
    explanation.checks.iter().find(|check| check.id == id)
}

fn key(explanation: &Explanation, id: CheckId) -> &str {
    &check(explanation, id).unwrap().current.message_key
}

/// The two configurations: the product default (Madrid association) and
/// the preset that closes at the common deadline (COMELEC).
const CONFIGURATIONS: [UnsignedScheduledClosePolicy; 2] = [REFUSE, RUN_AS_SYSTEM];

#[test]
fn without_signatures_needed_it_runs() {
    for action in [OPEN, CLOSE] {
        for policy in CONFIGURATIONS {
            let explanation = evaluate(&inputs(
                action,
                copy(false, policy.clone()),
                published(false, policy.clone()),
                Coverage::NoApproval,
            ));
            assert_eq!(explanation.outcome, ScheduledOutcomeKind::Runs);
            assert_eq!(explanation.deciding, CheckId::NeedsSignatures);
            assert!(
                check(&explanation, CheckId::NeedsSignatures)
                    .unwrap()
                    .allows
            );
            assert_eq!(
                key(&explanation, CheckId::NeedsSignatures),
                keys::NEEDS_SIGNATURES_NO
            );
            assert!(check(&explanation, CheckId::Covered).is_none());
            assert_eq!(explanation.authorized_by, None);
            assert_eq!(explanation.next_step.message_key, keys::NEXT_NONE);
        }
    }
}

#[test]
fn a_covered_transition_runs_authorized_by_the_approval() {
    for action in [OPEN, CLOSE] {
        for policy in CONFIGURATIONS {
            let explanation = evaluate(&inputs(
                action,
                copy(true, policy.clone()),
                published(true, policy.clone()),
                Coverage::Covered(approval()),
            ));
            assert_eq!(explanation.outcome, ScheduledOutcomeKind::Runs);
            assert_eq!(explanation.deciding, CheckId::Covered);
            assert_eq!(explanation.authorized_by, Some(approval()));
            let covered = check(&explanation, CheckId::Covered).unwrap();
            assert!(covered.allows);
            assert_eq!(covered.current.message_key, keys::COVERED_YES);
            assert_eq!(covered.current.params["code"], json!("K7Q-2M"));
            assert!(
                !check(&explanation, CheckId::NeedsSignatures)
                    .unwrap()
                    .allows
            );
            assert_eq!(
                check(&explanation, CheckId::NeedsSignatures)
                    .unwrap()
                    .current
                    .params["signatures"],
                json!(2)
            );
            // The policy doesn't matter to a covered close.
            assert!(check(&explanation, CheckId::UnsignedClose).is_none());
        }
    }
}

#[test]
fn an_uncovered_opening_is_refused_whatever_the_policy() {
    for policy in CONFIGURATIONS {
        for coverage in [
            Coverage::NoApproval,
            Coverage::NotInApproval(approval()),
            Coverage::Changed {
                by: approval(),
                edit: None,
            },
        ] {
            let explanation = evaluate(&inputs(
                OPEN,
                copy(true, policy.clone()),
                published(true, policy.clone()),
                coverage,
            ));
            assert_eq!(explanation.outcome, ScheduledOutcomeKind::Refused);
            assert_eq!(explanation.deciding, CheckId::Covered);
            assert!(!check(&explanation, CheckId::Covered).unwrap().allows);
            assert!(check(&explanation, CheckId::UnsignedClose).is_none());
            assert_eq!(
                explanation.next_step.message_key,
                keys::NEXT_PUBLISH_AND_APPROVE
            );
        }
    }
}

#[test]
fn the_covered_check_says_why_a_row_is_not_covered() {
    let reason = |coverage| {
        let explanation = evaluate(&inputs(
            OPEN,
            copy(true, REFUSE),
            published(true, REFUSE),
            coverage,
        ));
        check(&explanation, CheckId::Covered)
            .unwrap()
            .current
            .clone()
    };
    assert_eq!(
        reason(Coverage::NoApproval).message_key,
        keys::COVERED_NO_APPROVAL
    );
    let not_in = reason(Coverage::NotInApproval(approval()));
    assert_eq!(not_in.message_key, keys::COVERED_NOT_IN_APPROVAL);
    assert_eq!(not_in.params["code"], json!("K7Q-2M"));
    assert_eq!(
        reason(Coverage::Changed {
            by: approval(),
            edit: None
        })
        .message_key,
        keys::COVERED_CHANGED
    );
    let edited = reason(Coverage::Changed {
        by: approval(),
        edit: Some(("2028-04-02T10:00:00Z".to_owned(), "ofov.admin".to_owned())),
    });
    assert_eq!(edited.message_key, keys::COVERED_CHANGED_BY);
    assert_eq!(edited.params["edited_by"], json!("ofov.admin"));
    assert_eq!(edited.params["edited_at"], json!("2028-04-02T10:00:00Z"));
}

#[test]
fn an_uncovered_close_follows_the_policy_of_both_copies() {
    // Both refuse (the default): refused.
    let refused = evaluate(&inputs(
        CLOSE,
        copy(true, REFUSE),
        published(true, REFUSE),
        Coverage::NoApproval,
    ));
    assert_eq!(refused.outcome, ScheduledOutcomeKind::Refused);
    assert_eq!(refused.deciding, CheckId::UnsignedClose);
    let unsigned = check(&refused, CheckId::UnsignedClose).unwrap();
    assert!(!unsigned.allows);
    assert_eq!(unsigned.current.message_key, keys::UNSIGNED_CLOSE_REFUSE);
    assert_eq!(
        unsigned.published.as_ref().unwrap().message_key,
        keys::UNSIGNED_CLOSE_REFUSE
    );

    // Both run as system: closes without signatures.
    let runs = evaluate(&inputs(
        CLOSE,
        copy(true, RUN_AS_SYSTEM),
        published(true, RUN_AS_SYSTEM),
        Coverage::NoApproval,
    ));
    assert_eq!(runs.outcome, ScheduledOutcomeKind::RunsUnsigned);
    assert_eq!(runs.deciding, CheckId::UnsignedClose);
    assert!(check(&runs, CheckId::UnsignedClose).unwrap().allows);
    assert_eq!(runs.authorized_by, None);
    assert_eq!(runs.next_step.message_key, keys::NEXT_ASK_SIGNERS_TO_CLOSE);
    let fired = evaluate(&Inputs {
        moment: Moment::FireTime,
        ..inputs(
            CLOSE,
            copy(true, RUN_AS_SYSTEM),
            published(true, RUN_AS_SYSTEM),
            Coverage::NoApproval,
        )
    });
    assert_eq!(fired.outcome, ScheduledOutcomeKind::RunsUnsigned);
    assert_eq!(fired.next_step.message_key, keys::NEXT_NONE);
}

#[test]
fn a_loosened_policy_waits_for_the_next_approved_publication() {
    let explanation = evaluate(&inputs(
        CLOSE,
        copy(true, RUN_AS_SYSTEM),
        published(true, REFUSE),
        Coverage::NoApproval,
    ));
    assert_eq!(explanation.outcome, ScheduledOutcomeKind::Refused);
    assert_eq!(explanation.deciding, CheckId::StricterCopy);
    let stricter = check(&explanation, CheckId::StricterCopy).unwrap();
    assert!(!stricter.allows);
    assert_eq!(
        stricter.current.message_key,
        keys::STRICTER_COPY_CURRENT_LOOSER
    );
    assert!(check(&explanation, CheckId::Defaults).unwrap().allows);
    assert_eq!(
        explanation.next_step.message_key,
        keys::NEXT_PUBLISH_AND_APPROVE
    );
}

#[test]
fn a_tightened_policy_applies_at_once() {
    let explanation = evaluate(&inputs(
        CLOSE,
        copy(true, REFUSE),
        published(true, RUN_AS_SYSTEM),
        Coverage::NoApproval,
    ));
    assert_eq!(explanation.outcome, ScheduledOutcomeKind::Refused);
    // The current settings decide: the published copy holds nothing back.
    assert_eq!(explanation.deciding, CheckId::UnsignedClose);
    let stricter = check(&explanation, CheckId::StricterCopy).unwrap();
    assert!(stricter.allows);
    assert_eq!(
        stricter.current.message_key,
        keys::STRICTER_COPY_CURRENT_STRICTER
    );
}

#[test]
fn a_loosened_rule_waits_and_a_tightened_one_applies_at_once() {
    // Open voting no longer needs signatures, but the published copy does.
    let loosened = evaluate(&inputs(
        OPEN,
        copy(false, REFUSE),
        published(true, REFUSE),
        Coverage::NoApproval,
    ));
    assert_eq!(loosened.outcome, ScheduledOutcomeKind::Refused);
    assert_eq!(loosened.deciding, CheckId::StricterCopy);
    assert_eq!(
        key(&loosened, CheckId::StricterCopy),
        keys::STRICTER_COPY_CURRENT_LOOSER
    );
    let needs = check(&loosened, CheckId::NeedsSignatures).unwrap();
    assert_eq!(needs.current.message_key, keys::NEEDS_SIGNATURES_NO);
    assert_eq!(
        needs.published.as_ref().unwrap().message_key,
        keys::NEEDS_SIGNATURES_YES
    );

    // Open voting needs signatures now, though the published copy didn't.
    let tightened = evaluate(&inputs(
        OPEN,
        copy(true, REFUSE),
        published(false, REFUSE),
        Coverage::NoApproval,
    ));
    assert_eq!(tightened.outcome, ScheduledOutcomeKind::Refused);
    assert_eq!(tightened.deciding, CheckId::Covered);
    assert_eq!(
        key(&tightened, CheckId::StricterCopy),
        keys::STRICTER_COPY_CURRENT_STRICTER
    );
    assert!(check(&tightened, CheckId::StricterCopy).unwrap().allows);
}

#[test]
fn a_tightened_rule_with_a_loosened_policy_takes_both_copies() {
    // Close voting needs signatures now (applies at once), and the policy
    // was loosened (applies after the next approved publication).
    let explanation = evaluate(&inputs(
        CLOSE,
        copy(true, RUN_AS_SYSTEM),
        published(false, REFUSE),
        Coverage::NoApproval,
    ));
    assert_eq!(explanation.outcome, ScheduledOutcomeKind::Refused);
    assert_eq!(explanation.deciding, CheckId::StricterCopy);
    assert_eq!(
        key(&explanation, CheckId::StricterCopy),
        keys::STRICTER_COPY_COMBINED
    );
}

#[test]
fn before_the_first_publication_the_defaults_apply() {
    // The default policy refuses an unsigned close, whatever the current one says.
    for policy in CONFIGURATIONS {
        let explanation = evaluate(&inputs(
            CLOSE,
            copy(true, policy.clone()),
            PublishedCopy::Nothing,
            Coverage::NoApproval,
        ));
        assert_eq!(explanation.outcome, ScheduledOutcomeKind::Refused);
        assert_eq!(
            key(&explanation, CheckId::Defaults),
            keys::DEFAULTS_NOTHING_PUBLISHED
        );
        if policy == RUN_AS_SYSTEM {
            assert_eq!(explanation.deciding, CheckId::Defaults);
            assert!(!check(&explanation, CheckId::Defaults).unwrap().allows);
        } else {
            assert_eq!(explanation.deciding, CheckId::UnsignedClose);
            assert!(check(&explanation, CheckId::Defaults).unwrap().allows);
        }
    }
    // The default rules need no signatures: the current rule decides.
    let open = evaluate(&inputs(
        OPEN,
        copy(false, REFUSE),
        PublishedCopy::Nothing,
        Coverage::NoApproval,
    ));
    assert_eq!(open.outcome, ScheduledOutcomeKind::Runs);

    // A publication from before snapshots stands for the defaults too.
    let old = evaluate(&inputs(
        CLOSE,
        copy(true, RUN_AS_SYSTEM),
        PublishedCopy::NoSnapshot {
            publication_id: Uuid::from_u128(3),
            published_at: Utc.with_ymd_and_hms(2027, 1, 1, 0, 0, 0).unwrap(),
        },
        Coverage::NoApproval,
    ));
    assert_eq!(old.outcome, ScheduledOutcomeKind::Refused);
    assert_eq!(old.deciding, CheckId::Defaults);
    assert_eq!(key(&old, CheckId::Defaults), keys::DEFAULTS_NO_SNAPSHOT);

    let published = evaluate(&inputs(
        OPEN,
        copy(false, REFUSE),
        published(false, REFUSE),
        Coverage::NoApproval,
    ));
    assert_eq!(key(&published, CheckId::Defaults), keys::DEFAULTS_PUBLISHED);
    assert_eq!(
        key(&published, CheckId::StricterCopy),
        keys::STRICTER_COPY_SAME
    );
}

#[test]
fn the_next_step_says_what_would_change_the_outcome() {
    let refused = |action, approval_required, moment| {
        evaluate(&Inputs {
            approval_required,
            moment,
            ..inputs(
                action,
                copy(true, REFUSE),
                published(true, REFUSE),
                Coverage::NoApproval,
            )
        })
        .next_step
        .message_key
    };
    assert_eq!(
        refused(OPEN, true, Moment::Prediction),
        keys::NEXT_PUBLISH_AND_APPROVE
    );
    // Without signatures on Approve configuration nothing can ever cover it.
    assert_eq!(
        refused(OPEN, false, Moment::Prediction),
        keys::NEXT_REQUIRE_CONFIGURATION_APPROVAL
    );
    assert_eq!(
        refused(OPEN, true, Moment::FireTime),
        keys::NEXT_ASK_SIGNERS_TO_OPEN
    );
    assert_eq!(
        refused(CLOSE, true, Moment::FireTime),
        keys::NEXT_ASK_SIGNERS_TO_CLOSE
    );
}

#[test]
fn every_key_is_a_scheduled_outcome_key() {
    for key in keys::ALL {
        assert!(key.starts_with("scheduledOutcome."), "{key}");
    }
    let mut unique = keys::ALL.to_vec();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), keys::ALL.len());
}

#[test]
fn the_fingerprint_covers_what_the_row_does() {
    let row = |local: &str, channels: Value| {
        transition_of(
            "6d1e0e56-6c3f-4a39-9d4d-8d1f1f1b7a01",
            "START_VOTING_PERIOD",
            Some(&json!({
                "scheduled_date": "2028-04-08T20:00:00Z",
                "local": local,
                "timezone": "Asia/Dubai",
            })),
            Some(&json!({ "election_id": "p1", "voting_channels": channels })),
        )
        .unwrap()
    };
    let a = row("2028-04-09T00:00", json!(["ONLINE", "KIOSK"]));
    assert_eq!(a.fingerprint, fingerprint(&a));
    // The order of the channels doesn't change what the row does.
    assert_eq!(
        a.fingerprint,
        row("2028-04-09T00:00", json!(["KIOSK", "ONLINE"])).fingerprint
    );
    assert_ne!(
        a.fingerprint,
        row("2028-04-09T01:00", json!(["ONLINE", "KIOSK"])).fingerprint
    );
    assert_ne!(
        a.fingerprint,
        row("2028-04-09T00:00", json!(["ONLINE"])).fingerprint
    );
    assert_eq!(a.election_id.as_deref(), Some("p1"));
    assert!(transition_of("x", "ALLOW_TALLY", None, None).is_none());
}

#[test]
fn a_post_publication_signs_its_rows_and_the_event_wide_ones() {
    let post = Uuid::from_u128(1);
    let other = Uuid::from_u128(2);
    let row = |id: &str, election: Option<Uuid>| ScheduledRow {
        transition: transition_of(
            id,
            "END_VOTING_PERIOD",
            Some(&json!({ "scheduled_date": "2028-05-08T11:00:00Z" })),
            Some(&json!({ "election_id": election.map(|id| id.to_string()) })),
        )
        .unwrap(),
        annotations: json!({}),
        written_now: false,
    };
    let rows = vec![row("c", Some(other)), row("b", None), row("a", Some(post))];
    let ids = |target| {
        schedule_for(&rows, target)
            .into_iter()
            .map(|transition| transition.scheduled_event_id)
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(Some(post)), ["a", "b"]);
    assert_eq!(ids(None), ["a", "b", "c"]);
}

#[test]
fn rule_and_policy_saves_say_how_they_apply() {
    let rule_of = |action, required| {
        let mut rule = SigningRule::default_for(action);
        if required {
            rule.requirement = SigningRequirement::Required;
        }
        rule
    };
    assert_eq!(
        rule_change_applies(&rule_of(OPEN, false), &rule_of(OPEN, true)),
        Some(ChangeApplies::Tightens)
    );
    assert_eq!(
        rule_change_applies(&rule_of(CLOSE, true), &rule_of(CLOSE, false)),
        Some(ChangeApplies::Loosens)
    );
    assert_eq!(
        rule_change_applies(&rule_of(OPEN, true), &rule_of(OPEN, true)),
        None
    );
    assert_eq!(
        rule_change_applies(
            &rule_of(SigningAction::GenerateReports, false),
            &rule_of(SigningAction::GenerateReports, true)
        ),
        None
    );
    let with = |action, required, signatures| {
        let mut rule = rule_of(action, required);
        rule.signatures = signatures;
        rule
    };
    // More signatures tighten, fewer loosen.
    assert_eq!(
        rule_change_applies(&with(CLOSE, true, 2), &with(CLOSE, true, 3)),
        Some(ChangeApplies::Tightens)
    );
    assert_eq!(
        rule_change_applies(&with(CLOSE, true, 3), &with(CLOSE, true, 2)),
        Some(ChangeApplies::Loosens)
    );
    // Approve configuration decides what can cover a schedule.
    let approve = SigningAction::ApproveConfiguration;
    assert_eq!(
        rule_change_applies(&rule_of(approve, false), &rule_of(approve, true)),
        Some(ChangeApplies::Tightens)
    );
    assert_eq!(
        rule_change_applies(&rule_of(approve, true), &rule_of(approve, false)),
        Some(ChangeApplies::Loosens)
    );
    let policies = |scope, close| LifecyclePolicies {
        initialization_scope: scope,
        unsigned_scheduled_close: close,
    };
    use InitializationScope::{EVENT, POST, POST_AND_COUNTRY};
    assert_eq!(
        policy_change_applies(&policies(POST, REFUSE), &policies(POST, RUN_AS_SYSTEM)),
        Some(ChangeApplies::Loosens)
    );
    assert_eq!(
        policy_change_applies(&policies(POST, RUN_AS_SYSTEM), &policies(POST, REFUSE)),
        Some(ChangeApplies::Tightens)
    );
    assert_eq!(
        policy_change_applies(&policies(POST, REFUSE), &policies(EVENT, REFUSE)),
        Some(ChangeApplies::Tightens)
    );
    assert_eq!(
        policy_change_applies(&policies(POST_AND_COUNTRY, REFUSE), &policies(POST, REFUSE)),
        Some(ChangeApplies::Loosens)
    );
    assert_eq!(
        policy_change_applies(
            &policies(EVENT, REFUSE),
            &policies(POST_AND_COUNTRY, REFUSE)
        ),
        Some(ChangeApplies::TightensAndLoosens)
    );
    assert_eq!(
        policy_change_applies(&policies(POST, RUN_AS_SYSTEM), &policies(EVENT, REFUSE)),
        Some(ChangeApplies::Tightens)
    );
    assert_eq!(
        policy_change_applies(&policies(POST, REFUSE), &policies(POST, REFUSE)),
        None
    );
}

#[test]
fn the_log_words_name_the_outcome_and_the_deciding_check() {
    let transition = transition_of(
        "row",
        "END_VOTING_PERIOD",
        Some(&json!({"scheduled_date": "2028-05-08T11:00:00Z", "local": "2028-05-08T19:00", "timezone": "Asia/Manila"})),
        Some(&json!({"election_id": "p"})),
    )
    .unwrap();
    let before = evaluate(&inputs(
        CLOSE,
        copy(true, REFUSE),
        published(true, REFUSE),
        Coverage::Covered(approval()),
    ));
    let after = evaluate(&inputs(
        CLOSE,
        copy(true, REFUSE),
        published(true, REFUSE),
        Coverage::Changed {
            by: approval(),
            edit: Some(("2028-04-02T10:00:00Z".to_owned(), "ofov.admin".to_owned())),
        },
    ));
    let text = change_description(&transition, "Dubai PCG", Some(&before), &after);
    assert_eq!(
        text,
        "Scheduled close of Dubai PCG at 2028-05-08 19:00 Asia/Manila: now refused (was: runs, \
         authorized by configuration K7Q-2M). Deciding check: it needs signatures and isn't in \
         the signed configuration; a scheduled close without signatures is refused. Next step: \
         publish and approve the configuration."
    );
    assert_eq!(
        fired_words(CLOSE, &before, "Dubai PCG"),
        "Closed voting at Dubai PCG on schedule, authorized by the signed configuration K7Q-2M"
    );
}

fn initialization_state(post: Uuid) -> EventState {
    let now = Utc.with_ymd_and_hms(2028, 5, 8, 11, 0, 0).unwrap();
    let mut initialization = EventInitialization::default();
    initialization.posts.insert(
        post.to_string(),
        crate::services::initialization_scope::PostInitialization {
            name: "Post A".to_owned(),
            requires_report: true,
            initialized: false,
            areas: Default::default(),
            initialized_areas: Default::default(),
        },
    );
    EventState {
        tenant_id: Uuid::from_u128(1),
        election_event_id: Uuid::from_u128(2),
        policies: LifecyclePolicies::default(),
        rules: Rules {
            open_voting: RuleSnapshot::default(),
            close_voting: RuleSnapshot::default(),
            approve_configuration: false,
        },
        snapshots: vec![],
        publications: vec![],
        approvals: vec![],
        posts: vec![],
        post_channels: HashMap::new(),
        own_rows: HashSet::new(),
        fired: HashMap::new(),
        now,
        initialization,
        live_closes: vec![],
        live_rows: vec![],
        live_row_metadata: HashMap::new(),
        transaction_now: now,
    }
}

fn opening_row(post: Uuid) -> ScheduledRow {
    ScheduledRow {
        transition: transition_of(
            "opening",
            "START_VOTING_PERIOD",
            Some(&json!({"scheduled_date": "2028-05-08T10:00:00Z"})),
            Some(&json!({"election_id": post})),
        )
        .unwrap(),
        annotations: json!({}),
        written_now: false,
    }
}

#[test]
fn an_authorized_opening_waits_in_prediction_and_execution_until_initialized() {
    let post = Uuid::from_u128(3);
    let mut state = initialization_state(post);
    let row = opening_row(post);
    for moment in [Moment::Prediction, Moment::FireTime] {
        let explanation = state.explain(&row, Some(post), moment);
        assert_eq!(
            explanation.outcome,
            ScheduledOutcomeKind::WaitingForInitialization
        );
        assert_eq!(explanation.deciding, CheckId::Initialization);
    }
    state
        .initialization
        .posts
        .get_mut(&post.to_string())
        .unwrap()
        .initialized = true;
    assert_eq!(
        state.explain(&row, Some(post), Moment::Prediction).outcome,
        ScheduledOutcomeKind::Runs
    );
}

#[test]
fn a_signed_close_still_bounds_a_waiting_open_when_live_close_is_deleted_or_delayed() {
    let post = Uuid::from_u128(3);
    let mut state = initialization_state(post);
    state
        .initialization
        .posts
        .get_mut(&post.to_string())
        .unwrap()
        .initialized = true;
    let close = transition_of(
        "signed-close",
        "END_VOTING_PERIOD",
        Some(&json!({"scheduled_date": "2028-05-08T11:00:00Z"})),
        None,
    )
    .unwrap();
    state.approvals.push(Approval {
        request_id: Uuid::from_u128(4),
        code: "signed".into(),
        target: None,
        executed_at: state.now,
        schedule: vec![close],
        signers: vec![],
        post_channels: Default::default(),
        initialization_scope: InitializationScope::POST,
        initialization_report_policies: BTreeMap::new(),
    });
    let row = opening_row(post);
    for live in [vec![], vec![(None, "2028-05-09T11:00:00Z".to_owned())]] {
        state.live_closes = live;
        let explanation = state.explain(&row, Some(post), Moment::FireTime);
        assert_eq!(explanation.outcome, ScheduledOutcomeKind::Refused);
        assert_eq!(explanation.deciding, CheckId::VotingClose);
    }
    let later = ScheduledRow {
        transition: transition_of(
            "next-opening",
            "START_VOTING_PERIOD",
            Some(&json!({"scheduled_date": "2028-05-08T12:00:00Z"})),
            Some(&json!({"election_id": post})),
        )
        .unwrap(),
        annotations: json!({}),
        written_now: false,
    };
    assert_eq!(
        state
            .explain(&later, Some(post), Moment::Prediction)
            .outcome,
        ScheduledOutcomeKind::Refused
    );
    state.approvals[0].schedule.push(later.transition.clone());
    assert_eq!(
        state
            .explain(&later, Some(post), Moment::Prediction)
            .outcome,
        ScheduledOutcomeKind::Runs
    );
}

#[test]
fn published_initialization_scope_comes_from_executed_subject_and_is_target_specific() {
    let post = Uuid::from_u128(3);
    let other = Uuid::from_u128(5);
    let mut state = initialization_state(post);
    let approval = Uuid::from_u128(4);
    state.approvals.push(Approval {
        request_id: approval,
        code: "signed".into(),
        target: None,
        executed_at: state.now,
        schedule: vec![],
        signers: vec![],
        post_channels: Default::default(),
        initialization_scope: InitializationScope::POST_AND_COUNTRY,
        initialization_report_policies: BTreeMap::new(),
    });
    state.snapshots.push(StoredSnapshot {
        publication_id: Uuid::new_v4(),
        election_id: Some(other),
        approval_request_id: None,
        created_at: state.now,
        snapshot: LifecycleSnapshot::default(),
    });
    state.snapshots.push(StoredSnapshot {
        publication_id: Uuid::new_v4(),
        election_id: None,
        approval_request_id: Some(approval),
        created_at: state.now,
        snapshot: LifecycleSnapshot::default(),
    });
    assert_eq!(
        state.initialization_scopes(Some(post)).published,
        Some(InitializationScope::POST_AND_COUNTRY)
    );
    assert_eq!(
        state.initialization_scopes(Some(other)).published,
        Some(InitializationScope::POST)
    );
}

#[test]
fn schedule_previews_update_own_online_overrides() {
    let post = Uuid::from_u128(3);
    let state = initialization_state(post);
    let change = PendingChange::ScheduledEvent {
        id: Some("own-opening".into()),
        event_processor: "START_VOTING_PERIOD".into(),
        cron_config: Some(json!({"scheduled_date":"2028-05-08T10:00:00Z"})),
        event_payload: Some(json!({"election_id":post})),
    };
    let (after, rows, _) = apply_change(&state, &[], &change);
    assert!(after
        .own_rows
        .contains(&(post, "START_VOTING_PERIOD".into())));
    let removal = PendingChange::ScheduledEvent {
        id: Some("own-opening".into()),
        event_processor: "ALLOW_TALLY".into(),
        cron_config: None,
        event_payload: None,
    };
    let (after, _, _) = apply_change(&after, &rows, &removal);
    assert!(!after
        .own_rows
        .contains(&(post, "START_VOTING_PERIOD".into())));
}

#[test]
fn partial_firing_preserves_previous_posts_and_times_without_recording_waiting_posts() {
    let first = json!({"at": "2028-05-08T10:00:00Z", "posts": [
        {"election_id": "p1", "outcome": "runs"},
        {"election_id": "p2", "outcome": "waiting-for-initialization"},
    ]});
    let first = merge_fired_outcomes(None, &first);
    assert_eq!(first["posts"].as_array().unwrap().len(), 1);
    let second =
        json!({"at": "2028-05-08T10:01:00Z", "posts": [{"election_id": "p2", "outcome": "runs"}]});
    let merged = merge_fired_outcomes(Some(&first), &second);
    assert_eq!(merged["posts"].as_array().unwrap().len(), 2);
    assert_eq!(merged["posts"][0]["fired_at"], "2028-05-08T10:00:00Z");
    assert_eq!(merged["posts"][1]["fired_at"], "2028-05-08T10:01:00Z");
}

#[test]
fn a_preview_keeps_completed_own_rows_and_close_boundaries() {
    let post = Uuid::from_u128(3);
    let mut state = initialization_state(post);
    let completed_close = ScheduledRow {
        transition: transition_of(
            "completed-close",
            "END_VOTING_PERIOD",
            Some(&json!({"scheduled_date":"2028-05-08T11:00:00Z"})),
            Some(&json!({"election_id":post})),
        )
        .unwrap(),
        annotations: json!({}),
        written_now: false,
    };
    state.own_rows.insert((post, "END_VOTING_PERIOD".into()));
    state.live_row_metadata.insert(
        "completed-close".into(),
        ScheduleRowMetadata {
            task_id: Some(generate_manage_date_task_name(
                &state.tenant_id.to_string(),
                &state.election_event_id.to_string(),
                Some(&post.to_string()),
                &EventProcessors::END_VOTING_PERIOD,
            )),
            stopped: true,
        },
    );
    state.live_rows.push(completed_close);
    let change = PendingChange::ScheduledEvent {
        id: Some("other-open".into()),
        event_processor: "START_VOTING_PERIOD".into(),
        cron_config: Some(json!({"scheduled_date":"2028-05-08T10:00:00Z"})),
        event_payload: None,
    };
    let (after, _, _) = apply_change(&state, &[], &change);
    assert!(after.own_rows.contains(&(post, "END_VOTING_PERIOD".into())));
    assert_eq!(
        after.live_closes,
        vec![(Some(post.to_string()), "2028-05-08T11:00:00Z".into())]
    );
}

#[test]
fn removing_live_required_keeps_waiting_until_the_published_requirement_is_replaced() {
    let post = Uuid::from_u128(3);
    let mut state = initialization_state(post);
    state
        .initialization
        .posts
        .get_mut(&post.to_string())
        .unwrap()
        .requires_report = false;
    let mut required = LifecycleSnapshot::default();
    required
        .initialization_report_policies
        .insert(post.to_string(), EInitializeReportPolicy::REQUIRED);
    state.snapshots.push(StoredSnapshot {
        publication_id: Uuid::new_v4(),
        election_id: Some(post),
        approval_request_id: None,
        created_at: state.now,
        snapshot: required,
    });
    let row = opening_row(post);
    assert!(matches!(
        state.initialization_refusal_at(post),
        Some(TransitionRefusal::InitializationReportRequired)
    ));
    assert_eq!(
        state.explain(&row, Some(post), Moment::FireTime).outcome,
        ScheduledOutcomeKind::WaitingForInitialization
    );
    let mut replacement = LifecycleSnapshot::default();
    replacement
        .initialization_report_policies
        .insert(post.to_string(), EInitializeReportPolicy::NOT_REQUIRED);
    state.snapshots.insert(
        0,
        StoredSnapshot {
            publication_id: Uuid::new_v4(),
            election_id: Some(post),
            approval_request_id: None,
            created_at: state.now,
            snapshot: replacement,
        },
    );
    assert!(state.initialization_refusal_at(post).is_none());
    assert_eq!(
        state.explain(&row, Some(post), Moment::Prediction).outcome,
        ScheduledOutcomeKind::Runs
    );
    // Old snapshots have no retained report switch: preserve their established live behavior.
    state.snapshots[0]
        .snapshot
        .initialization_report_policies
        .clear();
    assert!(state.initialization_refusal_at(post).is_none());
    let old: LifecycleSnapshot = serde_json::from_value(json!({"policies":{}, "open_voting":{"required":false,"digest":""}, "close_voting":{"required":false,"digest":""}, "schedule":[]})).unwrap();
    assert!(old.initialization_report_policies.is_empty());
    assert!(serde_json::to_value(old)
        .unwrap()
        .get("initialization_report_policies")
        .is_none());
}

#[test]
fn event_scope_keeps_a_peers_published_report_requirement() {
    let post = Uuid::from_u128(3);
    let peer = Uuid::from_u128(5);
    let mut state = initialization_state(post);
    state
        .initialization
        .posts
        .get_mut(&post.to_string())
        .unwrap()
        .initialized = true;
    state.initialization.posts.insert(
        peer.to_string(),
        crate::services::initialization_scope::PostInitialization {
            name: "Peer".into(),
            requires_report: false,
            initialized: false,
            areas: Default::default(),
            initialized_areas: Default::default(),
        },
    );
    state.policies.initialization_scope = InitializationScope::EVENT;
    let mut snapshot = LifecycleSnapshot::default();
    snapshot
        .initialization_report_policies
        .insert(peer.to_string(), EInitializeReportPolicy::REQUIRED);
    state.snapshots.push(StoredSnapshot {
        publication_id: Uuid::new_v4(),
        election_id: None,
        approval_request_id: None,
        created_at: state.now,
        snapshot,
    });
    assert!(matches!(
        state.initialization_refusal_at(post),
        Some(TransitionRefusal::EventNotInitialized { .. })
    ));
}

#[test]
fn an_unrelated_preview_never_promotes_a_dummy_task_to_post_ownership() {
    let post = Uuid::from_u128(3);
    let mut state = initialization_state(post);
    let dummy = opening_row(post);
    state.live_row_metadata.insert(
        dummy.transition.scheduled_event_id.clone(),
        ScheduleRowMetadata {
            task_id: Some("dummy".into()),
            stopped: false,
        },
    );
    state.live_rows.push(dummy.clone());
    let change = PendingChange::ScheduledEvent {
        id: Some("unrelated".into()),
        event_processor: "START_VOTING_PERIOD".into(),
        cron_config: Some(json!({"scheduled_date":"2028-05-08T10:00:00Z"})),
        event_payload: None,
    };
    let (after, _, _) = apply_change(&state, &[dummy], &change);
    assert!(!after
        .own_rows
        .contains(&(post, "START_VOTING_PERIOD".into())));
}

#[test]
fn a_stopped_edit_rearms_only_in_the_future_and_uses_exact_channel_ownership() {
    let post = Uuid::from_u128(3);
    let mut state = initialization_state(post);
    let stopped = opening_row(post);
    state.live_row_metadata.insert(
        "opening".into(),
        ScheduleRowMetadata {
            task_id: Some(generate_manage_date_task_name(
                &state.tenant_id.to_string(),
                &state.election_event_id.to_string(),
                Some(&post.to_string()),
                &EventProcessors::START_VOTING_PERIOD,
            )),
            stopped: true,
        },
    );
    state.live_rows.push(stopped);
    state.own_rows.insert((post, "START_VOTING_PERIOD".into()));
    let edit = |date, channels| PendingChange::ScheduledEvent {
        id: Some("opening".into()),
        event_processor: "START_VOTING_PERIOD".into(),
        cron_config: Some(json!({"scheduled_date":date})),
        event_payload: Some(json!({"election_id":post, "voting_channels":channels})),
    };
    let (past, active, _) = apply_change(
        &state,
        &[],
        &edit("2028-05-08T10:00:00Z", json!(["ONLINE"])),
    );
    assert!(active.is_empty());
    assert!(past
        .own_rows
        .contains(&(post, "START_VOTING_PERIOD".into())));
    let (kiosk, active, _) =
        apply_change(&state, &[], &edit("2028-05-08T10:00:00Z", json!(["KIOSK"])));
    assert!(active.is_empty());
    assert!(!kiosk
        .own_rows
        .contains(&(post, "START_VOTING_PERIOD".into())));
    let (future, active, _) = apply_change(
        &state,
        &[],
        &edit("2028-05-08T12:00:00Z", json!(["ONLINE"])),
    );
    assert_eq!(active.len(), 1);
    assert!(!future.live_row_metadata["opening"].stopped);
}

#[test]
fn a_dummy_or_kiosk_own_close_cannot_mask_the_common_online_close() {
    let post = Uuid::from_u128(3);
    for (channels, task_is_valid) in [(json!(["ONLINE"]), false), (json!(["KIOSK"]), true)] {
        let mut state = initialization_state(post);
        state
            .initialization
            .posts
            .get_mut(&post.to_string())
            .unwrap()
            .initialized = true;
        let common = ScheduledRow {
            transition: transition_of(
                "common",
                "END_VOTING_PERIOD",
                Some(&json!({"scheduled_date":"2028-05-08T11:00:00Z"})),
                None,
            )
            .unwrap(),
            annotations: json!({}),
            written_now: false,
        };
        let own = ScheduledRow {
            transition: transition_of(
                "own",
                "END_VOTING_PERIOD",
                Some(&json!({"scheduled_date":"2028-05-08T13:00:00Z"})),
                Some(&json!({"election_id":post, "voting_channels":channels})),
            )
            .unwrap(),
            annotations: json!({}),
            written_now: false,
        };
        let task = if task_is_valid {
            generate_manage_date_task_name(
                &state.tenant_id.to_string(),
                &state.election_event_id.to_string(),
                Some(&post.to_string()),
                &EventProcessors::END_VOTING_PERIOD,
            )
        } else {
            "dummy".into()
        };
        state.live_row_metadata.insert(
            "own".into(),
            ScheduleRowMetadata {
                task_id: Some(task),
                stopped: false,
            },
        );
        state.live_rows = vec![common, own];
        let explanation = state.explain(&opening_row(post), Some(post), Moment::FireTime);
        assert_eq!(explanation.outcome, ScheduledOutcomeKind::Refused);
        assert_eq!(explanation.deciding, CheckId::VotingClose);
    }
}

#[test]
fn signed_post_opening_precedence_does_not_depend_on_live_task_ownership() {
    let post = Uuid::from_u128(3);
    let mut state = initialization_state(post);
    state
        .initialization
        .posts
        .get_mut(&post.to_string())
        .unwrap()
        .initialized = true;
    let own = opening_row(post);
    let global = ScheduledRow {
        transition: transition_of(
            "global-opening",
            "START_VOTING_PERIOD",
            Some(&json!({"scheduled_date":"2028-05-08T09:00:00Z"})),
            None,
        )
        .unwrap(),
        annotations: json!({}),
        written_now: false,
    };
    state.approvals.push(Approval {
        request_id: Uuid::new_v4(),
        code: "approved".into(),
        target: None,
        executed_at: state.now,
        schedule: vec![global.transition.clone(), own.transition],
        signers: vec![],
        post_channels: BTreeMap::new(),
        initialization_scope: InitializationScope::POST,
        initialization_report_policies: BTreeMap::new(),
    });
    assert!(state.own_rows.is_empty()); // live own task was removed or tampered
    let result = state.explain(&global, Some(post), Moment::FireTime);
    assert_eq!(result.outcome, ScheduledOutcomeKind::Refused);
    assert_eq!(result.deciding, CheckId::Covered);
    assert_eq!(
        key(&result, CheckId::Covered),
        keys::COVERED_OVERRIDDEN_BY_SIGNED_POST
    );
    state.approvals[0]
        .schedule
        .retain(|entry| entry.election_id.is_none());
    assert_eq!(
        state.explain(&global, Some(post), Moment::FireTime).outcome,
        ScheduledOutcomeKind::Runs
    );
}

#[test]
fn signed_snapshot_uses_subject_and_rejects_wrong_reference() {
    let publication = Uuid::new_v4();
    let request = Some(Uuid::new_v4());
    let authoritative = LifecycleSnapshot {
        open_voting: RuleSnapshot {
            required: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let subject = crate::services::signing::actions::configuration::ConfigurationSubject {
        ballot_publication_id: publication.to_string(),
        digest: "signed".into(),
        signing_rules: vec![],
        scheduled_events: 0,
        ballots_and_contests: "first-version".into(),
        policies: authoritative.policies.clone(),
        open_voting: authoritative.open_voting.clone(),
        close_voting: authoritative.close_voting.clone(),
        schedule: vec![],
        post_channels: BTreeMap::new(),
        initialization_report_policies: BTreeMap::new(),
    };
    let forged = serde_json::to_value(LifecycleSnapshot::default()).unwrap();
    let signed = serde_json::to_value(subject).unwrap();
    assert_eq!(
        decode_stored_snapshot(
            forged.clone(),
            request,
            Some(signed.clone()),
            Some(target_scope_key(None)),
            publication,
            None
        )
        .unwrap(),
        authoritative
    );
    assert!(
        decode_stored_snapshot(forged.clone(), request, None, None, publication, None).is_err()
    );
    assert!(decode_stored_snapshot(
        forged.clone(),
        request,
        Some(signed.clone()),
        Some(target_scope_key(None)),
        Uuid::new_v4(),
        None
    )
    .is_err());
    assert!(decode_stored_snapshot(
        forged,
        request,
        Some(signed),
        Some(target_scope_key(None)),
        publication,
        Some(Uuid::new_v4())
    )
    .is_err());
}

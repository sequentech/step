// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! These policy values are embedded in hashed ballot styles. Pin their public
//! JSON names and Borsh discriminants so reordering an enum cannot silently
//! change the meaning or signature of an already published election.

use borsh::{BorshDeserialize, BorshSerialize};
use sequent_core::ballot::*;
use sequent_core::types::ceremonies::{
    AutomaticRecountPolicy, CeremoniesPolicy, CountingAlgType, TallyOperation,
    TieBreakingMethod,
};
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

#[path = "support/borsh.rs"]
mod wire;

fn assert_policy_wire<T>(value: T, name: &str, discriminant: u8)
where
    T: Serialize
        + DeserializeOwned
        + BorshSerialize
        + BorshDeserialize
        + PartialEq
        + Debug,
{
    let json = serde_json::Value::String(name.into());
    assert_eq!(serde_json::to_value(&value).unwrap(), json);
    assert_eq!(serde_json::from_value::<T>(json).unwrap(), value);
    wire::assert_stream_contract(&value, &[discriminant]);
    assert!(
        borsh::from_slice::<T>(&[255]).is_err(),
        "unknown discriminants must not fall back to another policy"
    );
}

// Explicit names and discriminants are compatibility fixtures, independent of
// enum iteration or the serializer. Adding/reordering a variant must not rewrite
// these expectations. The macro only supplies the same assertions to each type.
macro_rules! policy_contract {
    ($test:ident, $policy:ident, $( $variant:ident => ($name:literal, $byte:literal) ),+ $(,)?) => {
        #[test]
        fn $test() {
            $(assert_policy_wire($policy::$variant, $name, $byte);)+
            assert!(serde_json::from_str::<$policy>("\"unknown-policy\"").is_err());
        }
    };
}

policy_contract!(candidate_order, CandidatesOrder,
    Random => ("random", 0),
    Custom => ("custom", 1),
    Alphabetical => ("alphabetical", 2),
);
policy_contract!(contest_order, ContestsOrder,
    Random => ("random", 0),
    Custom => ("custom", 1),
    Alphabetical => ("alphabetical", 2),
);
policy_contract!(election_order, ElectionsOrder,
    Random => ("random", 0),
    Custom => ("custom", 1),
    Alphabetical => ("alphabetical", 2),
);
policy_contract!(early_voting, EarlyVotingPolicy,
    AllowEarlyVoting => ("allow_early_voting", 0),
    NoEarlyVoting => ("no_early_voting", 1),
);
policy_contract!(cast_vote_assurance, CastVoteGoldLevelPolicy,
    GoldLevel => ("gold-level", 0),
    NoGoldLevel => ("no-gold-level", 1),
);
policy_contract!(start_title, StartScreenTitlePolicy,
    Election => ("election", 0),
    ElectionEvent => ("election-event", 1),
);
policy_contract!(security_confirmation, ESecurityConfirmationPolicy,
    NONE => ("none", 0),
    MANDATORY => ("mandatory", 1),
);
policy_contract!(audit_button, AuditButtonCfg,
    SHOW => ("show", 0),
    NOT_SHOW => ("not-show", 1),
    SHOW_IN_HELP => ("show-in-help", 2),
);
policy_contract!(cast_vote_logs, ShowCastVoteLogs,
    ShowLogsTab => ("show-logs-tab", 0),
    HideLogsTab => ("hide-logs-tab", 1),
);
policy_contract!(invalid_votes, InvalidVotePolicy,
    ALLOWED => ("allowed", 0),
    WARN => ("warn", 1),
    WARN_INVALID_IMPLICIT_AND_EXPLICIT => ("warn-invalid-implicit-and-explicit", 2),
    NOT_ALLOWED => ("not-allowed", 3),
    ALLOWED_WITH_EXCLUSIVE_EXPLICIT => ("allowed-with-exclusive-explicit", 4),
);
policy_contract!(candidate_selection, CandidatesSelectionPolicy,
    RADIO => ("radio", 0),
    CUMULATIVE => ("cumulative", 1),
);
policy_contract!(checkbox_icon, CandidatesIconCheckboxPolicy,
    SQUARE_CHECKBOX => ("square-checkbox", 0),
    ROUND_CHECKBOX => ("round-checkbox", 1),
);
policy_contract!(key_ceremony, KeysCeremonyPolicy,
    ELECTION_EVENT => ("ELECTION_EVENT", 0),
    ELECTION => ("ELECTION", 1),
);
policy_contract!(support_materials, SupportMaterialsPolicy,
    Off => ("off", 0),
    Optional => ("optional", 1),
    MandatoryForVoting => ("mandatory_for_voting", 2),
);
policy_contract!(results_status, ResultsWebsiteStatus,
    Enabled => ("enabled", 0),
    Disabled => ("disabled", 1),
);
policy_contract!(results_access, ResultsWebsiteAccess,
    Public => ("public", 0),
    Authenticated => ("authenticated", 1),
);
policy_contract!(results_scope, ResultsWebsiteVisibilityScope,
    FullEvent => ("full_event", 0),
    AreaBased => ("area_based", 1),
);
policy_contract!(grace_period, EGracePeriodPolicy,
    NO_GRACE_PERIOD => ("no-grace-period", 0),
    GRACE_PERIOD_WITHOUT_ALERT => ("grace-period-without-alert", 1),
);
policy_contract!(initialization_report, EInitializeReportPolicy,
    REQUIRED => ("required", 0),
    NOT_REQUIRED => ("not-required", 1),
);
policy_contract!(countdown, ECountdownPolicy,
    NO_COUNTDOWN => ("NO_COUNTDOWN", 0),
    COUNTDOWN => ("COUNTDOWN", 1),
    COUNTDOWN_WITH_ALERT => ("COUNTDOWN_WITH_ALERT", 2),
);
policy_contract!(under_votes, EUnderVotePolicy,
    ALLOWED => ("allowed", 0),
    WARN => ("warn", 1),
    WARN_ONLY_IN_REVIEW => ("warn-only-in-review", 2),
    WARN_AND_ALERT => ("warn-and-alert", 3),
);
policy_contract!(blank_votes, EBlankVotePolicy,
    ALLOWED => ("allowed", 0),
    WARN => ("warn", 1),
    WARN_ONLY_IN_REVIEW => ("warn-only-in-review", 2),
    NOT_ALLOWED => ("not-allowed", 3),
);
policy_contract!(over_votes, EOverVotePolicy,
    ALLOWED => ("allowed", 0),
    ALLOWED_WITH_MSG => ("allowed-with-msg", 1),
    ALLOWED_WITH_MSG_AND_ALERT => ("allowed-with-msg-and-alert", 2),
    NOT_ALLOWED_WITH_MSG_AND_ALERT => ("not-allowed-with-msg-and-alert", 3),
    NOT_ALLOWED_WITH_MSG_AND_DISABLE => ("not-allowed-with-msg-and-disable", 4),
);
policy_contract!(duplicate_ranks, EDuplicatedRankPolicy,
    ALLOWED_WARN_AND_DIALOG => ("allowed-warn-and-dialog", 0),
    NOT_ALLOWED_WARN_AND_DIALOG => ("not-allowed-warn-and-dialog", 1),
);
policy_contract!(preference_gaps, EPreferenceGapsPolicy,
    ALLOWED_WARN_AND_DIALOG => ("allowed-warn-and-dialog", 0),
    NOT_ALLOWED_WARN_AND_DIALOG => ("not-allowed-warn-and-dialog", 1),
);
policy_contract!(enrollment, Enrollment,
    ENABLED => ("enabled", 0),
    DISABLED => ("disabled", 1),
);
policy_contract!(otp, Otp,
    ENABLED => ("enabled", 0),
    DISABLED => ("disabled", 1),
);
policy_contract!(voter_signing, VoterSigningPolicy,
    NO_SIGNATURE => ("no-signature", 0),
    WITH_SIGNATURE => ("with-signature", 1),
);
policy_contract!(voter_certificate, VoterCertificatePolicy,
    DISABLED => ("disabled", 0),
    ENABLED => ("enabled", 1),
);
policy_contract!(lockdown, LockedDown,
    LOCKED_DOWN => ("locked-down", 0),
    NOT_LOCKED_DOWN => ("not-locked-down", 1),
);
policy_contract!(publication, Publish,
    ALWAYS => ("always", 0),
    AFTER_LOCKDOWN => ("after-lockdown", 1),
);
policy_contract!(init_report, InitReport,
    ALLOWED => ("allowed", 0),
    DISALLOWED => ("disallowed", 1),
);
policy_contract!(manual_start, ManualStartVotingPeriod,
    ALLOWED => ("allowed", 0),
    ONLY_WHEN_INITIALIZATION_REPORT_HAS_BEEN_PERFORMED => ("only-when-initialization-report-has-been-performed", 1),
);
policy_contract!(voting_period_end, VotingPeriodEnd,
    ALLOWED => ("allowed", 0),
    DISALLOWED => ("disallowed", 1),
);
policy_contract!(tally, Tally,
    ALWAYS_ALLOW => ("always-allow", 0),
    ONLY_WHEN_VOTING_PERIOD_ENDS => ("allow-when-voting-period-ends", 1),
);
policy_contract!(allow_tally_status, AllowTallyStatus,
    ALLOWED => ("allowed", 0),
    DISALLOWED => ("disallowed", 1),
    REQUIRES_VOTING_PERIOD_END => ("requires-voting-period-end", 2),
);
policy_contract!(consolidated_report, ConsolidatedReportPolicy,
    DO_NOT_GENERATE => ("do-not-generate", 0),
    GENERATE => ("generate", 1),
);
policy_contract!(voting_screen_back, VotingScreenBackPolicy,
    ELECTION_SELECTION_SCREEN => ("election-selection-screen", 0),
    START_SCREEN => ("start-screen", 1),
);
policy_contract!(ceremony_automation, CeremoniesPolicy,
    MANUAL_CEREMONIES => ("manual-ceremonies", 0),
    AUTOMATED_CEREMONIES => ("automated-ceremonies", 1),
);
policy_contract!(automatic_recount, AutomaticRecountPolicy,
    ENABLED => ("enabled", 0),
    DISABLED => ("disabled", 1),
);
policy_contract!(tally_operation, TallyOperation,
    ProcessBallotsAll => ("process-ballots-all", 0),
    AggregateResults => ("aggregate-results", 1),
    SkipCandidateResults => ("skip-candidate-results", 2),
);
policy_contract!(tie_breaking_method, TieBreakingMethod,
    Random => ("Random", 0),
    ExternalProcedure => ("ExternalProcedure", 1),
);
policy_contract!(counting_algorithm, CountingAlgType,
    PluralityAtLarge => ("plurality-at-large", 0),
    InstantRunoff => ("instant-runoff", 1),
    BordaNauru => ("borda-nauru", 2),
    Borda => ("borda", 3),
    BordaMasMadrid => ("borda-mas-madrid", 4),
    PairwiseBeta => ("pairwise-beta", 5),
    Desborda3 => ("desborda3", 6),
    Desborda2 => ("desborda2", 7),
    Desborda => ("desborda", 8),
    Cumulative => ("cumulative", 9),
);

#[test]
fn datetime_format_presets_and_custom_payload_keep_distinct_encodings() {
    for (value, name, byte) in [
        (VotingPortalDateTimeFormat::LegacyGb24h, "legacy-gb-24h", 0),
        (VotingPortalDateTimeFormat::IsoLocal, "iso-local", 1),
        (VotingPortalDateTimeFormat::Us12h, "us-12h", 2),
        (VotingPortalDateTimeFormat::LocaleMedium, "locale-medium", 3),
        (VotingPortalDateTimeFormat::DateOnly, "date-only", 4),
    ] {
        assert_policy_wire(value, name, byte);
    }
    let custom = VotingPortalDateTimeFormat::Custom("yyyy".into());
    assert_eq!(
        serde_json::to_value(&custom).unwrap(),
        serde_json::json!({"custom": "yyyy"})
    );
    // Discriminant 5, followed by a little-endian u32 UTF-8 byte length.
    wire::assert_stream_contract(
        &custom,
        &[5, 4, 0, 0, 0, b'y', b'y', b'y', b'y'],
    );
    assert!(borsh::from_slice::<VotingPortalDateTimeFormat>(&[
        5, 1, 0, 0, 0, 255
    ])
    .is_err());
}

#[test]
fn encrypted_ballot_layout_policies_keep_their_published_wire_values() {
    assert_policy_wire(
        ContestEncryptionPolicy::MULTIPLE_CONTESTS,
        "multiple-contests",
        0,
    );
    assert_policy_wire(
        ContestEncryptionPolicy::SINGLE_CONTEST,
        "single-contest",
        1,
    );
    assert_policy_wire(MultiContestEncodingMode::LEGACY, "legacy", 0);
    assert_policy_wire(
        MultiContestEncodingMode::EXPANDED_CAPACITY,
        "expanded-capacity",
        1,
    );
    assert_policy_wire(DecodedBallotsInclusionPolicy::INCLUDED, "included", 0);
    assert_policy_wire(
        DecodedBallotsInclusionPolicy::NOT_INCLUDED,
        "not-included",
        1,
    );
    assert_policy_wire(DeclineToVotePolicy::DISABLED, "disabled", 0);
    assert_policy_wire(DeclineToVotePolicy::ENABLED, "enabled", 1);
    assert_policy_wire(BlankBallotsPolicy::DISABLED, "disabled", 0);
    assert_policy_wire(BlankBallotsPolicy::ENABLED, "enabled", 1);
}

#[test]
fn voting_weight_and_tie_policies_keep_distinct_wire_values() {
    assert_policy_wire(
        WeightedVotingPolicy::DISABLED_WEIGHTED_VOTING,
        "disabled-weighted-voting",
        0,
    );
    assert_policy_wire(
        WeightedVotingPolicy::AREAS_WEIGHTED_VOTING,
        "areas-weighted-voting",
        1,
    );
    assert_policy_wire(
        WeightedVotingPolicy::VOTERS_WEIGHTED_VOTING,
        "voters-weighted-voting",
        2,
    );
    assert_policy_wire(DelegatedVotingPolicy::DISABLED, "disabled", 0);
    assert_policy_wire(DelegatedVotingPolicy::ENABLED, "enabled", 1);
    assert_policy_wire(TieBreakingPolicy::RANDOM, "random", 0);
    assert_policy_wire(
        TieBreakingPolicy::EXTERNAL_PROCEDURE,
        "external-procedure",
        1,
    );
}

#[test]
fn language_and_voting_state_wire_values_do_not_change_between_surfaces() {
    assert_policy_wire(
        LanguageDetectionPolicy::BROWSER_DETECT,
        "browser-detect",
        0,
    );
    assert_policy_wire(
        LanguageDetectionPolicy::FORCE_DEFAULT,
        "force-default",
        1,
    );
    assert_policy_wire(VotingStatus::NOT_STARTED, "NOT_STARTED", 0);
    assert_policy_wire(VotingStatus::OPEN, "OPEN", 1);
    assert_policy_wire(VotingStatus::PAUSED, "PAUSED", 2);
    assert_policy_wire(VotingStatus::CLOSED, "CLOSED", 3);
    assert_policy_wire(VotingStatusChannel::ONLINE, "ONLINE", 0);
    assert_policy_wire(VotingStatusChannel::KIOSK, "KIOSK", 1);
    assert_policy_wire(VotingStatusChannel::EARLY_VOTING, "EARLY_VOTING", 2);
    assert_policy_wire(VotingStatusChannel::TELEPHONE, "TELEPHONE", 3);
}

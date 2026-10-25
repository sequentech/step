// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::require_fresh_board_approval;
use crate::test_claims::Claims;
use chrono::{DateTime, Utc};
use rocket::http::Status;
use sequent_core::signing::SigningAction;

const BOARD_ACTIONS: [SigningAction; 8] = [
    SigningAction::InitializeVoting,
    SigningAction::OpenVoting,
    SigningAction::CloseVoting,
    SigningAction::GenerateElectionReturns,
    SigningAction::GenerateReports,
    SigningAction::TransmitResults,
    SigningAction::ApproveVoter,
    SigningAction::ApproveConfiguration,
];

#[test]
fn every_board_approval_requires_fresh_external_authentication() {
    let now = DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap();
    let mut claims = Claims::new("tenant", "board").build();
    claims.acr = "silver".into();
    claims.iat = now.timestamp();
    for action in BOARD_ACTIONS {
        for auth_time in
            [None, Some(now.timestamp() - 60), Some(now.timestamp() + 1)]
        {
            claims.auth_time = auth_time;
            let refusal =
                require_fresh_board_approval(&claims, action, now).unwrap_err();
            assert_eq!(refusal.status, Status::Forbidden);
            assert_eq!(
                refusal.extensions["reason"],
                "reauthentication-required"
            );
        }
        claims.auth_time = Some(now.timestamp() - 59);
        assert!(require_fresh_board_approval(&claims, action, now).is_ok());
    }
}

#[test]
fn stale_before_commit_cannot_reuse_an_earlier_successful_check() {
    let now = DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap();
    let mut claims = Claims::new("tenant", "board").build();
    claims.auth_time = Some(now.timestamp() - 59);
    assert!(require_fresh_board_approval(
        &claims,
        SigningAction::CloseVoting,
        now
    )
    .is_ok());
    assert!(require_fresh_board_approval(
        &claims,
        SigningAction::CloseVoting,
        now + chrono::Duration::seconds(1)
    )
    .is_err());
}

#[test]
fn trustee_custody_actions_keep_their_existing_authentication_contract() {
    let now = DateTime::<Utc>::from_timestamp(1_800_000_000, 0).unwrap();
    let mut claims = Claims::new("tenant", "trustee").build();
    claims.auth_time = None;
    for action in [
        SigningAction::ConfirmCustodyProof,
        SigningAction::ContributeCustodyProof,
    ] {
        assert!(require_fresh_board_approval(&claims, action, now).is_ok());
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::domain::trustee_signatures::TrusteeSignatures;
use sequent_core::types::ceremonies::{CeremoniesPolicy, TallyTrustee, TallyTrusteeStatus};

#[derive(Debug, PartialEq, Eq)]
pub enum TrusteeSelection {
    NoCeremony,
    Insufficient { available: usize, threshold: usize },
    Ready(Vec<String>),
}

/// The trustees a tally may use: every one with automated ceremonies,
/// else those who restored their key, signed when the rule makes them sign.
pub fn eligible_trustees(
    trustees: Vec<TallyTrustee>,
    policy: CeremoniesPolicy,
    signatures: &TrusteeSignatures,
) -> Vec<String> {
    trustees
        .into_iter()
        .filter(|trustee| {
            policy == CeremoniesPolicy::AUTOMATED_CEREMONIES
                || (trustee.status == TallyTrusteeStatus::KEY_RESTORED
                    && signatures.counts(&trustee.name))
        })
        .map(|trustee| trustee.name)
        .collect()
}

pub fn select_trustees(ordered: Vec<String>, threshold: usize) -> TrusteeSelection {
    let selected: Vec<_> = ordered.into_iter().take(threshold).collect();
    if selected.len() < threshold {
        TrusteeSelection::Insufficient {
            available: selected.len(),
            threshold,
        }
    } else {
        TrusteeSelection::Ready(selected)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum BoardMessagePlan {
    New(usize),
    Replay,
    Wait,
}

pub fn board_message_plan(ids: &[i64], last_processed: i64, replay: bool) -> BoardMessagePlan {
    if let Some(index) = ids.iter().position(|id| *id > last_processed) {
        BoardMessagePlan::New(index)
    } else if replay {
        BoardMessagePlan::Replay
    } else {
        BoardMessagePlan::Wait
    }
}

pub fn execution_is_complete(plaintext_count: usize, batch_count: usize) -> bool {
    plaintext_count == batch_count
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionConclusion {
    AwaitingInput,
    InProgress,
    Completed,
}

pub fn execution_conclusion(pending_ties: bool, complete: bool) -> ExecutionConclusion {
    if pending_ties {
        ExecutionConclusion::AwaitingInput
    } else if complete {
        ExecutionConclusion::Completed
    } else {
        ExecutionConclusion::InProgress
    }
}

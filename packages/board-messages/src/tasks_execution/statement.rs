// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::electoral_log::newtypes::*;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use strum::Display;

use super::newtypes::{ElectionEventIdString, TaskExecutionType, TenantIdString};

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

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
pub struct StatementHead {
    pub event: ElectionEventIdString,
    pub kind: StatementType,
    pub timestamp: Timestamp,
}
impl StatementHead {
    pub fn from_body(event: ElectionEventIdString, body: &StatementBody) -> Self {
        let kind = match body {
            StatementBody::startTask(_, _, _) => StatementType::StartTask,
        };
        let timestamp = crate::timestamp();

        StatementHead {
            event,
            kind,
            timestamp,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Deserialize, Serialize, Debug)]
pub enum StatementBody {
    startTask(TenantIdString, ElectionEventIdString, TaskExecutionType),
}

#[derive(BorshSerialize, BorshDeserialize, Display, Deserialize, Serialize, Debug)]
pub enum StatementType {
    StartTask,
    // CastVoteError,
    // ElectionPublish,
    // ElectionVotingPeriodOpen,
    // ElectionVotingPeriodClose,
    // ElectionVotingPeriodPause,
    // ElectionEventVotingPeriodOpen,
    // ElectionEventVotingPeriodClose,
    // ElectionEventVotingPeriodPause,
    // KeyGeneration,
    // KeyInsertionStart,
    // KeyInsertionCeremony,
    // TallyOpen,
    // TallyClose,
    // SendCommunication,
}

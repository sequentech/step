// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Typed views and validation helpers for JSON fields stored in Hasura rows.

use super::core::{Candidate, Contest, Election, ElectionEvent};
use crate::ballot::{
    CandidatePresentation, ContestPresentation, ElectionEventPresentation,
    ElectionEventStatistics, ElectionEventStatus, ElectionPresentation,
    ElectionStatistics, ElectionStatus,
};
use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::default::Default;
use strum_macros::{Display, EnumString};

/// Voting channels configuration.
#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub struct VotingChannels {
    /// Online voting channel.
    pub online: Option<bool>,
    /// In-person kiosk voting channel.
    pub kiosk: Option<bool>,
    /// Telephone voting channel.
    pub telephone: Option<bool>,
    /// Paper ballot voting channel.
    pub paper: Option<bool>,
}

/// Reference to the ImmuDB bulletin board attached to an election event.
#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub struct BulletinBoardReference {
    /// ImmuDB database identifier.
    pub id: i64,
    /// ImmuDB database name.
    pub database_name: String,
    /// When true, the bulletin board is archived and read-only.
    pub is_archived: bool,
}

impl ElectionEvent {
    /// Validates that JSON columns deserialize into their expected typed views.
    pub fn validate(&self) -> Result<()> {
        if let Some(presentation) = &self.presentation {
            serde_json::from_value::<ElectionEventPresentation>(
                presentation.clone(),
            )?;
        }

        if let Some(voting_channels) = &self.voting_channels {
            serde_json::from_value::<VotingChannels>(voting_channels.clone())?;
        }

        if let Some(status) = &self.status {
            serde_json::from_value::<ElectionEventStatus>(status.clone())?;
        }

        if let Some(statistics) = &self.statistics {
            serde_json::from_value::<ElectionEventStatistics>(
                statistics.clone(),
            )?;
        }

        if let Some(bulletin_board_reference) = &self.bulletin_board_reference {
            serde_json::from_value::<BulletinBoardReference>(
                bulletin_board_reference.clone(),
            )?;
        }

        Ok(())
    }
}

impl Election {
    /// Validates that JSON columns deserialize into their expected typed views.
    pub fn validate(&self) -> Result<()> {
        if let Some(presentation) = &self.presentation {
            serde_json::from_value::<ElectionPresentation>(
                presentation.clone(),
            )?;
        }

        if let Some(voting_channels) = &self.voting_channels {
            serde_json::from_value::<VotingChannels>(voting_channels.clone())?;
        }

        if let Some(status) = &self.status {
            serde_json::from_value::<ElectionStatus>(status.clone())?;
        }

        if let Some(statistics) = &self.statistics {
            serde_json::from_value::<ElectionStatistics>(statistics.clone())?;
        }

        Ok(())
    }
}

impl Contest {
    /// Validates that JSON columns deserialize into their expected typed views.
    pub fn validate(&self) -> Result<()> {
        if let Some(presentation) = &self.presentation {
            serde_json::from_value::<ContestPresentation>(
                presentation.clone(),
            )?;
        }

        Ok(())
    }
}

impl Candidate {
    /// Validates that JSON columns deserialize into their expected typed views.
    pub fn validate(&self) -> Result<()> {
        if let Some(presentation) = &self.presentation {
            serde_json::from_value::<CandidatePresentation>(
                presentation.clone(),
            )?;
        }

        Ok(())
    }
}

/// Lifecycle status of a [`super::core::TasksExecution`] record.
#[derive(
    Display,
    Serialize,
    Deserialize,
    Debug,
    PartialEq,
    Eq,
    Clone,
    EnumString,
    Default,
    JsonSchema,
)]
pub enum TasksExecutionStatus {
    /// The worker is still processing the task.
    #[default]
    IN_PROGRESS,
    /// The task completed successfully.
    SUCCESS,
    /// The task failed with an error.
    FAILED,
    /// The task was cancelled before completion.
    CANCELLED,
}

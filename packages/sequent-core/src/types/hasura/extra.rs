// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::core::{Candidate, Contest, Election, ElectionEvent};
use crate::ballot::{
    CandidatePresentation, ContestPresentation, ElectionEventPresentation,
    ElectionEventStatistics, ElectionEventStatus, ElectionPresentation,
    ElectionStatistics, ElectionStatus,
};
use anyhow::Result;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::default::Default;
use strum_macros::{Display, EnumString};

/// Store if voting is enabled in each channel.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub struct VotingChannels {
    pub online: Option<bool>,
    pub kiosk: Option<bool>,
    pub telephone: Option<bool>,
    pub paper: Option<bool>,
}

/// Reference to the bulletin board metadata in the database.
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub struct BulletinBoardReference {
    pub id: i64,
    pub database_name: String,
    pub is_archived: bool,
}

impl ElectionEvent {
    /// Validate the content of the election event's necessary fields, such as
    /// presentation, voting channels, status, statistics, and bulletin board
    /// reference.
    ///
    /// # Errors
    /// Returns an error if any of the JSON fields fail to deserialize into their
    /// expected types.
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
    /// Validate the content of the election's necessary fields, such as
    /// presentation, voting channels, status, and statistics.
    ///
    /// # Errors
    /// Returns an error if any of the JSON fields fail to deserialize into their
    /// expected types.
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
    /// Validate the content of the contest's necessary fields, such as
    /// presentation.
    ///
    /// # Errors
    /// Returns an error if the presentation JSON fails to deserialize.
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
    /// Validate the content of the candidate's necessary fields, such as
    /// presentation.
    ///
    /// # Errors
    /// Returns an error if the presentation JSON fails to deserialize.
    pub fn validate(&self) -> Result<()> {
        if let Some(presentation) = &self.presentation {
            serde_json::from_value::<CandidatePresentation>(
                presentation.clone(),
            )?;
        }

        Ok(())
    }
}

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

/// Status of task execution.
#[allow(missing_docs)]
#[allow(non_camel_case_types)]
pub enum TasksExecutionStatus {
    #[default]
    IN_PROGRESS,
    SUCCESS,
    FAILED,
    CANCELLED,
}

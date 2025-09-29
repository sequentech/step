// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

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
use serde_json::{from_value, Value};
use std::default::Default;
use std::ops::Deref;
use strum_macros::{Display, EnumString};

#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub struct VotingChannels {
    pub online: Option<bool>,
    pub kiosk: Option<bool>,
    pub telephone: Option<bool>,
    pub paper: Option<bool>,
}

#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub struct BulletinBoardReference {
    pub id: i64,
    pub database_name: String,
    pub is_archived: bool,
}

impl ElectionEvent {
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

    pub fn get_weighted_voting_policy(&self) -> WeightedVotingPolicy {
        let event_presentation: Option<Value> = self.presentation.clone();
        let Some(presentation) = event_presentation else {
            return WeightedVotingPolicy::default();
        };

        let policy = presentation
            .get("weighted_voting_policy")
            .cloned()
            .unwrap_or(Value::Null);

        from_value::<WeightedVotingPolicy>(policy)
            .unwrap_or(WeightedVotingPolicy::default())
    }
}

impl Election {
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
    PartialEq,
    Eq,
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
)]
pub struct Weight(Option<u64>);

impl Default for Weight {
    fn default() -> Self {
        Self { 0: Some(1) } // default weight is 1
    }
}

impl Deref for Weight {
    type Target = Option<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(
    PartialEq,
    Eq,
    Debug,
    Clone,
    Serialize,
    Deserialize,
    BorshSerialize,
    BorshDeserialize,
    Default,
)]
pub struct AreaAnnotations {
    pub weight: Weight,
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
pub enum TasksExecutionStatus {
    #[default]
    IN_PROGRESS,
    SUCCESS,
    FAILED,
    CANCELLED,
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
pub enum WeightedVotingPolicy {
    #[default]
    #[serde(rename = "disabled-weighted-voting")]
    DISABLED_WEIGHTED_VOTING,
    #[serde(rename = "areas-weighted-voting")]
    AREAS_WEIGHTED_VOTING,
}

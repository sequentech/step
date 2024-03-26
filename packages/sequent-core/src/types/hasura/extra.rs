use super::core::{Candidate, Contest, Election, ElectionEvent};
use crate::ballot::{
    CandidatePresentation, ContestPresentation, ElectionDates,
    ElectionEventPresentation, ElectionEventStatistics, ElectionEventStatus,
    ElectionPresentation, ElectionStatistics, ElectionStatus,
};
use anyhow::{anyhow, Result};
use serde::Deserialize;

#[derive(PartialEq, Eq, Debug, Clone, Deserialize)]
pub struct VotingChannels {
    pub online: Option<bool>,
    pub kiosk: Option<bool>,
    pub telephone: Option<bool>,
    pub paper: Option<bool>,
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

        if let Some(dates) = &self.dates {
            serde_json::from_value::<ElectionDates>(dates.clone())?;
        }

        if let Some(status) = &self.status {
            serde_json::from_value::<ElectionEventStatus>(status.clone())?;
        }

        if let Some(statistics) = &self.statistics {
            serde_json::from_value::<ElectionEventStatistics>(
                statistics.clone(),
            )?;
        }

        Ok(())
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

        if let Some(dates) = &self.dates {
            serde_json::from_value::<ElectionDates>(dates.clone())?;
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

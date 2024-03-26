use super::core::Contest;
use crate::ballot::ContestPresentation;
use anyhow::{anyhow, Result};

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

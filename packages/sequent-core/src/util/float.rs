// SPDX-FileCopyrightText: 2024 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::{anyhow, Result};
use ordered_float::NotNan;

// Newtype wrapper for f64
pub struct FloatWrapper(pub f64);

// Implement TryFrom for the wrapper type
impl TryFrom<FloatWrapper> for NotNan<f64> {
    type Error = anyhow::Error;

    fn try_from(wrapper: FloatWrapper) -> Result<Self> {
        NotNan::new(wrapper.0).map_err(|err| anyhow!("{:?}", err))
    }
}

// Optional: Implement From<f64> for FloatWrapper for convenience
impl From<f64> for FloatWrapper {
    fn from(value: f64) -> Self {
        FloatWrapper(value)
    }
}

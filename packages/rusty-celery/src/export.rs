// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub use crate::async_trait::async_trait;
pub use crate::serde::{Deserialize, Serialize};
pub use crate::tokio::runtime::Runtime;
pub use std::sync::Arc;
pub type Result<T> = std::result::Result<T, crate::error::CeleryError>;
pub type BeatResult<T> = std::result::Result<T, crate::error::BeatError>;

// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[cfg(feature = "reports")]
pub mod pdf;
#[cfg(feature = "reports")]
pub mod reports;
#[cfg(feature = "openid")]
pub mod openid;

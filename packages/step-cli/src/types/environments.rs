// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(clippy::upper_case_acronyms)]
/// Environment type
pub enum Environment {
    /// Local environment
    LOCAL,
    /// Development environment
    DEV,
    /// Staging environment
    STAGING,
    /// Production environment
    PRODUCTION,
}

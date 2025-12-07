// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Native-specific session implementations with tokio optimizations
pub mod session_m;
pub mod session_master;

// Re-export generic Session from protocol (for backward compatibility)
pub use crate::protocol::session::Session;
pub use session_m::SessionM;

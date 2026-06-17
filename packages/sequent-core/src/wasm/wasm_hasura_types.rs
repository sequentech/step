// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Hasura voting channel types exposed to the frontend WASM layer.
#![allow(
    missing_docs,
    reason = "TypeScript extern type mirroring IVotingChannels."
)]

use wasm_bindgen::prelude::*;

/// Which voting channels are enabled for an election event.
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct VotingChannels {
    /// Whether online voting is enabled.
    pub online: Option<bool>,
    /// Whether kiosk voting is enabled.
    pub kiosk: Option<bool>,
    /// Whether telephone voting is enabled.
    pub telephone: Option<bool>,
    /// Whether paper voting is enabled.
    pub paper: Option<bool>,
}

#[wasm_bindgen(typescript_custom_section)]
const IVOTING_CHANNELS: &'static str = r#"
interface IVotingChannels {
    online?: boolean;
    kiosk?: boolean;
    telephone?: boolean;
    paper?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IVotingChannels")]
    pub type IVotingChannels;
}

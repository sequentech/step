// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[derive(PartialEq, Eq, Debug, Clone)]
/// A representation of the voting channels available.
pub struct VotingChannels {
    /// Whether online voting is available.
    pub online: Option<bool>,
    /// Whether kiosk voting is available.
    pub kiosk: Option<bool>,
    /// Whether telephone voting is available.
    pub telephone: Option<bool>,
    /// Whether paper voting is available.
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
    /// TypeScript interface describing the voting channels available.
    #[wasm_bindgen(typescript_type = "IVotingChannels")]
    pub type IVotingChannels;
}

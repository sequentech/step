// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct VotingChannels {
    pub online: Option<bool>,
    pub kiosk: Option<bool>,
    pub telephone: Option<bool>,
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

// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const TALLY_ELECTION_STATUS: &'static str = r#"
enum ITallyElectionStatus {
    WAITING = "Waiting",
    MIXING = "Mixing",
    DECRYPTING = "Decrypting",
    COUNTING = "Counting",
    SUCCESS = "Success",
    ERROR = "Error",
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ITallyElectionStatus")]
    pub type ITallyElectionStatus;
}

#[wasm_bindgen(typescript_custom_section)]
const ITALLY_ELECTION: &'static str = r#"
interface ITallyElection {
    election_id: string;
    status: ITallyElectionStatus;
    progress: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ITallyElection")]
    pub type ITallyElection;
}
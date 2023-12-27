// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const IAUDIENCE_SELECTION: &'static str = r#"
enum IAudienceSelection {
    ALL_USERS = "ALL_USERS",
    NOT_VOTED = "NOT_VOTED",
    VOTED = "VOTED",
    SELECTED = "SELECTED"
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IAudienceSelection")]
    pub type IAudienceSelection;
}

#[wasm_bindgen(typescript_custom_section)]
const ICOMMUNICATION_TYPE: &'static str = r#"
enum ICommunicationType {
    CREDENTIALS = "CREDENTIALS",
    BALLOT_RECEIPT = "BALLOT_RECEIPT",
    PARTICIPATION_REPORT = "PARTICIPATION_REPORT",
    ELECTORAL_RESULTS = "ELECTORAL_RESULTS",
    OTP = "OTP"
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ICommunicationType")]
    pub type ICommunicationType;
}

#[wasm_bindgen(typescript_custom_section)]
const ICOMMUNICATION_METHOD: &'static str = r#"
enum ICommunicationMethod {
    EMAIL = "EMAIL",
    SMS = "SMS"
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ICommunicationMethod")]
    pub type ICommunicationMethod;
}

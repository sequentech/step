// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const IINVALID_PLAINTEXT_ERROR_TYPE: &'static str = r#"
enum IInvalidPlaintextErrorType {
    Explicit = "Explicit",
    Implicit = "Implicit",
    EncodingError = "EncodingError"
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IInvalidPlaintextErrorType")]
    pub type IInvalidPlaintextErrorType;
}

#[wasm_bindgen(typescript_custom_section)]
const IINVALID_PLAINTEXT_ERROR: &'static str = r#"
interface IInvalidPlaintextError {
    error_type: IInvalidPlaintextErrorType;
    candidate_id?: string;
    message?: string;
    message_map: Map<string, string>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IInvalidPlaintextError")]
    pub type IInvalidPlaintextError;
}

#[wasm_bindgen(typescript_custom_section)]
const IDECODED_VOTE_CONTEST: &'static str = r#"
interface IDecodedVoteContest {
    contest_id: string;
    is_explicit_invalid: boolean;
    invalid_errors: Array<IInvalidPlaintextError>;
    invalid_alerts: Array<IInvalidPlaintextError>;
    choices: Array<IDecodedVoteChoice>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IDecodedVoteContest")]
    pub type IDecodedVoteContest;
}

#[wasm_bindgen(typescript_custom_section)]
const IDECODED_VOTE_CHOICE: &'static str = r#"
interface IDecodedVoteChoice {
    id: string;
    selected: number;
    write_in_text?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IDecodedVoteChoice")]
    pub type IDecodedVoteChoice;
}

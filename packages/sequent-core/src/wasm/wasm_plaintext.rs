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
    answer_id?: number;
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
const IDECODED_VOTE_QUESTION: &'static str = r#"
interface IDecodedVoteQuestion {
    is_explicit_invalid: boolean;
    invalid_errors: Array<IInvalidPlaintextError>;
    choices: Array<IDecodedVoteChoice>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IDecodedVoteQuestion")]
    pub type IDecodedVoteQuestion;
}

#[wasm_bindgen(typescript_custom_section)]
const IDECODED_VOTE_CHOICE: &'static str = r#"
interface IDecodedVoteChoice {
    id: number;
    selected: number;
    writein_text?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IDecodedVoteChoice")]
    pub type IDecodedVoteChoice;
}

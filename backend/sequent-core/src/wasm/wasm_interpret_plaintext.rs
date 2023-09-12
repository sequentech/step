// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const QUESTION_STATE: &'static str = r#"
enum IQuestionStateEnum {
    ElectionChooserScreen = "ElectionChooserScreen",
    ReceivingElection = "ReceivingElection",
    ErrorScreen = "ErrorScreen",
    HelpScreen = "HelpScreen",
    StartScreen = "StartScreen",
    MultiQuestion = "MultiQuestion",
    PairwiseBeta = "PairwiseBeta",
    DraftsElectionScreen = "DraftsElectionScreen",
    AuditBallotScreen = "AuditBallotScreen",
    PcandidatesElectionScreen = "PcandidatesElectionScreen",
    TwoQuestionsConditionalScreen = "TwoQuestionsConditionalScreen",
    SimultaneousQuestionsScreen = "SimultaneousQuestionsScreen",
    ConditionalAccordionScreen = "ConditionalAccordionScreen",
    EncryptingBallotScreen = "EncryptingBallotScreen",
    CastOrCancelScreen = "CastOrCancelScreen",
    ReviewScreen = "ReviewScreen",
    CastingBallotScreen = "CastingBallotScreen",
    SuccessScreen = "SuccessScreen",
    ShowPdf = "ShowPdf"
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IQuestionStateEnum")]
    pub type IQuestionStateEnum;
}

#[wasm_bindgen(typescript_custom_section)]
const IANSWER_PROPERTIES: &'static str = r#"
interface IAnswerProperties {
    points?: number;
    write_in?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IAnswerProperties")]
    pub type IAnswerProperties;
}

#[wasm_bindgen(typescript_custom_section)]
const IQUESTION_LAYOUT_PROPERTIES: &'static str = r#"
interface IQuestionLayoutProperties {
    state: string;
    sorted: boolean;
    ordered: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IQuestionLayoutProperties")]
    pub type IQuestionLayoutProperties;
}

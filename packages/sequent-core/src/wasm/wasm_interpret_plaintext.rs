// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const CONTEST_STATE: &'static str = r#"
enum IContestStateEnum {
    ElectionChooserScreen = "ElectionChooserScreen",
    ReceivingElection = "ReceivingElection",
    ErrorScreen = "ErrorScreen",
    HelpScreen = "HelpScreen",
    StartScreen = "StartScreen",
    MultiContest = "MultiContest",
    PairwiseBeta = "PairwiseBeta",
    DraftsElectionScreen = "DraftsElectionScreen",
    AuditBallotScreen = "AuditBallotScreen",
    PcandidatesElectionScreen = "PcandidatesElectionScreen",
    TwoContestsConditionalScreen = "TwoContestsConditionalScreen",
    SimultaneousContestsScreen = "SimultaneousContestsScreen",
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
    #[wasm_bindgen(typescript_type = "IContestStateEnum")]
    pub type IContestStateEnum;
}

#[wasm_bindgen(typescript_custom_section)]
const IANSWER_PROPERTIES: &'static str = r#"
interface ICandidateProperties {
    points?: number;
    write_in?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ICandidateProperties")]
    pub type ICandidateProperties;
}

#[wasm_bindgen(typescript_custom_section)]
const ICONTEST_LAYOUT_PROPERTIES: &'static str = r#"
interface IContestLayoutProperties {
    state: string;
    sorted: boolean;
    ordered: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IContestLayoutProperties")]
    pub type IContestLayoutProperties;
}

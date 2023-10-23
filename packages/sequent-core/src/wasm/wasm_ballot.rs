// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const IPUBLIC_KEY_CONFIG: &'static str = r#"
interface IPublicKeyConfig {
    public_key: string;
    is_demo: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IPublicKeyConfig")]
    pub type IPublicKeyConfig;
}

#[wasm_bindgen(typescript_custom_section)]
const ICANDIDATE_URL: &'static str = r#"
interface ICandidateUrl {
    url: string;
    kind?: string;
    title?: string;
    is_image: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ICandidateUrl")]
    pub type ICandidateUrl;
}

#[wasm_bindgen(typescript_custom_section)]
const ICANDIDATE_PRESENTATION: &'static str = r#"
interface ICandidatePresentation {
    is_explicit_invalid: boolean;
    is_category_list: boolean;
    is_write_in: boolean;
    sort_order?: number;
    urls?: Array<ICandidateUrl>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ICandidatePresentation")]
    pub type ICandidatePresentation;
}

#[wasm_bindgen(typescript_custom_section)]
const ICANDIDATE: &'static str = r#"
interface ICandidate {
    id: string;
    tenant_id: string;
    election_event_id: string;
    election_id: string;
    contest_id: string;
    name?: string;
    description?: string;
    candidate_type?: string;
    presentation?: ICandidatePresentation;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ICandidate")]
    pub type ICandidate;
}

#[wasm_bindgen(typescript_custom_section)]
const ICONTEST_PRESENTATION: &'static str = r#"
interface IContestPresentation {
    allow_writeins: boolean;
    base32_writeins: boolean;
    invalid_vote_policy: string;
    cumulative_number_of_checkboxes?: number;
    shuffle_categories: boolean;
    shuffle_all_options: boolean;
    shuffle_category_list?: Array<string>;
    show_points: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IContestPresentation")]
    pub type IContestPresentation;
}

#[wasm_bindgen(typescript_custom_section)]
const ICONTEST: &'static str = r#"
interface IContest {
    id: string;
    tenant_id: string;
    election_event_id: string;
    election_id: string;
    name?: string;
    description?: string;
    max_votes: number;
    min_votes: number;
    voting_type?: string;
    counting_algorithm?: string;
    is_encrypted: boolean;
    candidates: Array<ICandidate>;
    presentation?: IContestPresentation;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IContest")]
    pub type IContest;
}

#[wasm_bindgen(typescript_custom_section)]
const IELECTION_EVENT_STATUS: &'static str = r#"
interface IElectionEventStatus {
    config_created?: boolean;
    stopped?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IElectionEventStatus")]
    pub type IElectionEventStatus;
}

#[wasm_bindgen(typescript_custom_section)]
const IVOTING_STATUS: &'static str = r#"
enum IVotingStatus {
    NOT_STARTED = "NOT_STARTED",
    OPEN = "OPEN",
    PAUSED = "PAUSED",
    CLOSED = "CLOSED",
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IVotingStatus")]
    pub type IVotingStatus;
}

#[wasm_bindgen(typescript_custom_section)]
const IBALLOT_STYLE: &'static str = r#"
interface IBallotStyle {
    id: string;
    tenant_id: string;
    election_event_id: string;
    election_id: string;
    description?: string;
    public_key?: IPublicKeyConfig;
    area_id: string;
    status?: IElectionStatus;
    contests: Array<IContest>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IBallotStyle")]
    pub type IBallotStyle;
}

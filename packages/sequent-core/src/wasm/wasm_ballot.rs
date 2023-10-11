// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use wasm_bindgen::prelude::*;

#[wasm_bindgen(typescript_custom_section)]
const ITRUSTEE_KEY_STATE: &'static str = r#"
interface ITrusteeKeyState {
    id: string;
    state: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "ITrusteeKeyState")]
    pub type ITrusteeKeyState;
}

#[wasm_bindgen(typescript_custom_section)]
const IMIXING_CATEGORY_SEGMENTATION: &'static str = r#"
interface IMixingCategorySegmentation {
    categoryName: string;
    categories: Array<string>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IMixingCategorySegmentation")]
    pub type IMixingCategorySegmentation;
}

#[wasm_bindgen(typescript_custom_section)]
const ISHARE_TEXT_ITEM: &'static str = r#"
interface IShareTextItem {
    network: string;
    button_text: string;
    social_message: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IShareTextItem")]
    pub type IShareTextItem;
}

#[wasm_bindgen(typescript_custom_section)]
const IURL: &'static str = r#"
interface IUrl {
    title: string;
    url: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IUrl")]
    pub type IUrl;
}

#[wasm_bindgen(typescript_custom_section)]
const IELECTION_EXTRA: &'static str = r#"
interface IElectionExtra {
    allow_voting_end_graceful_period?: boolean;
    start_screen__skip?: boolean;
    booth_log_out__disable?: boolean;
    disable__demo_voting_booth?: boolean;
    disable__public_home?: boolean;
    disable_voting_booth_audit_ballot?: boolean;
    disable__election_chooser_screen?: boolean;
    success_screen__hide_ballot_tracker?: boolean;
    success_screen__hide_qr_code?: boolean;
    success_screen__hide_download_ballot_ticket?: boolean;
    success_screen__redirect__url?: string;
    success_screen__redirect_to_login?: boolean;
    success_screen__redirect_to_login__text?: string;
    success_screen__redirect_to_login__auto_seconds?: number;
    success_screen__ballot_ticket__logo_url?: string;
    success_screen__ballot_ticket__logo_header?: string;
    success_screen__ballot_ticket__logo_subheader?: string;
    success_screen__ballot_ticket__h3?: string;
    success_screen__ballot_ticket__h4?: string;
    public_title?: string;
    review_screen__split_cast_edit?: boolean;
    show_skip_question_button?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IElectionExtra")]
    pub type IElectionExtra;
}

#[wasm_bindgen(typescript_custom_section)]
const IQUESTION_CONDITION: &'static str = r#"
interface IQuestionCondition {
    question_id: numner;
    answer_id: number;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IQuestionCondition")]
    pub type IQuestionCondition;
}

#[wasm_bindgen(typescript_custom_section)]
const ICONDITIONAL_QUESTION: &'static str = r#"
interface IConditionalQuestion {
    question_id: numner;
    when_any: Array<IQuestionCondition>;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IConditionalQuestion")]
    pub type IConditionalQuestion;
}

#[wasm_bindgen(typescript_custom_section)]
const IELECTION_PRESENTATION: &'static str = r#"
interface IElectionPresentation {
    share_text?: Array<IShareTextItem>;
    theme: string;
    urls: Array<IUrl>;
    theme_css: string;
    extra_options?: IElectionExtra;
    show_login_link_on_home?: boolean;
    election_board_ceremony?: boolean;
    conditional_questions?: Array<IConditionalQuestion>;
    pdf_url?: IUrl;
    anchor_continue_btn_to_bottom?: boolean;
    i18n_override?: { [key: string]: { [key: string]: string } };
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IElectionPresentation")]
    pub type IElectionPresentation;
}

#[wasm_bindgen(typescript_custom_section)]
const IQUESTION_EXTRA: &'static str = r#"
interface IQuestionExtra {
    group?: string;
    next_button?: string;
    shuffled_categories?: string;
    shuffling_policy?: string;
    ballot_parity_criteria?: string;
    restrict_choices_by_tag__name?: string;
    restrict_choices_by_tag__max?: string;
    restrict_choices_by_tag__max_error_msg?: string;
    accordion_folding_policy?: string;
    restrict_choices_by_no_tag__max?: string;
    force_allow_blank_vote?: string;
    recommended_preset__tag?: string;
    recommended_preset__title?: string;
    recommended_preset__accept_text?: string;
    recommended_preset__deny_text?: string;
    shuffle_categories?: boolean;
    shuffle_all_options?: boolean;
    shuffle_category_list?: Array<string>;
    show_points?: boolean;
    default_selected_option_ids?: Array<number>;
    select_categories_1click?: boolean;
    answer_columns_size?: i64;
    answer_group_columns_size?: i64;
    select_all_category_clicks?: i64;
    enable_panachage?: boolean;
    cumulative_number_of_checkboxes?: number; 
    enable_checkable_lists?: string;
    allow_writeins?: boolean;
    invalid_vote_policy?: string;
    review_screen__show_question_description?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IQuestionExtra")]
    pub type IQuestionExtra;
}

#[wasm_bindgen(typescript_custom_section)]
const IANSWER: &'static str = r#"
interface IAnswer {
    id: string;
    category: string;
    details: string;
    sort_order: number;
    urls: Array<IUrl>;
    text: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IAnswer")]
    pub type IAnswer;
}

#[wasm_bindgen(typescript_custom_section)]
const IQUESTION: &'static str = r#"
interface IQuestion {
    id: string;
    description: string;
    layout: string;
    max: number;
    min: number;
    num_winners: number;
    title: string;
    tally_type: string;
    answer_total_votes_percentage: string;
    answers: Array<IAnswer>;
    extra_options?: IQuestionExtra;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IQuestion")]
    pub type IQuestion;
}

#[wasm_bindgen(typescript_custom_section)]
const IELECTION_CONFIG: &'static str = r#"
interface IElectionConfig {
    id: string;
    layout: string;
    director: string;
    authorities: Array<string>;
    title: string;
    description: string;
    questions: Array<IQuestion>;
    start_date?: string;
    end_date?: string;
    presentation: IElectionPresentation;
    extra_data?: string;
    tallyPipesConfig?: string;
    ballotBoxesResultsConfig?: string;
    virtual: boolean;
    tally_allowed: boolean;
    publicCandidates: boolean;
    segmentedMixing?: boolean;
    virtualSubelections?: Array<number>;
    mixingCategorySegmentation?: IMixingCategorySegmentation;
    logo_url?: string;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IElectionConfig")]
    pub type IElectionConfig;
}

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
const IELECTION_DTO: &'static str = r#"
interface IBallotStyle {
    id: string;
    configuration: IElectionConfig;
    state: string;
    startDate?: string;
    endDate?: string;
    public_key?: IPublicKeyConfig;
    tallyPipesConfig?: string;
    ballotBoxesResultsConfig?: string;
    results?: string;
    resultsUpdated?: string;
    virtual: boolean;
    tallyAllowed: boolean;
    publicCandidates: boolean;
    logo_url?: string;
    trusteeKeysState: Array<ITrusteeKeyState>;
    segmentedMixing?: boolean;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IBallotStyle")]
    pub type IBallotStyle;
}

#[wasm_bindgen(typescript_custom_section)]
const IELECTION_PAYLOAD: &'static str = r#"
interface IElectionPayload {
    date: string;
    payload: IBallotStyle;
}
"#;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "IElectionPayload")]
    pub type IElectionPayload;
}

// SPDX-FileCopyrightText: 2022 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#![allow(non_snake_case)]
#![allow(dead_code)]
use crate::hasura_types::Uuid;
use borsh::{BorshDeserialize, BorshSerialize};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use strand::context::Ctx;
use strand::elgamal::Ciphertext;
use strand::zkp::Schnorr;

pub const TYPES_VERSION: u32 = 0;

/* -> Ciphertext<C: Ctx>
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct BallotChoice {
    pub alpha: String, // gr
    pub beta: String,  // mhr
}
*/
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct ReplicationChoice<C: Ctx> {
    pub ciphertext: Ciphertext<C>,
    pub plaintext: C::P,
    pub randomness: C::X,
}

/* -> Schnorr<C>
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Debug, Clone)]
pub struct CyphertextProof {
    pub challenge: String,
    pub commitment: String,
    pub response: String,
}
*/

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct TrusteeKeyState {
    pub id: String,
    pub state: String,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct MixingCategorySegmentation {
    pub categoryName: String,
    pub categories: Vec<String>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ShareTextItem {
    pub network: String,
    pub button_text: String,
    pub social_message: String,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct Url {
    pub title: String,
    pub url: String,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ElectionExtra {
    pub allow_voting_end_graceful_period: Option<bool>,
    pub start_screen__skip: Option<bool>,
    pub booth_log_out__disable: Option<bool>,
    pub disable__demo_voting_booth: Option<bool>,
    pub disable__public_home: Option<bool>,
    pub disable_voting_booth_audit_ballot: Option<bool>,
    pub disable__election_chooser_screen: Option<bool>,
    pub success_screen__hide_ballot_tracker: Option<bool>,
    pub success_screen__hide_qr_code: Option<bool>,
    pub success_screen__hide_download_ballot_ticket: Option<bool>,
    pub success_screen__redirect__url: Option<String>,
    pub success_screen__redirect_to_login: Option<bool>,
    pub success_screen__redirect_to_login__text: Option<String>,
    pub success_screen__redirect_to_login__auto_seconds: Option<i64>,
    pub success_screen__ballot_ticket__logo_url: Option<String>,
    pub success_screen__ballot_ticket__logo_header: Option<String>,
    pub success_screen__ballot_ticket__logo_subheader: Option<String>,
    pub success_screen__ballot_ticket__h3: Option<String>,
    pub success_screen__ballot_ticket__h4: Option<String>,
    pub public_title: Option<String>,
    pub review_screen__split_cast_edit: Option<bool>,
    pub show_skip_question_button: Option<bool>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct QuestionCondition {
    pub question_id: i64,
    pub answer_id: i64,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ConditionalQuestion {
    pub question_id: i64,
    pub when_any: Vec<QuestionCondition>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ElectionPresentation {
    pub share_text: Option<Vec<ShareTextItem>>,
    pub theme: String,
    pub urls: Vec<Url>,
    pub theme_css: String,
    pub extra_options: Option<ElectionExtra>,
    pub show_login_link_on_home: Option<bool>,
    pub election_board_ceremony: Option<bool>, // default = false
    pub conditional_questions: Option<Vec<ConditionalQuestion>>,
    pub pdf_url: Option<Url>,
    pub anchor_continue_btn_to_bottom: Option<bool>,

    // Override translations for languages. Example:
    // {"en": {"avRegistration.forgotPassword": "Whatever"}}
    pub i18n_override: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct QuestionExtra {
    pub group: Option<String>,
    pub next_button: Option<String>,
    pub shuffled_categories: Option<String>,
    pub shuffling_policy: Option<String>,
    pub ballot_parity_criteria: Option<String>,
    pub restrict_choices_by_tag__name: Option<String>,
    pub restrict_choices_by_tag__max: Option<String>,
    pub restrict_choices_by_tag__max_error_msg: Option<String>,
    pub accordion_folding_policy: Option<String>,
    pub restrict_choices_by_no_tag__max: Option<String>,
    pub force_allow_blank_vote: Option<String>,
    pub recommended_preset__tag: Option<String>,
    pub recommended_preset__title: Option<String>,
    pub recommended_preset__accept_text: Option<String>,
    pub recommended_preset__deny_text: Option<String>,
    pub shuffle_categories: Option<bool>,
    pub shuffle_all_options: Option<bool>,
    pub shuffle_category_list: Option<Vec<String>>,
    pub show_points: Option<bool>,
    pub default_selected_option_ids: Option<Vec<i64>>,
    pub select_categories_1click: Option<bool>,
    pub answer_columns_size: Option<i64>,
    pub answer_group_columns_size: Option<i64>,
    pub select_all_category_clicks: Option<i64>,
    pub enable_panachage: Option<bool>, // default = true
    pub cumulative_number_of_checkboxes: Option<u64>, // default = 1
    pub enable_checkable_lists: Option<String>, // default = "disabled"
    pub allow_writeins: Option<bool>,   // default = false
    pub invalid_vote_policy: Option<String>, /* allowed, warn, not-allowed, warn-invalid-implicit-and-explicit */
    pub review_screen__show_question_description: Option<bool>, /* default =
                                              * false */
    pub base32_writeins: Option<bool>,
}

impl QuestionExtra {
    pub fn new() -> QuestionExtra {
        QuestionExtra {
            group: None,
            next_button: None,
            shuffled_categories: None,
            shuffling_policy: None,
            ballot_parity_criteria: None,
            restrict_choices_by_tag__name: None,
            restrict_choices_by_tag__max: None,
            restrict_choices_by_tag__max_error_msg: None,
            accordion_folding_policy: None,
            restrict_choices_by_no_tag__max: None,
            force_allow_blank_vote: None,
            recommended_preset__tag: None,
            recommended_preset__title: None,
            recommended_preset__accept_text: None,
            recommended_preset__deny_text: None,
            shuffle_categories: None,
            shuffle_all_options: None,
            shuffle_category_list: None,
            show_points: None,
            default_selected_option_ids: None,
            select_categories_1click: None,
            answer_columns_size: None,
            answer_group_columns_size: None,
            select_all_category_clicks: None,
            enable_panachage: None,
            cumulative_number_of_checkboxes: None,
            enable_checkable_lists: None,
            allow_writeins: None,
            invalid_vote_policy: None,
            review_screen__show_question_description: None,
            base32_writeins: None,
        }
    }
}

impl Default for QuestionExtra {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct Answer {
    pub id: Uuid,
    pub category: String,
    pub details: String,
    pub sort_order: i64,
    pub urls: Vec<Url>,
    pub text: String,
}

impl Answer {
    pub fn is_explicit_invalid(&self) -> bool {
        self.urls
            .iter()
            .any(|url| url.title == "invalidVoteFlag" && url.url == "true")
    }
    pub fn is_write_in(&self) -> bool {
        self.urls
            .iter()
            .any(|url| url.title == "isWriteIn" && url.url == "true")
    }
    pub fn set_is_write_in(&mut self, is_write_in: bool) {
        if is_write_in == self.is_write_in() {
            return;
        }
        if is_write_in {
            self.urls.push(Url {
                title: "isWriteIn".to_string(),
                url: "true".to_string(),
            })
        } else {
            self.urls = self
                .urls
                .clone()
                .into_iter()
                .filter(|url| {
                    !(url.title == "invalidVoteFlag" && url.url == "true")
                })
                .collect();
        }
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct Question {
    pub id: Uuid,
    pub description: String,
    pub layout: String,
    pub max: i64,
    pub min: i64,
    pub num_winners: i64,
    pub title: String,
    pub tally_type: String,
    pub answer_total_votes_percentage: String,
    pub answers: Vec<Answer>,
    pub extra_options: Option<QuestionExtra>,
}

impl Question {
    pub fn allow_writeins(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| options.allow_writeins.unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn base32_writeins(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| options.base32_writeins.unwrap_or(false))
            .unwrap_or(false)
    }

    pub fn allow_explicit_invalid(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| {
                vec![
                    "allowed".to_string(),
                    "warn".to_string(),
                    "warn-invalid-implicit-and-explicit".to_string(),
                ]
                .contains(
                    &options
                        .invalid_vote_policy
                        .clone()
                        .unwrap_or_else(|| "not-allowed".to_string()),
                )
            })
            .unwrap_or(false)
    }

    pub fn cumulative_number_of_checkboxes(&self) -> u64 {
        self.extra_options
            .as_ref()
            .map(|options| options.cumulative_number_of_checkboxes.unwrap_or(1))
            .unwrap_or(1)
    }

    pub fn show_points(&self) -> bool {
        self.extra_options
            .as_ref()
            .map(|options| options.show_points.unwrap_or(false))
            .unwrap_or(false)
    }
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ElectionConfig {
    pub id: Uuid,
    pub layout: String,
    pub director: String,
    pub authorities: Vec<String>,
    pub title: String,
    pub description: String,
    pub questions: Vec<Question>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub presentation: ElectionPresentation,
    pub extra_data: Option<String>,
    pub tallyPipesConfig: Option<String>,
    pub ballotBoxesResultsConfig: Option<String>,
    pub r#virtual: bool,
    pub tally_allowed: bool,
    pub publicCandidates: bool,
    pub segmentedMixing: Option<bool>,
    pub virtualSubelections: Option<Vec<i64>>,
    pub mixingCategorySegmentation: Option<MixingCategorySegmentation>,
    pub logo_url: Option<String>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct Pk {
    pub q: String,
    pub p: String,
    pub y: String,
    pub g: String,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct PublicKeyConfig {
    pub public_key: String,
    pub is_demo: bool,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ElectionDTO {
    pub id: Uuid,
    pub configuration: ElectionConfig,
    pub state: String,
    pub startDate: Option<String>,
    pub endDate: Option<String>,
    pub pks: Option<String>,
    pub public_key: Option<PublicKeyConfig>,
    pub tallyPipesConfig: Option<String>,
    pub ballotBoxesResultsConfig: Option<String>,
    pub results: Option<String>,
    pub resultsUpdated: Option<String>,
    pub r#virtual: bool,
    pub tallyAllowed: bool,
    pub publicCandidates: bool,
    pub logo_url: Option<String>,
    pub trusteeKeysState: Vec<TrusteeKeyState>,
    pub segmentedMixing: Option<bool>,
}

#[derive(
    BorshSerialize,
    BorshDeserialize,
    Serialize,
    Deserialize,
    JsonSchema,
    PartialEq,
    Eq,
    Debug,
    Clone,
)]
pub struct ElectionPayload {
    pub date: String,
    pub payload: ElectionDTO,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct RawAuditableBallot<C: Ctx> {
    pub election_url: String,
    pub issue_date: String,
    pub choices: Vec<ReplicationChoice<C>>,
    pub proofs: Vec<Schnorr<C>>,
    pub ballot_hash: String,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct AuditableBallot<C: Ctx> {
    pub version: u32,
    pub issue_date: String,
    pub choices: Vec<ReplicationChoice<C>>,
    pub proofs: Vec<Schnorr<C>>,
    pub ballot_hash: String,
    pub config: ElectionDTO,
}

#[derive(BorshSerialize, BorshDeserialize, PartialEq, Eq, Debug, Clone)]
pub struct HashableBallot<C: Ctx> {
    pub version: u32,
    pub choices: Vec<Ciphertext<C>>,
    pub issue_date: String,
    pub proofs: Vec<Schnorr<C>>,
}

impl<C: Ctx> From<&AuditableBallot<C>> for HashableBallot<C> {
    fn from(value: &AuditableBallot<C>) -> HashableBallot<C> {
        assert!(TYPES_VERSION == value.version);
        HashableBallot {
            version: TYPES_VERSION,
            choices: value
                .choices
                .clone()
                .into_iter()
                .map(|choice| choice.ciphertext)
                .collect(),
            issue_date: value.issue_date.clone(),
            proofs: value.proofs.clone(),
        }
    }
}

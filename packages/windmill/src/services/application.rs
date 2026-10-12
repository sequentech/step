// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::approval_matrix::evaluate::{
    evaluate, Candidate, DecisionRecord, MatrixVersion, RuleInputs,
};
use super::approval_matrix::store::get_current_approval_matrix;
use super::approval_matrix::{FieldMatch, IdentityMethod};
use super::users::{lookup_users, FilterOption, ListUsersFilter};
use crate::postgres::application::get_permission_label_from_post;
use crate::postgres::area::get_elections_by_area;
use crate::postgres::area::get_event_areas;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::messaging::MessageRecord;
use crate::services::celery_app::get_celery_app;
use crate::services::database::get_hasura_pool;
use crate::services::messaging::audit::post_attempt_outcomes_or_warn;
use crate::services::messaging::dispatch::{
    deliver, record_without_channel, Delivery, Dispatcher, Recipient,
};
use crate::services::messaging::reconcile::RetryContent;
use crate::services::providers::{email_sender::EmailSender, sms_sender::SmsSender};
use crate::services::reports::template_time::insert_template_time_variables;
use crate::services::reports::template_time::NotificationTimeContext;
use crate::services::reports::utils::get_public_asset_template;
use crate::services::temp_path::PUBLIC_ASSETS_I18N_DEFAULTS;
use crate::tasks::send_template::RetryDirectory;
use crate::tasks::send_template::{first_attribute, get_variables, render_contents};
use crate::types::application::ApplicationRejectReason;
use crate::types::application::NoMatchingVoterPolicy;
use crate::{
    postgres::application::{insert_application, update_application_status},
    types::application::ApplicationStatus,
    types::application::ApplicationType,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use keycloak::types::CredentialRepresentation;
use sequent_core::ballot::{ElectionEventPresentation, I18nContent};
use sequent_core::serialization::deserialize_with_path::*;
use sequent_core::services::generate_urls::AuthAction;
use sequent_core::types::messaging::MessageReason;
use sequent_core::types::messaging::{MessageChannel, MessageContent, MessagePurpose};
use sequent_core::types::voter_channel::{voter_channel, VoterChannel};

use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::translation_scopes::template_translations;
use sequent_core::services::translations::DEFAULT_LANG;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::keycloak::{
    User, AREA_ID_ATTR_NAME, LOCALE_ATTR_NAME, MESSAGE_CHANNEL_ATTR_NAME,
    MESSAGE_CONSENT_ATTR_NAME, MESSAGE_NUMBER_ATTR_NAME, MESSENGER_ID_ATTR_NAME,
    MESSENGER_PAGE_ATTR_NAME, MOBILE_PHONE_ATTR_NAME, VERIFIED_CHANNELS_ATTR_NAME,
    VIBER_NUMBER_ATTR_NAME, WHATSAPP_NUMBER_ATTR_NAME,
};
use sequent_core::types::templates::{EmailConfig, SendTemplateBody, SmsConfig, TemplateMethod};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};
use strum_macros::Display;
use tracing::{debug, event, info, instrument, warn, Level};
use uuid::Uuid;

use sequent_core::types::templates::AudienceSelection::SELECTED;
use sequent_core::types::templates::TemplateMethod::{EMAIL, SMS};
use unicode_normalization::char::decompose_canonical;

const ID_CARD_TYPE_FIELD: &str = "sequent.read-only.id-card-type";
const FIRST_NAME_FIELD: &str = "firstName";
const FIRST_AND_MIDDLE_NAME_FIELD: &str = "firstName.middleName";
const SEVERAL_VOTERS_MATCH_REASON: &str = "Several registry voters match";

#[allow(non_camel_case_types)]
#[derive(Display, Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum ECardType {
    #[strum(serialize = "philSysID")]
    #[serde(rename = "philSysID")]
    PHILSYS_ID,
    #[strum(serialize = "seamanBook")]
    #[serde(rename = "seamanBook")]
    SEAMANS_BOOK,
    #[strum(serialize = "driversLicense")]
    #[serde(rename = "driversLicense")]
    DRIVER_LICENSE,
    #[strum(serialize = "philippinePassport")]
    #[serde(rename = "philippinePassport")]
    PHILIPPINE_PASSPORT,
    #[strum(serialize = "iBP")]
    #[serde(rename = "iBP")]
    IBP,
}

/// The identity document types enrollment knows, as the applicant data
/// names them.
pub fn id_card_types() -> Vec<String> {
    [
        ECardType::PHILIPPINE_PASSPORT,
        ECardType::PHILSYS_ID,
        ECardType::SEAMANS_BOOK,
        ECardType::DRIVER_LICENSE,
        ECardType::IBP,
    ]
    .iter()
    .map(ECardType::to_string)
    .collect()
}

/// Struct for email/sms Accepted/Rejected Communication object.
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ApplicationCommunication {
    accepted: ApplicationCommunicationChannels,
    rejected: ApplicationCommunicationChannels,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct ApplicationCommunicationChannels {
    email: EmailConfig,
    sms: SmsConfig,
}

#[instrument(skip_all, err)]
pub async fn verify_application(
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
    applicant_id: &str,
    applicant_data: &HashMap<String, String>,
    tenant_id: &str,
    election_event_id: &str,
    labels: &Option<Value>,
    annotations: &ApplicationAnnotations,
) -> Result<ApplicationVerificationResult> {
    let realm = get_event_realm(tenant_id, election_event_id);

    // Check Election Event exists
    let _event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;

    // The saved matrix names the compared fields; without one, the built-in
    // matrix compares the search attributes the enrollment flow sends.
    let matrix = get_current_approval_matrix(
        hasura_transaction,
        tenant_id,
        election_event_id,
        search_attributes_list(annotations)?,
    )
    .await?;

    // Generate a filter with applicant data
    let filter = get_filter_from_applicant_data(
        tenant_id.to_string(),
        Some(election_event_id.to_string()),
        None,
        None,
        realm,
        None,
        &matrix.matrix.compared_fields,
        applicant_data,
    )?;

    // Uses applicant data to lookup possible users
    let users = lookup_users(hasura_transaction, keycloak_transaction, filter).await?;
    debug!("Found {} users before verification", users.len());

    // Decides the enrollment from the list of found possible users
    let (result, decision) =
        automatic_verification(users.clone(), &matrix, &annotations, applicant_data)?;
    info!("Verification result: {:?}", result);

    // Set the annotations
    let annotations = ApplicationAnnotations {
        session_id: annotations.session_id.clone(),
        credentials: annotations.credentials.clone(),
        verified_by: None,
        rejection_reason: result.rejection_reason.clone(),
        rejection_message: result.rejection_message.clone(),
        unset_attributes: annotations.unset_attributes.clone(),
        search_attributes: annotations.search_attributes.clone(),
        update_attributes: annotations.update_attributes.clone(),
        identity_method: annotations.identity_method,
        mismatches: result.mismatches,
        fields_match: result.fields_match.clone(),
        manual_verify_reason: result.manual_verify_reason.clone(),
        decision: Some(decision),
        no_matching_voter_policy: annotations.no_matching_voter_policy.clone(),
    };

    let (mut permission_label, area_id) = get_permission_label_and_area_from_applicant_data(
        hasura_transaction,
        applicant_data,
        tenant_id,
        election_event_id,
    )
    .await?;
    // Add a permission label only if the embassy matches the voter in db
    if !matches!(
        result
            .fields_match
            .as_ref()
            .and_then(|value| value.get("embassy")),
        Some(true)
    ) {
        permission_label = None;
    }

    let mut final_applicant_data = applicant_data.clone();
    final_applicant_data.insert("username".to_string(), result.username.clone());

    insert_application(
        hasura_transaction,
        tenant_id,
        election_event_id,
        &area_id,
        applicant_id,
        &final_applicant_data,
        labels,
        &annotations,
        &result.application_type,
        &result.application_status,
        &permission_label,
    )
    .await?;
    // The application just stored in this transaction.
    let application_id: Uuid = hasura_transaction
        .query_one(
            r#"
            SELECT id FROM sequent_backend.applications
            WHERE tenant_id = $1 AND election_event_id = $2 AND applicant_id = $3
            ORDER BY created_at DESC, id
            LIMIT 1
            "#,
            &[
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
                &applicant_id,
            ],
        )
        .await
        .with_context(|| "Error reading the stored application")?
        .try_get("id")?;

    Ok(ApplicationVerificationResult {
        application_id: Some(application_id.to_string()),
        ..result
    })
}

#[instrument(err, skip_all)]
async fn get_permission_label_and_area_from_applicant_data(
    hasura_transaction: &Transaction<'_>,
    applicant_data: &HashMap<String, String>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(Option<String>, Option<Uuid>)> {
    // Enrollments that don't ask for a Post have no area or permission label.
    let (Some(post_name), Some(post_description)) = (
        applicant_data
            .get("country")
            .and_then(|country| country.split('/').next()),
        applicant_data.get("embassy"),
    ) else {
        return Ok((None, None));
    };

    info!("Found post: {:?}", &post_name);
    info!("Found embassy: {:?}", &post_description);

    return get_permission_label_from_post(
        hasura_transaction,
        &post_name,
        &post_description,
        tenant_id,
        election_event_id,
    )
    .await;
}

#[instrument(err, skip_all)]
fn get_filter_from_applicant_data(
    tenant_id: String,
    election_event_id: Option<String>,
    election_id: Option<String>,
    area_id: Option<String>,
    realm: String,
    search: Option<String>,
    compared_fields: &[String],
    applicant_data: &HashMap<String, String>,
) -> Result<ListUsersFilter> {
    let mut first_name = None;
    let mut last_name = None;
    let mut username = None;
    let mut email = None;
    let mut attributes_map = HashMap::new();

    for attribute in compared_fields {
        match attribute.as_str() {
            "firstName" => {
                first_name = applicant_data
                    .get("firstName")
                    .and_then(|value| Some(FilterOption::IsEqualNormalized(value.to_string())));
            }
            "lastName" => {
                last_name = applicant_data
                    .get("lastName")
                    .and_then(|value| Some(FilterOption::IsEqualNormalized(value.to_string())));
            }
            "username" => {
                username = applicant_data
                    .get("username")
                    .and_then(|value| Some(FilterOption::IsEqualNormalized(value.to_string())));
            }
            "email" => {
                email = applicant_data
                    .get("email")
                    .and_then(|value| Some(FilterOption::IsEqualNormalized(value.to_string())));
            }
            "embassy" => {
                // Ignore embassy to speed up user lookup
            }
            _ => {
                let value = applicant_data
                    .get(attribute.as_str())
                    .cloned()
                    // Return an empty string if a value is missing from the applicant data.
                    .unwrap_or("".to_string());

                attributes_map.insert(attribute.to_string(), value);
            }
        }
    }

    let attributes = if attributes_map.is_empty() {
        None
    } else {
        Some(attributes_map)
    };

    Ok(ListUsersFilter {
        tenant_id,
        election_event_id,
        election_id,
        area_id,
        realm,
        search,
        first_name,
        last_name,
        username,
        email,
        limit: None,
        offset: None,
        user_ids: None,
        attributes,
        email_verified: None,
        enabled: Some(true),
        sort: None,
        has_voted: None,
        authorized_to_election_alias: None,
    })
}

#[instrument(skip_all)]
fn build_manual_verify_reason(fields_match: HashMap<String, bool>) -> String {
    let mismatch_fields = fields_match
        .iter()
        .filter(|(_, &value)| !value)
        .map(|(key, _)| match key.as_str() {
            "firstName" => "First Name",
            "middleName" => "Middle Name",
            "lastName" => "Last Name",
            "embassy" => "Post",
            "dateOfBirth" => "Date Of Birth",
            "firstName.middleName" => "First Name + Middle Name",
            _ => key,
        })
        .collect::<Vec<&str>>()
        .join(", ");

    format!("Mismatch at {}", mismatch_fields)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationAnnotations {
    session_id: Option<String>,
    credentials: Option<Value>,
    verified_by: Option<String>,
    rejection_reason: Option<ApplicationRejectReason>,
    rejection_message: Option<String>,
    #[serde(rename = "unset-attributes")]
    unset_attributes: Option<String>,
    #[serde(rename = "search-attributes")]
    search_attributes: Option<String>,
    #[serde(rename = "update-attributes")]
    update_attributes: Option<String>,
    /// How the enrollment flow established the applicant's identity, when
    /// it reports it.
    #[serde(
        rename = "identity-method",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    identity_method: Option<IdentityMethod>,
    mismatches: Option<usize>,
    fields_match: Option<HashMap<String, bool>>,
    manual_verify_reason: Option<String>,
    /// The approval matrix version, rule and inputs that decided the
    /// application.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    decision: Option<DecisionRecord>,
    #[serde(
        rename = "no-matching-voter-policy",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    no_matching_voter_policy: Option<NoMatchingVoterPolicy>,
}

/// The fields the enrollment flow asks to compare with the registry.
fn search_attributes_list(annotations: &ApplicationAnnotations) -> Result<Vec<String>> {
    let search_attributes = annotations.search_attributes.as_ref().ok_or(anyhow!(
        "Error obtaining search_attributes from annotations"
    ))?;
    Ok(search_attributes
        .split(',')
        .map(|field| field.trim().to_string())
        .filter(|field| !field.is_empty())
        .collect())
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationVerificationResult {
    /// The stored application's ID, for the applicant's support
    /// reference. Set once the application is stored.
    #[serde(default)]
    pub application_id: Option<String>,
    pub user_id: Option<String>,
    pub username: String,
    pub application_status: ApplicationStatus,
    pub application_type: ApplicationType,
    pub mismatches: Option<usize>,
    pub fields_match: Option<HashMap<String, bool>>,
    pub attributes_unset: Option<HashMap<String, bool>>,
    pub rejection_reason: Option<ApplicationRejectReason>,
    pub rejection_message: Option<String>,
    pub manual_verify_reason: Option<String>,
}

/// The compared fields the matrix's rules see for one registry voter. First
/// and middle name compared together count as the first name; the middle
/// name then has no result of its own.
fn rule_fields(
    compared_fields: &[String],
    fields_match: &HashMap<String, bool>,
) -> BTreeMap<String, FieldMatch> {
    let result = |is_match: &bool| match is_match {
        true => FieldMatch::MATCHES,
        false => FieldMatch::DIFFERS,
    };
    compared_fields
        .iter()
        .filter_map(|field| {
            let is_match = fields_match.get(field).or_else(|| {
                (field == FIRST_NAME_FIELD)
                    .then(|| fields_match.get(FIRST_AND_MIDDLE_NAME_FIELD))
                    .flatten()
            })?;
            Some((field.clone(), result(is_match)))
        })
        .collect()
}

/// Decides the enrollment with the approval matrix from the registry voters
/// the lookup returned.
#[instrument(err, skip_all)]
fn automatic_verification(
    users: Vec<User>,
    matrix: &MatrixVersion,
    annotations: &ApplicationAnnotations,
    applicant_data: &HashMap<String, String>,
) -> Result<(ApplicationVerificationResult, DecisionRecord)> {
    let unset_attributes: String = annotations
        .unset_attributes
        .clone()
        .ok_or(anyhow!("Error obtaining unset_attributes from annotations"))?;
    let compared_fields = &matrix.matrix.compared_fields;
    let identity = annotations.identity_method;
    let valid_id = applicant_data.get(ID_CARD_TYPE_FIELD).cloned();

    let mut comparisons = vec![];
    for user in users {
        let (mismatches, mismatches_unset, fields_match, attributes_unset) = check_mismatches(
            &user,
            applicant_data,
            compared_fields.join(","),
            unset_attributes.clone(),
        )?;
        let candidate = Candidate {
            user_id: user.id.clone(),
            username: user.username.clone(),
            inputs: RuleInputs {
                identity,
                voter_found: true,
                already_enrolled: mismatches_unset > 0,
                valid_id: valid_id.clone(),
                fields: rule_fields(compared_fields, &fields_match),
                differing: mismatches,
            },
        };
        comparisons.push((candidate, fields_match, attributes_unset));
    }

    let candidates = comparisons
        .iter()
        .map(|(candidate, _, _)| candidate.clone())
        .collect();
    let evaluation = evaluate(matrix, identity, valid_id, candidates);
    let record = evaluation.record;

    // An identity that matches no voter of the census goes to review, if the
    // realm asks for it, instead of being rejected. The record keeps what the
    // approval matrix decided.
    let no_voter_to_review = record.decision == ApplicationStatus::REJECTED
        && record.reason == Some(ApplicationRejectReason::NO_VOTER)
        && annotations.no_matching_voter_policy == Some(NoMatchingVoterPolicy::PENDING_APPROVAL);
    let status = if no_voter_to_review {
        ApplicationStatus::PENDING
    } else {
        record.decision.clone()
    };
    info!(
        "Approval matrix version {} decided {} with rule {:?}",
        record.matrix_version, record.decision, record.rule
    );

    let compared = evaluation.compared.and_then(|compared| {
        comparisons
            .into_iter()
            .find(|(candidate, _, _)| *candidate == compared)
    });
    let (mismatches, fields_match, attributes_unset) = match compared {
        Some((candidate, fields_match, attributes_unset)) => (
            candidate.inputs.differing,
            fields_match,
            Some(attributes_unset),
        ),
        None => (
            compared_fields.len(),
            compared_fields
                .iter()
                .map(|field| (field.clone(), false))
                .collect(),
            None,
        ),
    };

    let manual_verify_reason = if record.decision == ApplicationStatus::ACCEPTED {
        None
    } else if record.accepted_candidates > 1 {
        Some(SEVERAL_VOTERS_MATCH_REASON.to_string())
    } else if record.inputs.voter_found && mismatches > 0 {
        Some(build_manual_verify_reason(fields_match.clone()))
    } else {
        None
    };

    let result = ApplicationVerificationResult {
        application_id: None,
        user_id: evaluation
            .voter
            .as_ref()
            .and_then(|voter| voter.user_id.clone()),
        username: evaluation
            .voter
            .and_then(|voter| voter.username)
            .unwrap_or_default(),
        application_type: match status {
            ApplicationStatus::PENDING => ApplicationType::MANUAL,
            ApplicationStatus::ACCEPTED | ApplicationStatus::REJECTED => ApplicationType::AUTOMATIC,
        },
        application_status: status,
        mismatches: Some(mismatches),
        fields_match: Some(fields_match),
        attributes_unset,
        rejection_reason: record.reason.clone(),
        rejection_message: None,
        manual_verify_reason,
    };

    Ok((result, record))
}

#[instrument(err)]
fn check_mismatches(
    user: &User,
    applicant_data: &HashMap<String, String>,
    fields_to_check: String,
    fields_to_check_unset: String,
) -> Result<(usize, usize, HashMap<String, bool>, HashMap<String, bool>)> {
    let mut match_result = HashMap::new();
    let mut unset_result = HashMap::new();
    let mut mismatches = 0;

    debug!(
        "Checking user with id: {:?}, fields to check: {:?}, unset to check: {:?}",
        user.id, fields_to_check, fields_to_check_unset
    );

    // Check if the card type is seamans_book or driver_license
    let card_type_flag = applicant_data
        .get(ID_CARD_TYPE_FIELD)
        .is_some_and(|card_type| {
            *card_type == ECardType::SEAMANS_BOOK.to_string()
                || *card_type == ECardType::DRIVER_LICENSE.to_string()
        });

    for field_to_check in fields_to_check.split(",") {
        let field_to_check = field_to_check.trim();

        // Special handling for firstName when card_type_flag is true
        if card_type_flag {
            if field_to_check == "middleName" {
                continue;
            } else if field_to_check == "firstName" {
                // Extract first and middle names from applicant_data
                let applicant_first_name = applicant_data
                    .get("firstName")
                    .map(|value| value.to_string().to_lowercase());
                let applicant_middle_name = applicant_data
                    .get("middleName")
                    .map(|value| value.to_string().to_lowercase());
                let applicant_combined = match (applicant_first_name, applicant_middle_name) {
                    (Some(first), Some(middle)) => {
                        Some(format!("{} {}", first, middle).trim().to_string())
                    }
                    (Some(first), None) => Some(first),
                    _ => None,
                };

                // Combine firstName and middleName for the user
                let user_combined = match (&user.first_name, &user.attributes) {
                    (Some(first_name), Some(attributes)) => {
                        let middle_name = attributes
                            .get("middleName")
                            .and_then(|values| values.first())
                            .map(|value| value.to_lowercase());
                        Some(
                            format!(
                                "{} {}",
                                first_name.to_lowercase(),
                                middle_name.unwrap_or_default()
                            )
                            .trim()
                            .to_string(),
                        )
                    }
                    (Some(first_name), None) => Some(first_name.to_lowercase()),
                    _ => None,
                };

                let is_match = is_fuzzy_match(applicant_combined, user_combined);
                match_result.insert("firstName.middleName".to_string(), is_match);

                if !is_match {
                    mismatches += 1;
                }
                continue;
            }
        }

        // Extract field from applicant_data
        let applicant_field_value = applicant_data
            .get(field_to_check)
            .map(|value| value.to_string().to_lowercase());

        // Extract field from user
        let user_field_value = match field_to_check {
            "firstName" => &user.first_name,
            "lastName" => &user.last_name,
            "username" => &user.username,
            "email" => &user.email,
            _ => &user
                .attributes
                .as_ref()
                .and_then(|attributes| attributes.get(field_to_check))
                .and_then(|values| values.first())
                .map(|value| value.to_string()),
        };

        let user_field_value = user_field_value.clone().map(|value| value.to_lowercase());
        let is_match = is_fuzzy_match(applicant_field_value, user_field_value);

        // Check match
        match_result.insert(field_to_check.to_string(), is_match);

        if !is_match {
            mismatches += 1;
        }
    }

    let mut unset_mismatches = 0;

    for fields_to_check_unset in fields_to_check_unset.split(",") {
        let field_to_check = fields_to_check_unset.trim();

        // Extract field from user
        let user_field_value = match field_to_check {
            "firstName" => &user.first_name,
            "lastName" => &user.last_name,
            "username" => &user.username,
            "email" => &user.email,
            _ => &user
                .attributes
                .as_ref()
                .and_then(|attributes| attributes.get(field_to_check))
                .and_then(|values| values.first())
                .map(|value| value.to_string()),
        };

        let user_field_value = user_field_value.clone().map(|value| value.to_lowercase());
        let is_set = user_field_value.unwrap_or_default().trim().len() > 0;

        // match is true only if the field is NOT set
        unset_result.insert(field_to_check.to_string(), !is_set);
        if is_set {
            unset_mismatches += 1;
        }
    }

    debug!("Mismatches {:?}", mismatches);
    debug!("Unset Mismatches {:?}", unset_mismatches);
    debug!("Match Result {:?}", match_result);
    debug!("Unset Result {:?}", unset_result);

    Ok((mismatches, unset_mismatches, match_result, unset_result))
}

/// Get the accepted/rejected message from the internalization object in the defaults file.
#[instrument(err)]
async fn get_i18n_default_application_communication(
    lang: &str,
    app_status: ApplicationStatus,
) -> Result<ApplicationCommunicationChannels> {
    let json_data = get_public_asset_template(PUBLIC_ASSETS_I18N_DEFAULTS)
        .await
        .map_err(|e| {
            anyhow::anyhow!(format!(
                "Error to get the ApplicationCommunication default data file {e:?}"
            ))
        })?;

    let i18n_data: I18nContent<HashMap<String, Option<Value>>> = deserialize_str(&json_data)
        .map_err(|e| {
            anyhow::anyhow!(format!("Error to parse the i18n_defaults data file {e:?}"))
        })?;

    let application: ApplicationCommunication = match i18n_data
        .get(lang)
        .unwrap_or(&HashMap::new())
        .get("application")
        .unwrap_or(&None)
    {
        Some(value) => deserialize_value(value.clone()).map_err(|e| {
            anyhow::anyhow!(format!(
                "Error to parse the ApplicationCommunication default data {e:?}"
            ))
        })?,
        None => {
            return Err(anyhow::anyhow!(
                "No default application data found in language: {}",
                lang
            ))
        }
    };

    match app_status {
        ApplicationStatus::ACCEPTED => Ok(application.accepted),
        ApplicationStatus::REJECTED => Ok(application.rejected),
        _ => Err(anyhow::anyhow!("Not a valid application status")),
    }
}

/// Get the accepted/rejected message from the internalization object in presentation.
#[instrument(skip_all)]
pub async fn get_i18n_application_communication(
    presentation: ElectionEventPresentation,
    lang: &str,
    app_status: ApplicationStatus,
    communication_method: TemplateMethod,
) -> Result<ApplicationCommunicationChannels> {
    let application_channels =
        get_i18n_default_application_communication(&lang, app_status.clone()).await?;
    Ok(apply_application_overrides(
        application_channels,
        &presentation,
        lang,
        &app_status,
    ))
}

/// The event's overrides of an application message, over the defaults. The
/// Localization tab stores `scope:key`; templates read the `templates:` >
/// unprefixed > `global:` overrides.
fn apply_application_overrides(
    mut application_channels: ApplicationCommunicationChannels,
    presentation: &ElectionEventPresentation,
    lang: &str,
    app_status: &ApplicationStatus,
) -> ApplicationCommunicationChannels {
    let Some(localization_map) = presentation
        .i18n
        .as_ref()
        .and_then(|val| val.get(lang))
        .map(template_translations)
    else {
        return application_channels;
    };
    let key_prefix = format!("application.{}", app_status.to_string().to_lowercase());

    if let Some(sms_message) = localization_map
        .get(&format!("{key_prefix}.sms.message"))
        .cloned()
    {
        application_channels.sms.message = sms_message;
    };

    if let Some(email_subject) = localization_map
        .get(&format!("{key_prefix}.email.subject"))
        .cloned()
    {
        application_channels.email.subject = email_subject;
    };

    if let Some(plaintext_body) = localization_map
        .get(&format!("{key_prefix}.email.plaintext_body"))
        .cloned()
    {
        application_channels.email.plaintext_body = plaintext_body;
    };

    if let Some(html_body) = localization_map
        .get(&format!("{key_prefix}.email.html_body"))
        .cloned()
    {
        application_channels.email.html_body = Some(html_body);
    };

    application_channels
}

/// Get the accepted/rejected message if configured, otherwise the default.
#[instrument(skip(presentation), err)]
pub async fn get_application_response_communication(
    communication_method: Option<TemplateMethod>,
    app_status: ApplicationStatus,
    presentation: ElectionEventPresentation,
) -> Result<(Option<EmailConfig>, Option<SmsConfig>)> {
    // Do not retrieve data when early return is desired.
    let communication_method: TemplateMethod = match communication_method {
        Some(communication_method) => communication_method,
        None => return Ok((None, None)),
    };

    let language_conf = presentation.language_conf.clone().unwrap_or_default();
    let lang = language_conf
        .default_language_code
        .unwrap_or(DEFAULT_LANG.into());

    // Read the configured data from presentation or default to the json file.
    let appl_comm = get_i18n_application_communication(
        presentation,
        &lang,
        app_status.clone(),
        communication_method.clone(),
    )
    .await?;

    match communication_method {
        EMAIL => Ok((Some(appl_comm.email), None)),
        SMS => Ok((None, Some(appl_comm.sms))),
        _ => Ok((None, None)),
    }
}

#[instrument(skip_all, err)]
pub async fn confirm_application(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    admin_id: &str,
    admin_name: &str,
    group_names: &Vec<String>,
) -> Result<(Application, User)> {
    // Update the application to ACCEPTED
    let application = update_application_status(
        hasura_transaction,
        id,
        tenant_id,
        election_event_id,
        user_id,
        ApplicationStatus::ACCEPTED,
        ApplicationType::MANUAL,
        None,
        None,
        admin_name,
        group_names,
    )
    .await
    .map_err(|err| anyhow!("Error updating application: {}", err))?;

    // Update user attributes and credentials
    let realm = get_event_realm(tenant_id, election_event_id);

    // Obtain application annotations
    let annotations = application
        .annotations
        .clone()
        .ok_or(anyhow!("Error obtaining application annotations"))?
        .as_object()
        .ok_or(anyhow!("Error parsing application annotations"))?
        .clone();

    // Obtain application credentials
    let credentials = annotations
        .get("credentials")
        .map(|value| {
            deserialize_value::<Vec<CredentialRepresentation>>(value.clone())
                .map_err(|err| anyhow!("Error parsing application credentials: {}", err))
        })
        .transpose()?;

    // Obtain voter attributes to update
    let attributes_to_store: Vec<String> = annotations
        .get("update-attributes")
        .ok_or(anyhow!(
            "Error obtaining update-attributes from application annotations"
        ))?
        .as_str()
        .ok_or(anyhow!(
            "Error parsing update-attributes from application annotations"
        ))?
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    // Get applicant data
    let applicant_data = application
        .applicant_data
        .clone()
        .as_object()
        .ok_or(anyhow!("Error parsing application applicant data"))?
        .clone();

    // Parse applicant data to update user
    let mut attributes: HashMap<String, Vec<String>> = applicant_data
        .iter()
        .filter(|(key, _value)| attributes_to_store.contains(key))
        .filter(|(key, _value)| !VERIFIED_CHANNEL_ATTRIBUTES.contains(&key.as_str()))
        .map(|(key, value)| {
            (
                key.to_owned(),
                value
                    .to_string()
                    .split(";")
                    .map(|value| value.trim_matches('"').to_string())
                    .collect(),
            )
        })
        .collect();
    // The verified channel, whatever the form's update list says.
    attributes.extend(verified_channel_attributes(&applicant_data));

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error obtaining keycloak admin client: {}", err))?;

    let user = client
        .get_user(&realm, user_id)
        .await
        .map_err(|err| anyhow!("Error getting the user: {err}"))?;

    let email = if attributes_to_store.contains(&"email".to_string()) {
        attributes
            .remove("email")
            .and_then(|value| value.first().cloned())
    } else {
        user.email
    };

    let first_name = if attributes_to_store.contains(&"firstName".to_string()) {
        attributes
            .remove("firstName")
            .and_then(|value| value.first().cloned())
    } else {
        user.first_name
    };

    let last_name = if attributes_to_store.contains(&"lastName".to_string()) {
        attributes
            .remove("lastName")
            .and_then(|value| value.first().cloned())
    } else {
        user.last_name
    };

    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error obtaining keycloak admin client: {}", err))?;

    let user = client
        .edit_user_with_credentials(
            &realm,
            user_id,
            None,
            Some(attributes),
            email,
            first_name,
            last_name,
            None,
            credentials,
            Some(false),
        )
        .await
        .map_err(|err| anyhow!("Error updating user: {err}"))?;

    send_application_communication_response(
        hasura_transaction,
        tenant_id,
        election_event_id,
        id,
        user_id,
        admin_id,
        &user,
        ApplicationStatus::ACCEPTED,
    )
    .await
    .map_err(|err| anyhow!("Error sending communication response: {err}"))?;

    Ok((application, user))
}

#[instrument(skip(hasura_transaction), err)]
pub async fn reject_application(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    admin_id: &str,
    rejection_reason: Option<String>,
    rejection_message: Option<String>,
    admin_name: &str,
    group_names: &Vec<String>,
) -> Result<Application> {
    // Update the application to REJECTED
    let application = update_application_status(
        hasura_transaction,
        id,
        tenant_id,
        election_event_id,
        user_id,
        ApplicationStatus::REJECTED,
        ApplicationType::MANUAL,
        rejection_reason,
        rejection_message,
        admin_name,
        &group_names,
    )
    .await
    .map_err(|err| anyhow!("Error updating application: {}", err))?;

    let applicant_data: HashMap<String, String> =
        deserialize_value(application.applicant_data.clone())
            .map_err(|err| anyhow!("Error parsing application applicant data: {}", err))?;

    // The phone number and the verified channel, so the result goes to
    // the applicant's one channel.
    let user = applicant_user(&applicant_data);

    send_application_communication_response(
        hasura_transaction,
        tenant_id,
        election_event_id,
        id,
        user_id,
        admin_id,
        &user,
        ApplicationStatus::REJECTED,
    )
    .await
    .map_err(|err| anyhow!("Error sending communication response: {err}"))?;

    Ok(application)
}

/// The applicant data keys that carry the verified channel, written by
/// Keycloak from its own notes (never from form fields). On acceptance they
/// replace the voter's channel attributes.
const VERIFIED_CHANNEL_ATTRIBUTES: [&str; 5] = [
    MESSAGE_CHANNEL_ATTR_NAME,
    MESSAGE_NUMBER_ATTR_NAME,
    MESSENGER_ID_ATTR_NAME,
    MESSENGER_PAGE_ATTR_NAME,
    MESSAGE_CONSENT_ATTR_NAME,
];

/// First-release channel attributes, removed when the voter's channel is
/// written: Keycloak merges attributes, so leaving one out doesn't remove
/// it.
const REPLACED_CHANNEL_ATTRIBUTES: [&str; 3] = [
    VERIFIED_CHANNELS_ATTR_NAME,
    WHATSAPP_NUMBER_ATTR_NAME,
    VIBER_NUMBER_ATTR_NAME,
];

/// The verified channel carried by an application, as the attributes to
/// write on the voter: the five channel attributes it has, and the
/// first-release ones emptied. Nothing when the application carries no
/// channel.
fn verified_channel_attributes(
    applicant_data: &serde_json::Map<String, Value>,
) -> HashMap<String, Vec<String>> {
    let mut attributes: HashMap<String, Vec<String>> = VERIFIED_CHANNEL_ATTRIBUTES
        .iter()
        .filter_map(|name| {
            let value = applicant_data.get(*name)?;
            let value = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            (!value.is_empty()).then(|| (name.to_string(), vec![value]))
        })
        .collect();
    if attributes.contains_key(MESSAGE_CHANNEL_ATTR_NAME) {
        for name in REPLACED_CHANNEL_ATTRIBUTES {
            attributes.insert(name.to_string(), vec![]);
        }
    }
    attributes
}

/// The application result's content on `channel`: the email for email, the
/// SMS text for the other channels (an approved template, where the channel
/// needs one, comes from the event's bindings).
async fn application_result_content(
    channel: MessageChannel,
    verdict: ApplicationStatus,
    presentation: ElectionEventPresentation,
    variables: &serde_json::Map<String, Value>,
) -> Result<Option<MessageContent>> {
    let method = match channel {
        MessageChannel::EMAIL => TemplateMethod::EMAIL,
        _ => TemplateMethod::SMS,
    };
    let (email, sms) =
        get_application_response_communication(Some(method), verdict, presentation).await?;
    let body = SendTemplateBody {
        audience_selection: None,
        audience_voter_ids: None,
        communication_method: None,
        schedule_now: None,
        schedule_date: None,
        email,
        sms,
        whatsapp: None,
        viber: None,
        messenger: None,
        channel_selection: Default::default(),
        send_id: None,
        document: None,
        name: None,
        alias: None,
        pdf_options: None,
        report_options: None,
        secret_attribute_names: Vec::new(),
        schedule_local: None,
        schedule_timezone: None,
    };
    let contents = render_contents(&body, variables).map_err(|err| anyhow!("{err:?}"))?;
    let source = match channel {
        MessageChannel::EMAIL => MessageChannel::EMAIL,
        _ => MessageChannel::SMS,
    };
    Ok(contents.get(&source).cloned())
}

/// The template reference of an application result: what lets the sweep
/// render it again for a retry. Also its key among the event's approved
/// template bindings.
pub fn application_template_alias(verdict: &ApplicationStatus) -> String {
    format!("application-{}", verdict.to_string().to_lowercase())
}

/// The applicant as the result is sent to them when there is no voter
/// account (a rejection): the names, the email, the phone number and the
/// verified channel the application carries.
fn applicant_user(applicant_data: &HashMap<String, String>) -> User {
    let attributes: HashMap<String, Vec<String>> = std::iter::once(MOBILE_PHONE_ATTR_NAME)
        .chain(VERIFIED_CHANNEL_ATTRIBUTES)
        .filter_map(|name| {
            applicant_data
                .get(name)
                .filter(|value| !value.is_empty())
                .map(|value| (name.to_string(), vec![value.clone()]))
        })
        .collect();
    User {
        id: None,
        attributes: (!attributes.is_empty()).then_some(attributes),
        email: applicant_data.get("email").cloned(),
        email_verified: None,
        enabled: None,
        first_name: applicant_data.get("firstName").cloned(),
        last_name: applicant_data.get("lastName").cloned(),
        username: applicant_data.get("username").cloned(),
        area: None,
        votes_info: None,
    }
}

/// The user's elections, for per-election (Post) channel restrictions:
/// from the user's area, or the area attribute.
fn user_elections(user: &User, elections_by_area: &HashMap<String, Vec<String>>) -> Vec<String> {
    user.area
        .as_ref()
        .and_then(|area| area.id.clone())
        .or_else(|| first_attribute(user, AREA_ID_ATTR_NAME).cloned())
        .and_then(|area_id| elections_by_area.get(&area_id).cloned())
        .unwrap_or_default()
}

/// Who an application result goes to: the user's one channel, with their
/// Messenger Page, and their elections.
fn application_recipient(
    user: &User,
    found: VoterChannel,
    voter_id: Option<String>,
    elections_by_area: &HashMap<String, Vec<String>>,
) -> Recipient {
    Recipient::of_voter(voter_id, found, user_elections(user, elections_by_area))
}

/// The result of a reviewed application for `user`, rendered on their one
/// channel with its template reference, so a transient failure is retried.
/// `None` when the user has no channel.
#[allow(clippy::too_many_arguments)]
pub async fn application_result_delivery(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    application_id: &str,
    user: &User,
    voter_id: Option<String>,
    verdict: ApplicationStatus,
) -> Result<Option<Delivery>> {
    let Some(found) = voter_channel(user.email.as_deref(), user.attributes.as_ref()) else {
        return Ok(None);
    };
    let channel = found.channel;
    let election_event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "Error obtaining election event")?;
    let presentation: ElectionEventPresentation = election_event
        .presentation
        .clone()
        .map(deserialize_value)
        .unwrap_or(Ok(ElectionEventPresentation::default()))?;
    let elections_by_area = get_elections_by_area(hasura_transaction, tenant_id, election_event_id)
        .await
        .with_context(|| "Error listing elections by area")?;
    let time_context =
        NotificationTimeContext::load(hasura_transaction, tenant_id, &election_event)
            .await
            .with_context(|| "Error loading the notification times")?;
    let mut variables = get_variables(
        user,
        Some(election_event.clone()),
        tenant_id.to_string(),
        AuthAction::Login,
    )
    .map_err(|err| anyhow!("{err:?}"))?;
    insert_template_time_variables(
        &mut variables,
        time_context.variables_for(first_attribute(user, AREA_ID_ATTR_NAME).map(String::as_str)),
    );
    let content =
        application_result_content(channel, verdict.clone(), presentation, &variables).await?;
    let alias = application_template_alias(&verdict);
    Ok(Some(Delivery {
        tenant_id: Uuid::parse_str(tenant_id)?,
        election_event_id: Some(Uuid::parse_str(election_event_id)?),
        purpose: MessagePurpose::NOTICE,
        recipient: application_recipient(user, found, voter_id, &elections_by_area),
        content,
        language: first_attribute(user, LOCALE_ATTR_NAME).cloned(),
        template_alias: Some(alias.clone()),
        logical_key: format!("application:{application_id}:{verdict}"),
        expires_at: None,
        account_id: None,
        template_key: Some(alias),
        provider_template: None,
    }))
}

/// Renders an application result again for a due retry, from the
/// application and the user's current record: the voter's for an accepted
/// application, the applicant data for a rejected one.
pub async fn application_retry(
    hasura_transaction: &Transaction<'_>,
    record: &MessageRecord,
    directory: &dyn RetryDirectory,
) -> Result<RetryContent> {
    let refused = |reason| Ok(RetryContent::Refused(reason));
    let parts: Option<(&str, &str)> = record
        .logical_key
        .as_deref()
        .and_then(|key| key.strip_prefix("application:"))
        .and_then(|rest| rest.split_once(':'));
    let (Some((application_id, verdict)), Some(event_id)) = (parts, record.election_event_id)
    else {
        return refused(MessageReason::NO_STORED_TEMPLATE);
    };
    let Ok(verdict) = verdict.parse::<ApplicationStatus>() else {
        return refused(MessageReason::NO_STORED_TEMPLATE);
    };
    let tenant_id = record.tenant_id.to_string();
    let event_id = event_id.to_string();
    let Some(row) = hasura_transaction
        .query_opt(
            "SELECT applicant_data FROM sequent_backend.applications
             WHERE id::text = $1 AND tenant_id = $2",
            &[&application_id, &record.tenant_id],
        )
        .await?
    else {
        return refused(MessageReason::NO_STORED_TEMPLATE);
    };
    let user = match verdict {
        ApplicationStatus::ACCEPTED => {
            let Some(voter_id) = record.voter_id.as_deref() else {
                return refused(MessageReason::NO_CONTACT_CHANNEL);
            };
            match directory
                .voter(hasura_transaction, &tenant_id, Some(&event_id), voter_id)
                .await?
            {
                Some(user) => user,
                None => return refused(MessageReason::NO_CONTACT_CHANNEL),
            }
        }
        _ => {
            let applicant_data: HashMap<String, String> =
                deserialize_value(row.try_get::<_, Value>("applicant_data")?)
                    .map_err(|err| anyhow!("unreadable applicant data: {err}"))?;
            applicant_user(&applicant_data)
        }
    };
    match application_result_delivery(
        hasura_transaction,
        &tenant_id,
        &event_id,
        application_id,
        &user,
        record.voter_id.clone(),
        verdict,
    )
    .await?
    {
        Some(delivery) => Ok(RetryContent::Message(Box::new(delivery))),
        None => refused(MessageReason::NO_CONTACT_CHANNEL),
    }
}

/// Sends the result of a reviewed application, accepted or rejected, on the
/// voter's one channel through the message ledger, and logs the attempt.
/// No other channel is tried; a voter without a channel gets one FAILED
/// record. Called after Keycloak was already changed, so it never fails the
/// review: a problem is logged, and the ledger row (or its absence) is the
/// record.
#[instrument(err, skip_all)]
pub async fn send_application_communication_response(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    application_id: &str,
    user_id: &str,
    admin_id: &str,
    user: &User,
    response_verdict: ApplicationStatus,
) -> Result<()> {
    let _ = admin_id;
    if !matches!(
        response_verdict,
        ApplicationStatus::ACCEPTED | ApplicationStatus::REJECTED
    ) {
        return Ok(());
    }
    // A rejected applicant has no voter account yet.
    let voter_id = user
        .id
        .clone()
        .or_else(|| (response_verdict == ApplicationStatus::ACCEPTED).then(|| user_id.to_string()));
    let sent = async {
        let dispatcher = Dispatcher::global().await?;
        let mut client = get_hasura_pool()
            .await
            .get()
            .await
            .map_err(|err| anyhow!("Error getting hasura db pool: {err}"))?;
        let delivery = application_result_delivery(
            hasura_transaction,
            tenant_id,
            election_event_id,
            application_id,
            user,
            voter_id.clone(),
            response_verdict.clone(),
        )
        .await?;
        match delivery {
            Some(delivery) => deliver(&mut client, dispatcher, &delivery).await,
            None => {
                record_without_channel(
                    &mut client,
                    Uuid::parse_str(tenant_id)?,
                    Some(Uuid::parse_str(election_event_id)?),
                    voter_id.clone(),
                    MessagePurpose::NOTICE,
                    Some(application_template_alias(&response_verdict)),
                    first_attribute(user, LOCALE_ATTR_NAME).cloned(),
                    &format!("application:{application_id}:{response_verdict}"),
                )
                .await
            }
        }
    }
    .await;
    let result = match sent {
        Ok(result) => result,
        Err(error) => {
            warn!("the application result was not sent: {error:#}");
            return Ok(());
        }
    };
    info!(state = %result.state, reason = ?result.reason, "application result message");
    post_attempt_outcomes_or_warn(
        hasura_transaction,
        &result.recorded,
        user.username.clone(),
        first_attribute(user, AREA_ID_ATTR_NAME).cloned(),
    )
    .await;
    Ok(())
}

#[instrument(err, skip_all)]
pub async fn get_group_names(realm: &str, user_id: &str) -> Result<Vec<String>> {
    let client = KeycloakAdminClient::new()
        .await
        .map_err(|err| anyhow!("Error create keycloak admin client: {err}"))?;

    // Fetch user groups from Keycloak
    let _groups = client
        .get_user_groups(&realm, user_id)
        .await
        .map_err(|err| anyhow!("Error fetch group names: {err}"))?;

    // Extract group names
    let group_names: Vec<String> = _groups
        .into_iter()
        .map(|group| group.group_name) // Assuming `group_name` is a String
        .collect();

    // Return group names as a JSON response
    Ok(group_names)
}

#[instrument(skip_all)]
fn string_to_unaccented(word: String) -> String {
    let mut unaccented_word = String::new();
    for l in word.chars() {
        let mut base_char = None;
        decompose_canonical(l, |c| {
            base_char.get_or_insert(c);
        });
        if let Some(base_char) = base_char {
            unaccented_word.push(base_char);
        }
    }
    unaccented_word
}

#[instrument(skip_all)]
fn to_unaccented_without_hyphen(word: Option<String>) -> Option<String> {
    let word = match word {
        Some(word) => word.replace("-", " ").replace(".", ""),
        None => return None,
    };
    let unaccented_word = string_to_unaccented(word);
    Some(unaccented_word)
}

/// Assumes that the inputs are already lowercase
#[instrument]
fn is_fuzzy_match(applicant_value: Option<String>, user_value: Option<String>) -> bool {
    let applicant_value_s = applicant_value.clone().unwrap_or_default();
    let user_value_s = user_value.clone().unwrap_or_default();
    let unaccented_applicant_value =
        to_unaccented_without_hyphen(applicant_value.clone()).unwrap_or_default();
    let unaccented_user_value =
        to_unaccented_without_hyphen(user_value.clone()).unwrap_or_default();
    match (
        applicant_value_s.trim() == user_value_s.trim(),
        applicant_value_s.trim() == unaccented_user_value.trim(),
        unaccented_applicant_value.trim() == user_value_s.trim(),
        unaccented_applicant_value.trim() == unaccented_user_value.trim(),
    ) {
        (false, false, false, false) => false,
        _ => true, // Return true if any condition is true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R7-02: an application result (and its retry, rendered again from the
    /// record) to a Messenger voter carries their Page, so a send through
    /// an account of another Page is refused.
    #[test]
    fn an_application_result_to_a_messenger_voter_carries_their_page() {
        let user = User {
            id: Some("voter-1".to_string()),
            attributes: Some(HashMap::from([
                (
                    "sequent.read-only.message-channel".to_string(),
                    vec!["MESSENGER".to_string()],
                ),
                (
                    "sequent.read-only.messenger-id".to_string(),
                    vec!["6543210".to_string()],
                ),
                (
                    "sequent.read-only.messenger-page".to_string(),
                    vec!["page-old".to_string()],
                ),
            ])),
            ..User::default()
        };
        let found = voter_channel(user.email.as_deref(), user.attributes.as_ref()).unwrap();
        let recipient = application_recipient(&user, found, user.id.clone(), &HashMap::new());
        assert_eq!(recipient.channel, MessageChannel::MESSENGER);
        assert_eq!(recipient.destination, "6543210");
        assert_eq!(recipient.messenger_page.as_deref(), Some("page-old"));
    }

    fn census_annotations(policy: Option<NoMatchingVoterPolicy>) -> ApplicationAnnotations {
        ApplicationAnnotations {
            session_id: None,
            credentials: None,
            verified_by: None,
            rejection_reason: None,
            rejection_message: None,
            unset_attributes: Some("email".to_string()),
            search_attributes: Some("firstName,lastName,dateOfBirth".to_string()),
            update_attributes: None,
            identity_method: None,
            mismatches: None,
            fields_match: None,
            manual_verify_reason: None,
            decision: None,
            no_matching_voter_policy: policy,
        }
    }

    fn census_matrix() -> MatrixVersion {
        MatrixVersion::built_in(
            ["firstName", "lastName", "dateOfBirth"]
                .map(str::to_string)
                .to_vec(),
        )
    }

    fn census_voter() -> User {
        User {
            id: Some("voter-id".to_string()),
            username: Some("voter".to_string()),
            first_name: Some("Maria".to_string()),
            last_name: Some("Santos".to_string()),
            attributes: Some(HashMap::from([(
                "dateOfBirth".to_string(),
                vec!["1980-01-01".to_string()],
            )])),
            ..Default::default()
        }
    }

    fn census_applicant(date_of_birth: &str) -> HashMap<String, String> {
        HashMap::from([
            ("firstName".to_string(), "Maria".to_string()),
            ("lastName".to_string(), "Santos".to_string()),
            ("dateOfBirth".to_string(), date_of_birth.to_string()),
            ("country".to_string(), "Spain/Madrid".to_string()),
            ("embassy".to_string(), "Madrid PE".to_string()),
        ])
    }

    #[test]
    fn single_mismatch_without_embassy_comparison_requires_manual_review() {
        let (result, _) = automatic_verification(
            vec![census_voter()],
            &census_matrix(),
            &census_annotations(None),
            &census_applicant("1990-02-02"),
        )
        .unwrap();

        assert_eq!(result.application_status, ApplicationStatus::PENDING);
        assert_eq!(result.application_type, ApplicationType::MANUAL);
        assert_eq!(result.user_id, None);
    }

    #[test]
    fn exact_match_without_embassy_comparison_is_accepted() {
        let (result, _) = automatic_verification(
            vec![census_voter()],
            &census_matrix(),
            &census_annotations(None),
            &census_applicant("1980-01-01"),
        )
        .unwrap();

        assert_eq!(result.application_status, ApplicationStatus::ACCEPTED);
        assert_eq!(result.user_id, Some("voter-id".to_string()));
    }

    #[test]
    fn test_no_matching_voter_is_rejected_by_default() {
        let (result, decision) = automatic_verification(
            vec![],
            &census_matrix(),
            &census_annotations(None),
            &HashMap::new(),
        )
        .unwrap();
        assert_eq!(decision.decision, ApplicationStatus::REJECTED);
        assert_eq!(result.application_status, ApplicationStatus::REJECTED);
        assert_eq!(result.application_type, ApplicationType::AUTOMATIC);
        assert_eq!(
            result.rejection_reason,
            Some(ApplicationRejectReason::NO_VOTER)
        );
    }

    #[test]
    fn test_no_matching_voter_goes_to_approval_with_its_policy() {
        let (result, decision) = automatic_verification(
            vec![],
            &census_matrix(),
            &census_annotations(Some(NoMatchingVoterPolicy::PENDING_APPROVAL)),
            &HashMap::new(),
        )
        .unwrap();
        // The record keeps what the approval matrix decided.
        assert_eq!(decision.decision, ApplicationStatus::REJECTED);
        assert_eq!(decision.reason, Some(ApplicationRejectReason::NO_VOTER));
        assert_eq!(result.application_status, ApplicationStatus::PENDING);
        assert_eq!(result.application_type, ApplicationType::MANUAL);
        assert_eq!(
            result.rejection_reason,
            Some(ApplicationRejectReason::NO_VOTER)
        );
        assert_eq!(result.user_id, None);
    }

    #[test]
    fn test_no_matching_voter_policy_annotation_is_optional() {
        let absent: ApplicationAnnotations =
            serde_json::from_str(r#"{"search-attributes": "firstName"}"#).unwrap();
        assert_eq!(absent.no_matching_voter_policy, None);
        let pending: ApplicationAnnotations = serde_json::from_str(
            r#"{"search-attributes": "firstName", "no-matching-voter-policy": "PENDING_APPROVAL"}"#,
        )
        .unwrap();
        assert_eq!(
            pending.no_matching_voter_policy,
            Some(NoMatchingVoterPolicy::PENDING_APPROVAL)
        );
    }

    fn default_channels() -> ApplicationCommunicationChannels {
        ApplicationCommunicationChannels {
            email: EmailConfig {
                subject: "default subject".to_string(),
                plaintext_body: "default body".to_string(),
                html_body: None,
            },
            sms: SmsConfig {
                message: "default sms".to_string(),
            },
        }
    }

    fn presentation_with(lang: &str, entries: &[(&str, &str)]) -> ElectionEventPresentation {
        let mut presentation = ElectionEventPresentation::default();
        presentation.i18n = Some(HashMap::from([(
            lang.to_string(),
            entries
                .iter()
                .map(|(key, value)| (key.to_string(), Some(value.to_string())))
                .collect(),
        )]));
        presentation
    }

    #[test]
    fn application_messages_read_scoped_overrides_from_the_localization_tab() {
        let presentation = presentation_with(
            "en",
            &[
                (
                    "global:application.accepted.email.subject",
                    "global subject",
                ),
                ("application.accepted.email.subject", "legacy subject"),
                (
                    "templates:application.accepted.email.subject",
                    "templates subject",
                ),
                ("global:application.accepted.sms.message", "global sms"),
                ("application.accepted.email.plaintext_body", "legacy body"),
                (
                    "votingPortal:application.accepted.email.html_body",
                    "portal only",
                ),
            ],
        );

        let channels = apply_application_overrides(
            default_channels(),
            &presentation,
            "en",
            &ApplicationStatus::ACCEPTED,
        );

        assert_eq!(channels.email.subject, "templates subject");
        assert_eq!(channels.email.plaintext_body, "legacy body");
        assert_eq!(channels.sms.message, "global sms");
        assert_eq!(channels.email.html_body, None);
    }

    #[test]
    fn application_messages_keep_the_defaults_without_overrides() {
        let channels = apply_application_overrides(
            default_channels(),
            &presentation_with(
                "es",
                &[("templates:application.accepted.sms.message", "es")],
            ),
            "en",
            &ApplicationStatus::ACCEPTED,
        );

        assert_eq!(channels.sms.message, "default sms");
    }

    #[test]
    fn application_results_keep_the_voters_post_restrictions() {
        let elections = HashMap::from([("area-1".to_string(), vec!["post-1".to_string()])]);
        let by_attribute = User {
            attributes: Some(HashMap::from([(
                AREA_ID_ATTR_NAME.to_string(),
                vec!["area-1".to_string()],
            )])),
            ..User::default()
        };
        assert_eq!(
            user_elections(&by_attribute, &elections),
            vec!["post-1".to_string()]
        );
        assert!(user_elections(&User::default(), &elections).is_empty());
        assert_eq!(
            application_template_alias(&ApplicationStatus::REJECTED),
            "application-rejected"
        );
    }

    #[test]
    fn test_accent_mark() {
        let applicant_value: Option<String> = Some("manuel".to_string());
        let user_value: Option<String> = Some("mánuel".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());

        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_grave_accent() {
        let applicant_value: Option<String> = Some("pierre".to_string());
        let user_value: Option<String> = Some("pièrre".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());
        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_circumflex() {
        let applicant_value: Option<String> = Some("paulo".to_string());
        let user_value: Option<String> = Some("paulô".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());
        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_tilde() {
        let applicant_value: Option<String> = Some("manuel".to_string());
        let user_value: Option<String> = Some("mañuel".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());
        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_umlaut() {
        let applicant_value: Option<String> = Some("muller".to_string());
        let user_value: Option<String> = Some("müller".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());
        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_umlaut_not_equal() {
        // German umlaut will not match with its 2 characters equivalents
        let applicant_value: Option<String> = Some("Mueller".to_string());
        let user_value: Option<String> = Some("Müller".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());
        assert!(
            !is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_hyphen_equals_space() {
        let applicant_value: Option<String> = Some("von-der-leyen".to_string());
        let user_value: Option<String> = Some("von der leyen".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());
        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_hyphen_equals_space_reverse() {
        let applicant_value: Option<String> = Some("von der leyen".to_string());
        let user_value: Option<String> = Some("von-der-leyen".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());

        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }

    #[test]
    fn test_none_vs_empty_string() {
        let applicant_value: Option<String> = None;
        let user_value: Option<String> = Some(" ".to_string());
        let is_match = is_fuzzy_match(applicant_value.clone(), user_value.clone());

        assert!(
            is_match,
            "applicant_value ({:?}) does not match user_value ({:?})",
            applicant_value, user_value
        );
    }
}

#[cfg(test)]
#[path = "application_matrix_tests.rs"]
mod matrix_tests;

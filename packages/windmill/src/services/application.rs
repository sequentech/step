// SPDX-FileCopyrightText: 2024 Sequent Legal <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::users::{lookup_users, FilterOption, ListUsersFilter};
use crate::postgres::application::get_permission_label_from_post;
use crate::postgres::area::get_event_areas;
use crate::postgres::election_event::get_election_event_by_id;
use crate::services::celery_app::get_celery_app;
use crate::services::providers::{email_sender::EmailSender, sms_sender::SmsSender};
use crate::services::reports::utils::get_public_asset_template;
use crate::services::temp_path::PUBLIC_ASSETS_I18N_DEFAULTS;
use crate::tasks::send_template::{send_template, send_template_email_or_sms};
use crate::types::application::ApplicationRejectReason;
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

use sequent_core::services::keycloak::get_event_realm;
use sequent_core::services::keycloak::KeycloakAdminClient;
use sequent_core::services::translations::DEFAULT_LANG;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::keycloak::{User, MOBILE_PHONE_ATTR_NAME};
use sequent_core::types::templates::{EmailConfig, SendTemplateBody, SmsConfig, TemplateMethod};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum_macros::Display;
use tracing::{debug, event, info, instrument, warn, Level};
use uuid::Uuid;

use sequent_core::types::templates::AudienceSelection::SELECTED;
use sequent_core::types::templates::TemplateMethod::{EMAIL, SMS};
use unicode_normalization::char::decompose_canonical;

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

    // Generate a filter with applicant data
    let filter = get_filter_from_applicant_data(
        tenant_id.to_string(),
        Some(election_event_id.to_string()),
        None,
        None,
        realm,
        None,
        &annotations,
        applicant_data,
    )?;

    // Uses applicant data to lookup possible users
    let users = lookup_users(hasura_transaction, keycloak_transaction, filter).await?;
    debug!("Found users before verification: {:?}", users);

    // Finds an user from the list of found possible users
    let result = automatic_verification(users.clone(), &annotations, applicant_data)?;
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
        mismatches: result.mismatches,
        fields_match: result.fields_match.clone(),
        manual_verify_reason: result.manual_verify_reason.clone(),
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

    Ok(result)
}

#[instrument(err, skip_all)]
async fn get_permission_label_and_area_from_applicant_data(
    hasura_transaction: &Transaction<'_>,
    applicant_data: &HashMap<String, String>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(Option<String>, Option<Uuid>)> {
    let post_name = applicant_data
        .get("country")
        .and_then(|country| country.split('/').next())
        .ok_or(anyhow!("Error with applicant country"))?;

    let post_description = applicant_data
        .get("embassy")
        .ok_or(anyhow!("Error with applicant embassy"))?;

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
    annotations: &ApplicationAnnotations,
    applicant_data: &HashMap<String, String>,
) -> Result<ListUsersFilter> {
    let search_attributes: String = annotations.search_attributes.clone().ok_or(anyhow!(
        "Error obtaining search_attributes from annotations"
    ))?;

    let mut first_name = None;
    let mut last_name = None;
    let mut username = None;
    let mut email = None;
    let mut attributes_map = HashMap::new();

    for attribute in search_attributes.split(",") {
        match attribute {
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
                    .get(attribute)
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
    mismatches: Option<usize>,
    fields_match: Option<HashMap<String, bool>>,
    manual_verify_reason: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApplicationVerificationResult {
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

#[instrument(err, skip_all)]
fn automatic_verification(
    users: Vec<User>,
    annotations: &ApplicationAnnotations,
    applicant_data: &HashMap<String, String>,
) -> Result<ApplicationVerificationResult> {
    let search_attributes: String = annotations.search_attributes.clone().ok_or(anyhow!(
        "Error obtaining search_attributes from annotations"
    ))?;

    let unset_attributes: String = annotations
        .unset_attributes
        .clone()
        .ok_or(anyhow!("Error obtaining unset_attributes from annotations"))?;

    // Set fields match all to false for default response
    let fields_match: HashMap<String, bool> = search_attributes
        .split(",")
        .map(|field| (field.trim().to_string(), false))
        .collect();

    let mut matched_user: Option<User> = None;
    let mut matched_status = ApplicationStatus::REJECTED;
    let mut matched_type = ApplicationType::AUTOMATIC;
    let mut verification_mismatches = Some(fields_match.len());
    let mut verification_fields_match = Some(fields_match);
    let mut verification_attributes_unset = None;
    let mut rejection_reason: Option<ApplicationRejectReason> =
        Some(ApplicationRejectReason::NO_VOTER);
    let mut rejection_message: Option<String> = None;
    let mut mismatch_reason = None;

    for user in users {
        let (mismatches, mismatches_unset, fields_match, attributes_unset) = check_mismatches(
            &user,
            applicant_data,
            search_attributes.clone(),
            unset_attributes.clone(),
        )?;
        let username = user.username.clone().unwrap_or_default();

        if mismatches > 0 {
            mismatch_reason = Some(build_manual_verify_reason(fields_match.clone()));
        }

        // If there are no mismatches..
        if mismatches == 0 {
            // if the fields that need to be unset but were set is more than 0,
            // this means we need to automatically reject. This is a user that
            // already exists and is verified.
            if mismatches_unset > 0 {
                matched_user = Some(user);
                matched_status = ApplicationStatus::REJECTED;
                matched_type = ApplicationType::AUTOMATIC;
                verification_mismatches = Some(mismatches);
                verification_fields_match = Some(fields_match);
                verification_attributes_unset = Some(attributes_unset);
                rejection_reason = Some(ApplicationRejectReason::ALREADY_APPROVED);
                rejection_message = None;
            } else {
                return Ok(ApplicationVerificationResult {
                    user_id: user.id,
                    username,
                    application_status: ApplicationStatus::ACCEPTED,
                    application_type: ApplicationType::AUTOMATIC,
                    mismatches: Some(mismatches),
                    fields_match: Some(fields_match),
                    attributes_unset: Some(attributes_unset),
                    rejection_reason: None,
                    rejection_message: None,
                    manual_verify_reason: None,
                });
            }
        // If there was only 1 mismatch
        } else if mismatches == 1 {
            // if the fields that need to be unset but were set is more than 0,
            // this means we need to automatically reject. This is a user that
            // already exists and is verified.
            if mismatches_unset > 0 {
                matched_user = Some(user);
                matched_status = ApplicationStatus::REJECTED;
                matched_type = ApplicationType::AUTOMATIC;
                verification_mismatches = Some(mismatches);
                verification_fields_match = Some(fields_match);
                verification_attributes_unset = Some(attributes_unset);
                rejection_reason = Some(ApplicationRejectReason::ALREADY_APPROVED);
                rejection_message = None;
            } else {
                if !fields_match.get("embassy").unwrap_or(&false) {
                    return Ok(ApplicationVerificationResult {
                        user_id: user.id,
                        username,
                        application_status: ApplicationStatus::ACCEPTED,
                        application_type: ApplicationType::AUTOMATIC,
                        mismatches: Some(mismatches),
                        fields_match: Some(fields_match),
                        attributes_unset: Some(attributes_unset),
                        rejection_reason: None,
                        rejection_message: None,
                        manual_verify_reason: None,
                    });
                }
                matched_user = None;
                matched_status = ApplicationStatus::PENDING;
                matched_type = ApplicationType::MANUAL;
                verification_mismatches = Some(mismatches);
                verification_fields_match = Some(fields_match);
                verification_attributes_unset = Some(attributes_unset);
                rejection_reason = Some(ApplicationRejectReason::NO_VOTER);
                rejection_message = None;
            }
        } else if mismatches == 2 && !fields_match.get("embassy").unwrap_or(&false) {
            matched_user = None;
            matched_status = ApplicationStatus::PENDING;
            matched_type = ApplicationType::MANUAL;
            verification_mismatches = Some(mismatches);
            verification_fields_match = Some(fields_match);
            verification_attributes_unset = Some(attributes_unset);
            rejection_reason = Some(ApplicationRejectReason::NO_VOTER);
            rejection_message = None;
        } else if mismatches == 2
            && !fields_match.get("middleName").unwrap_or(&false)
            && !fields_match.get("lastName").unwrap_or(&false)
        {
            matched_user = None;
            matched_status = ApplicationStatus::PENDING;
            matched_type = ApplicationType::MANUAL;
            verification_mismatches = Some(mismatches);
            verification_fields_match = Some(fields_match);
            verification_attributes_unset = Some(attributes_unset);
            rejection_reason = Some(ApplicationRejectReason::NO_VOTER);
            rejection_message = None;
        } else if matched_status != ApplicationStatus::PENDING {
            matched_user = None;
            matched_status = ApplicationStatus::REJECTED;
            matched_type = ApplicationType::AUTOMATIC;
            verification_mismatches = Some(mismatches);
            verification_fields_match = Some(fields_match);
            verification_attributes_unset = Some(attributes_unset);
            rejection_reason = Some(ApplicationRejectReason::NO_VOTER);
            rejection_message = None;
        }
    }

    info!("matched_status: {}", matched_status.to_string());
    info!(
        "rejection_reason: {}",
        rejection_reason.clone().unwrap_or_default().to_string()
    );

    Ok(ApplicationVerificationResult {
        user_id: matched_user.clone().and_then(|user| user.id),
        username: matched_user
            .clone()
            .and_then(|user| user.username)
            .unwrap_or_default(),
        application_status: matched_status,
        application_type: matched_type,
        mismatches: verification_mismatches,
        fields_match: verification_fields_match,
        attributes_unset: verification_attributes_unset,
        rejection_reason,
        rejection_message,
        manual_verify_reason: mismatch_reason,
    })
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

    let card_type = applicant_data
        .get("sequent.read-only.id-card-type")
        .cloned()
        .ok_or(anyhow!("Error converting applicant_data to map"))?;

    // Check if the card type is seamans_book or driver_license
    let card_type_flag = card_type == ECardType::SEAMANS_BOOK.to_string()
        || card_type == ECardType::DRIVER_LICENSE.to_string();

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
    let mut application_channels =
        get_i18n_default_application_communication(&lang, app_status.clone()).await?;
    let Some(localization_map) = presentation
        .i18n
        .map(|val| val.get(lang).cloned())
        .flatten()
    else {
        return Ok(application_channels);
    };
    let key_prefix = format!("application.{}", app_status.to_string().to_lowercase());

    if let Some(sms_message) = localization_map
        .get(&format!("{key_prefix}.sms.message"))
        .cloned()
        .flatten()
    {
        application_channels.sms.message = sms_message;
    };

    if let Some(email_subject) = localization_map
        .get(&format!("{key_prefix}.email.subject"))
        .cloned()
        .flatten()
    {
        application_channels.email.subject = email_subject;
    };

    if let Some(plaintext_body) = localization_map
        .get(&format!("{key_prefix}.email.plaintext_body"))
        .cloned()
        .flatten()
    {
        application_channels.email.plaintext_body = plaintext_body;
    };

    if let Some(html_body) = localization_map
        .get(&format!("{key_prefix}.email.html_body"))
        .cloned()
        .flatten()
    {
        application_channels.email.html_body = Some(html_body);
    };

    Ok(application_channels)
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
            serde_json::from_value::<Vec<CredentialRepresentation>>(value.clone())
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

    info!("update_attributes={attributes:?}, attributes_to_store={attributes_to_store:?}");

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

    let first_name = applicant_data.get("firstName").map(String::from);
    let last_name = applicant_data.get("lastName").map(String::from);
    let username = applicant_data.get("username").map(String::from);
    let email = applicant_data.get("email").map(String::from);

    let phone_number: Option<String> = applicant_data.get(MOBILE_PHONE_ATTR_NAME).map(String::from);

    let attributes = match phone_number {
        Some(phone_number) => {
            let mut attr: HashMap<String, Vec<String>> = HashMap::new();
            attr.insert(
                MOBILE_PHONE_ATTR_NAME.to_string(),
                vec![phone_number.clone()],
            );
            Some(attr)
        }
        None => None,
    };

    let user = User {
        id: None,
        attributes, // The phone is needed for the sms and should go in the attributes.
        email,
        email_verified: None,
        enabled: None,
        first_name,
        last_name,
        username,
        area: None,
        votes_info: None,
    };

    send_application_communication_response(
        hasura_transaction,
        tenant_id,
        election_event_id,
        user_id,
        admin_id,
        &user,
        ApplicationStatus::REJECTED,
    )
    .await
    .map_err(|err| anyhow!("Error sending communication response: {err}"))?;

    Ok(application)
}

#[instrument(err, skip_all)]
pub async fn send_application_communication_response(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    user_id: &str,
    admin_id: &str,
    user: &User,
    response_verdict: ApplicationStatus,
) -> Result<()> {
    // Check if voter provided email otherwise use SMS
    let communication_method = if user.email.is_some() {
        Some(EMAIL)
    } else if user
        .attributes
        .as_ref()
        .and_then(|attributes| attributes.get(MOBILE_PHONE_ATTR_NAME))
        .and_then(|values| values.first())
        .map(|value| value.to_string())
        .is_some()
    {
        Some(SMS)
    } else {
        None
    };

    // Get the presentation to obtain the default language and presentation.i18n
    let presentation: ElectionEventPresentation =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
            .await
            .with_context(|| "Error obtaining election event")?
            .presentation
            .map(deserialize_value)
            .unwrap_or(Ok(ElectionEventPresentation::default()))?;

    let (email_config, sms_config) = get_application_response_communication(
        communication_method.clone(),
        response_verdict.clone(),
        presentation,
    )
    .await?;

    match response_verdict {
        ApplicationStatus::ACCEPTED => {
            let user_ids = vec![user_id.to_string()];
            // Send confirmation email or SMS
            let payload: SendTemplateBody = SendTemplateBody {
                audience_selection: Some(SELECTED),
                audience_voter_ids: Some(user_ids),
                r#type: Some(
                    sequent_core::types::templates::TemplateType::MANUALLY_VERIFY_APPROVAL,
                ),
                communication_method,
                schedule_now: Some(true),
                schedule_date: None,
                email: email_config,
                sms: sms_config,
                document: None,
                name: None,
                alias: None,
                pdf_options: None,
                report_options: None,
            };

            let celery_app = get_celery_app().await;

            let task = celery_app
                .send_task(send_template::new(
                    payload,
                    tenant_id.to_string(),
                    admin_id.to_string(),
                    Some(election_event_id.to_string()),
                ))
                .await?;
            event!(Level::INFO, "Sent SEND_TEMPLATE task {}", task.task_id);
        }

        ApplicationStatus::REJECTED => {
            let email_sender = EmailSender::new().await?;
            let sms_sender = SmsSender::new().await?;
            send_template_email_or_sms(
                hasura_transaction,
                user,
                &None,
                tenant_id,
                None,
                &email_config, // EmailConfig
                &sms_config,   // SmsConfig
                &email_sender,
                &sms_sender,
                communication_method.clone(),
            )
            .await
            .map_err(|err| anyhow!("Error sending email or sms: {err}"))?;
        }
        _ => {}
    }

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

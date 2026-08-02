// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::template_renderer::{
    GenerateReportMode, ReportOriginatedFrom, ReportOrigins, TemplateRenderer,
};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::reports::ReportType;
use crate::services::temp_path::PUBLIC_ASSETS_LOGO_IMG;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::{
    get_event_realm, get_realm_attributes, KeycloakAdminClient,
};
use sequent_core::services::pdf;
use sequent_core::services::s3::get_minio_url;
use sequent_core::types::keycloak::{
    CredentialInputPolicy, REALM_ATTR_CREDENTIAL_INPUT_PATTERN, REALM_ATTR_CREDENTIAL_INPUT_POLICY,
};
use sequent_core::util::temp_path::get_public_assets_path_env_var;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::fmt::{Debug, Formatter};
use std::str::FromStr;
use tracing::instrument;

#[derive(Serialize, Deserialize, Clone)]
pub struct UserData {
    pub election_event_name: String,
    pub issue_date: String,
    pub voter_first_name: String,
    pub voter_last_name: String,
    pub voter_full_name: String,
    pub username: String,
    pub password: String,
    pub voting_portal_url: String,
    pub logo_url: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
    pub file_logo: String,
}

pub struct VoterInformationLetterTemplate {
    ids: ReportOrigins,
    credential: String,
}

impl Debug for VoterInformationLetterTemplate {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("VoterInformationLetterTemplate")
            .field("ids", &self.ids)
            .field("credential", &"[REDACTED]")
            .finish()
    }
}

impl VoterInformationLetterTemplate {
    pub fn new(
        tenant_id: String,
        election_event_id: String,
        voter_id: String,
        credential: String,
    ) -> Self {
        Self {
            ids: ReportOrigins {
                tenant_id,
                election_event_id,
                election_id: None,
                template_alias: None,
                voter_id: Some(voter_id),
                report_origin: ReportOriginatedFrom::ExportFunction,
                executer_username: None,
                tally_session_id: None,
            },
            credential,
        }
    }

    pub fn new_preview(ids: ReportOrigins) -> Self {
        Self {
            ids,
            credential: String::new(),
        }
    }

    #[instrument(skip_all, err)]
    pub async fn render_pdf(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Vec<u8>> {
        let (user_template, extra_config) = self
            .user_tpl_and_extra_cfg_provider(hasura_transaction)
            .await
            .with_context(|| "Failed to load Voter Information Letter template")?;
        let html = self
            .generate_report_inner(
                GenerateReportMode::REAL,
                hasura_transaction,
                keycloak_transaction,
                &user_template,
            )
            .await
            .with_context(|| "Failed to render Voter Information Letter template")?;

        pdf::PdfRenderer::render_pdf(
            html,
            Some(extra_config.pdf_options.to_print_to_pdf_options()),
        )
        .await
        .with_context(|| "Failed to render Voter Information Letter PDF")
    }
}

fn translated_event_name(presentation: Option<&Value>) -> Option<String> {
    let presentation = presentation?;
    let language = presentation
        .get("language_conf")
        .and_then(|configuration| configuration.get("default_language_code"))
        .and_then(Value::as_str)
        .unwrap_or("en");
    let i18n = presentation.get("i18n")?;

    i18n.get(language)
        .and_then(|translation| translation.get("name"))
        .and_then(Value::as_str)
        .or_else(|| {
            i18n.as_object().and_then(|translations| {
                translations
                    .values()
                    .find_map(|translation| translation.get("name").and_then(Value::as_str))
            })
        })
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn apply_structured_pattern(credential: &str, pattern: &str) -> Option<String> {
    if credential.is_empty() || !credential.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    let mut digits = credential.bytes();
    let mut formatted = String::with_capacity(pattern.len());
    for token in pattern.bytes() {
        match token {
            b'd' => formatted.push(digits.next()? as char),
            b'-' => formatted.push('-'),
            _ => return None,
        }
    }

    if digits.next().is_some() {
        None
    } else {
        Some(formatted)
    }
}

fn credential_for_presentation(
    credential: &str,
    input_policy: Option<&str>,
    input_pattern: Option<&str>,
) -> String {
    let is_structured = input_policy.and_then(|value| CredentialInputPolicy::from_str(value).ok())
        == Some(CredentialInputPolicy::STRUCTURED);
    if !is_structured {
        return credential.to_string();
    }

    input_pattern
        .and_then(|pattern| apply_structured_pattern(credential, pattern))
        .unwrap_or_else(|| credential.to_string())
}

#[async_trait]
impl TemplateRenderer for VoterInformationLetterTemplate {
    type UserData = UserData;
    type SystemData = SystemData;

    fn base_name(&self) -> String {
        "credentials".to_string()
    }

    fn get_report_type(&self) -> ReportType {
        ReportType::CREDENTIALS
    }

    fn prefix(&self) -> String {
        format!(
            "voter_information_letter_{}",
            self.ids.voter_id.clone().unwrap_or_default()
        )
    }

    fn get_tenant_id(&self) -> String {
        self.ids.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.ids.election_event_id.clone()
    }

    fn get_report_origin(&self) -> ReportOriginatedFrom {
        self.ids.report_origin
    }

    fn get_initial_template_alias(&self) -> Option<String> {
        self.ids.template_alias.clone()
    }

    fn get_voter_id(&self) -> Option<String> {
        self.ids.voter_id.clone()
    }

    fn contains_sensitive_data(&self) -> bool {
        true
    }

    #[instrument(skip_all, err)]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        _keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        let voter_id = self
            .ids
            .voter_id
            .as_deref()
            .ok_or_else(|| anyhow!("Missing voter id"))?;
        let event = get_election_event_by_id(
            hasura_transaction,
            &self.ids.tenant_id,
            &self.ids.election_event_id,
        )
        .await?;
        let realm = get_event_realm(&self.ids.tenant_id, &self.ids.election_event_id);
        let voter = KeycloakAdminClient::new()
            .await?
            .get_user(&realm, voter_id)
            .await?;
        let realm_attributes =
            get_realm_attributes(&self.ids.tenant_id, &self.ids.election_event_id)
                .await
                .with_context(|| "Failed to load election event credential presentation")?;
        let password = credential_for_presentation(
            &self.credential,
            realm_attributes
                .get(REALM_ATTR_CREDENTIAL_INPUT_POLICY)
                .map(String::as_str),
            realm_attributes
                .get(REALM_ATTR_CREDENTIAL_INPUT_PATTERN)
                .map(String::as_str),
        );

        let first_name = voter.first_name.unwrap_or_default();
        let last_name = voter.last_name.unwrap_or_default();
        let username = voter
            .username
            .ok_or_else(|| anyhow!("Voter does not have a username"))?;
        let portal_base = env::var("VOTING_PORTAL_URL")
            .map_err(|_| anyhow!("VOTING_PORTAL_URL env var missing"))?;

        Ok(UserData {
            election_event_name: translated_event_name(event.presentation.as_ref())
                .or(event.description)
                .unwrap_or_else(|| "Election".to_string()),
            issue_date: Utc::now().format("%B %-d, %Y").to_string(),
            voter_full_name: format!("{} {}", first_name, last_name).trim().to_string(),
            voter_first_name: first_name,
            voter_last_name: last_name,
            username,
            password,
            voting_portal_url: format!(
                "{}/tenant/{}/event/{}/login",
                portal_base.trim_end_matches('/'),
                self.ids.tenant_id,
                self.ids.election_event_id
            ),
            logo_url: format!(
                "{}/{}/{}",
                get_minio_url()?,
                get_public_assets_path_env_var()?,
                PUBLIC_ASSETS_LOGO_IMG
            ),
        })
    }

    #[instrument(skip_all, err)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
            file_logo: format!(
                "{}/{}/{}",
                get_minio_url()?,
                get_public_assets_path_env_var()?,
                PUBLIC_ASSETS_LOGO_IMG
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::credential_for_presentation;

    #[test]
    fn formats_matching_structured_credentials() {
        assert_eq!(
            credential_for_presentation(
                "1234567890123456",
                Some("structured"),
                Some("dddd-dddd-dddd-dddd"),
            ),
            "1234-5678-9012-3456",
        );
    }

    #[test]
    fn leaves_standard_credentials_unchanged() {
        assert_eq!(
            credential_for_presentation(
                "1234567890123456",
                Some("standard"),
                Some("dddd-dddd-dddd-dddd"),
            ),
            "1234567890123456",
        );
    }

    #[test]
    fn leaves_credentials_unchanged_without_a_pattern() {
        assert_eq!(
            credential_for_presentation("1234567890123456", Some("structured"), None),
            "1234567890123456",
        );
    }

    #[test]
    fn leaves_credentials_unchanged_when_the_pattern_does_not_match() {
        assert_eq!(
            credential_for_presentation(
                "12345678",
                Some("structured"),
                Some("dddd-dddd-dddd-dddd"),
            ),
            "12345678",
        );
    }
}

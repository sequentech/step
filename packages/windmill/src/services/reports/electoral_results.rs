// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{Local, TimeZone};
use deadpool_postgres::{Client as DbClient, Transaction};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use velvet::pipes::generate_reports::TemplateData;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct ElectoralResults {
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
}

impl ElectoralResults {
    pub fn new(tenant_id: String, election_event_id: String, election_id: Option<String>) -> Self {
        ElectoralResults {
            tenant_id,
            election_event_id,
            election_id,
        }
    }
}

#[async_trait]
impl TemplateRenderer for ElectoralResults {
    type UserData = TemplateData;
    type SystemData = SystemData;

    fn get_report_type() -> ReportType {
        ReportType::ELECTORAL_RESULTS
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_election_id(&self) -> Option<String> {
        self.election_id.clone()
    }

    fn base_name() -> String {
        "electoral_results".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "{base_name}_{election_event_id}_{election_id:?}",
            base_name = Self::base_name(),
            election_event_id = self.election_event_id,
            election_id = self.election_id,
        )
    }

    #[instrument(err, skip(self, hasura_transaction, keycloak_transaction))]
    async fn prepare_user_data(
        &self,
        hasura_transaction: &Transaction<'_>,
        keycloak_transaction: &Transaction<'_>,
    ) -> Result<Self::UserData> {
        Err(anyhow::anyhow!("Unimplemented"))
    }

    #[instrument(err, skip(self))]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}

#[instrument(err, skip(hasura_transaction, keycloak_transaction))]
pub async fn generate_report(
    document_id: &str,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
    mode: GenerateReportMode,
    hasura_transaction: &Transaction<'_>,
    keycloak_transaction: &Transaction<'_>,
) -> Result<()> {
    let renderer = ElectoralResults::new(
        tenant_id.to_string(),
        election_event_id.to_string(),
        election_id.map(|s| s.to_string()),
    );
    renderer
        .execute_report(
            document_id,
            tenant_id,
            election_event_id,
            false,
            None,
            mode,
            hasura_transaction,
            keycloak_transaction,
        )
        .await
}

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::template_renderer::*;
use crate::postgres::reports::ReportType;
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_postgres::{Client as DbClient, Transaction};
use serde::{Deserialize, Serialize};
use tracing::{info, instrument};
use velvet::pipes::generate_reports::TemplateData;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SystemData {
    pub rendered_user_template: String,
}

#[derive(Debug)]
pub struct InitializationTemplate {
    tenant_id: String,
    election_event_id: String,
    election_id: Option<String>,
}

impl InitializationTemplate {
    pub fn new(tenant_id: String, election_event_id: String, election_id: Option<String>, template_id: Option<String>) -> Self {
        InitializationTemplate {
            tenant_id,
            election_event_id,
            election_id,
            template_id,
        }
    }
}

#[async_trait]
impl TemplateRenderer for InitializationTemplate {
    type UserData = TemplateData;
    type SystemData = SystemData;

    fn get_report_type(&self) -> ReportType {
        ReportType::INITIALIZATION_REPORT
    }

    fn get_tenant_id(&self) -> String {
        self.tenant_id.clone()
    }

    fn get_election_event_id(&self) -> String {
        self.election_event_id.clone()
    }

    fn get_template_id(&self) -> Option<String> {
        self.template_id.clone()
    }

    fn get_election_id(&self) -> Option<String> {
        self.election_id.clone()
    }

    fn base_name(&self) -> String {
        "initialization_report".to_string()
    }

    fn prefix(&self) -> String {
        format!(
            "{base_name}_{election_event_id}_{election_id:?}",
            base_name = self.base_name(),
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

    #[instrument(err, skip_all)]
    async fn prepare_system_data(
        &self,
        rendered_user_template: String,
    ) -> Result<Self::SystemData> {
        Ok(SystemData {
            rendered_user_template,
        })
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Database audit collection uses the same native append and receipt contract.
use super::audit_types::ElectoralLogMessage;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use strum_macros::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Display, EnumString)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DatabaseAuditColumn {
    AuditType,
    Class,
    Command,
    Dbname,
    SessionId,
    Statement,
    User,
}
impl DatabaseAuditColumn {
    pub fn sql(self) -> &'static str {
        match self {
            Self::AuditType => "(database_metadata->>'audit_type')",
            Self::Class => "(database_metadata->>'class')",
            Self::Command => "(database_metadata->>'command')",
            Self::Dbname => "(database_metadata->>'dbname')",
            Self::SessionId => "(database_metadata->>'session_id')",
            Self::Statement => "(database_metadata->>'statement')",
            Self::User => "(database_metadata->>'user')",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum DatabaseAuditTable {
    #[default]
    PgauditHasura,
    PgauditKeycloak,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseAuditRow {
    #[serde(default)]
    pub id: i64,
    pub audit_type: String,
    pub class: String,
    pub command: String,
    pub dbname: String,
    pub server_timestamp: i64,
    pub session_id: String,
    pub statement: String,
    pub user: String,
}
impl DatabaseAuditRow {
    pub fn into_record(mut self, table: DatabaseAuditTable) -> Result<ElectoralLogMessage> {
        self.id = 0;
        Ok(ElectoralLogMessage {
            id: 0,
            created: self.server_timestamp,
            sender_pk: self.user.clone(),
            statement_timestamp: self.server_timestamp,
            statement_kind: table.to_string(),
            message: serde_json::to_vec(&self)?,
            version: "1".into(),
            user_id: Some(self.user),
            username: None,
            election_id: None,
            area_id: None,
            ballot_id: None,
        })
    }
}

pub fn database_audit_board(
    tenant: &str,
    event: &str,
    table: DatabaseAuditTable,
) -> Result<String> {
    let tenant = uuid::Uuid::parse_str(tenant)?;
    let event = uuid::Uuid::parse_str(event)?;
    Ok(format!("{table}-{tenant}-{event}"))
}

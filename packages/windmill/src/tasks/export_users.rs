// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::hasura::election_event::get_election_event;
use crate::postgres::area::get_areas_by_name;
use crate::postgres::keycloak_realm;
use crate::services::database::{get_hasura_pool, get_keycloak_pool};
use crate::services::s3;
use crate::types::error::{Error, Result};
use anyhow::{anyhow, Context};
use base64::prelude::*;
use celery::error::TaskError;
use csv::StringRecord;
use deadpool_postgres::{Client as DbClient, Transaction as _};
use futures::pin_mut;
use rand::prelude::*;
use rand::{thread_rng, Rng};
use regex::Regex;
use ring::{digest, pbkdf2};
use rocket::futures::SinkExt as _;
use sequent_core::services::keycloak::{get_event_realm, get_tenant_realm};
use sequent_core::services::{keycloak, reports};
use sequent_core::types::keycloak::TENANT_ID_ATTR_NAME;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Seek;
use std::num::NonZeroU32;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};
use tracing::{debug, info, instrument};

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct ExportUsersBody {
    pub tenant_id: String,
    pub election_event_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportUsersOutput {
    pub document_id: String,
    pub task_id: String,
}

#[instrument(err)]
#[wrap_map_err::wrap_map_err(TaskError)]
#[celery::task(max_retries = 0)]
pub async fn export_users(body: ExportUsersBody, document_id: String) -> Result<()> {
    Ok(())
}

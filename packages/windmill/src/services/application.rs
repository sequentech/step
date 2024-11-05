// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::services::cast_votes::get_users_with_vote_info;
use crate::services::database::PgConfig;
use crate::{
    postgres::application::{insert_application, update_confirm_application},
    postgres::area::get_areas,
    types::application::ApplicationStatus,
    types::application::ApplicationType,
};
use anyhow::{anyhow, Context, Result};
use deadpool_postgres::Transaction;
use futures::stream::Filter;
use sequent_core::types::hasura::core::Application;
use sequent_core::types::keycloak::*;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    convert::From,
};
use tokio_postgres::row::Row;
use tokio_postgres::types::ToSql;
use tracing::{event, info, instrument, Level};
use uuid::Uuid;

#[instrument(skip(hasura_transaction), err)]
pub async fn verify_application(
    hasura_transaction: &Transaction<'_>,
    applicant_id: &str,
    applicant_data: &Value,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    labels: &Option<Value>,
    annotations: &Option<Value>,
) -> Result<()> {
    // TODO Search user

    // TODO User match matrix

    // If user not matched create a manual verification application
    insert_application(
        hasura_transaction,
        tenant_id,
        election_event_id,
        area_id,
        &applicant_id,
        applicant_data,
        labels,
        annotations,
        ApplicationType::MANUAL,
        ApplicationStatus::PENDING,
    )
    .await?;

    // TODO Respond with user found if success

    Ok(())
}

#[instrument(skip(hasura_transaction), err)]
pub async fn confirm_application(
    hasura_transaction: &Transaction<'_>,
    id: &str,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
    user_id: &str,
) -> Result<Option<Application>> {
    // TODO Update user attributes.

    // Update the application to ACCEPTED
    let result = update_confirm_application(
        hasura_transaction,
        &id,
        &tenant_id,
        &election_event_id,
        &area_id,
        ApplicationStatus::ACCEPTED,
    )
    .await?;

    Ok(result)
}

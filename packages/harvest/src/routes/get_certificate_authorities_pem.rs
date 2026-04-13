// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Client as DbClient;
use rocket::http::{ContentType, Status};
use tracing::instrument;
use windmill::postgres::certificate_authority::get_certificate_authorities_pem;
use windmill::services::database::get_hasura_pool;

/// Returns all CA certificates as a concatenated PEM bundle
/// (plain text, no authentication required).
#[instrument]
#[get("/certificate-authorities/pem", format = "any")]
pub async fn get_cas_pem() -> Result<(ContentType, String), (Status, String)> {
    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let pems = get_certificate_authorities_pem(&hasura_transaction)
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let bundle = pems.join("\n");
    Ok((ContentType::new("application", "x-pem-file"), bundle))
}

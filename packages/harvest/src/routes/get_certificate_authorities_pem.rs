// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use deadpool_postgres::Client as DbClient;
use rocket::http::{ContentType, Status};
use tracing::instrument;
use uuid::Uuid;
use windmill::postgres::certificate_authority::get_certificate_authorities_pem;
use windmill::services::database::get_hasura_pool;

/// Returns all CA certificates for the given election event as a
/// concatenated PEM bundle (plain text, no authentication required).
#[instrument]
#[get(
    "/election-event/<election_event_id>/certificate-authorities/pem",
    format = "any"
)]
pub async fn get_cas_pem(
    election_event_id: &str,
) -> Result<(ContentType, String), (Status, String)> {
    let election_event_uuid =
        Uuid::parse_str(election_event_id).map_err(|e| {
            (
                Status::BadRequest,
                format!("Invalid election_event_id: {e}"),
            )
        })?;

    let mut hasura_db_client: DbClient = get_hasura_pool()
        .await
        .get()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;
    let hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let pems = get_certificate_authorities_pem(
        &hasura_transaction,
        election_event_uuid,
    )
    .await
    .map_err(|e| (Status::InternalServerError, format!("{e:?}")))?;

    let bundle = pems.join("\n");
    Ok((ContentType::new("application", "x-pem-file"), bundle))
}

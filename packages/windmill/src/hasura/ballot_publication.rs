// SPDX-FileCopyrightText: 2023 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use chrono::NaiveDateTime;
use graphql_client::{GraphQLQuery, Response};
use reqwest;
use std::env;
use tracing::{event, instrument, Level};

use crate::services::date::ISO8601;
use crate::services::to_result::ToResult;
pub use crate::types::hasura_types::*;
use sequent_core::services::connection;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/insert_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct InsertBallotPublication;

#[instrument(skip_all)]
pub async fn insert_ballot_publication(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    election_ids: Vec<String>,
    user_id: String,
) -> Result<Response<insert_ballot_publication::ResponseData>> {
    let variables = insert_ballot_publication::Variables {
        tenant_id,
        election_event_id,
        election_ids,
        user_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = InsertBallotPublication::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<insert_ballot_publication::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetBallotPublication;

#[instrument(skip_all)]
pub async fn get_ballot_publication(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<Response<get_ballot_publication::ResponseData>> {
    let variables = get_ballot_publication::Variables {
        tenant_id,
        election_event_id,
        ballot_publication_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetBallotPublication::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_ballot_publication::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/update_ballot_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct UpdateBallotPublication;

#[instrument(skip_all)]
pub async fn update_ballot_publication_d(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
    is_generated: bool,
    published_at: Option<NaiveDateTime>,
) -> Result<Response<update_ballot_publication::ResponseData>> {
    let published_at_str = published_at.clone().map(|naive| ISO8601::from_date(&naive));
    let variables = update_ballot_publication::Variables {
        ballot_publication_id: ballot_publication_id,
        election_event_id: election_event_id,
        tenant_id: tenant_id,
        is_generated: is_generated,
        published_at: published_at_str,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = UpdateBallotPublication::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<update_ballot_publication::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/soft_delete_other_ballot_publications.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize",
    variables_derives = "Debug,Clone"
)]
pub struct SoftDeleteOtherBallotPublications;

#[instrument(skip(auth_headers))]
pub async fn soft_delete_other_ballot_publications(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<Response<soft_delete_other_ballot_publications::ResponseData>> {
    let variables = soft_delete_other_ballot_publications::Variables {
        ballot_publication_id: ballot_publication_id,
        election_event_id: election_event_id,
        tenant_id: tenant_id,
    };
    event!(Level::INFO, "request_body {:?}", variables.clone());
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = SoftDeleteOtherBallotPublications::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<soft_delete_other_ballot_publications::ResponseData> =
        res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_previous_publication.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetPreviousPublication;

#[instrument(skip_all)]
pub async fn get_previous_publication(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    published_at: String,
    election_ids: Option<Vec<String>>,
) -> Result<Response<get_previous_publication::ResponseData>> {
    let variables = get_previous_publication::Variables {
        tenant_id,
        election_event_id,
        published_at,
        election_ids,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetPreviousPublication::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_previous_publication::ResponseData> = res.json().await?;
    response_body.ok()
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_publication_ballot_styles.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetPublicationBallotStyles;

#[instrument(skip_all)]
pub async fn get_publication_ballot_styles(
    auth_headers: connection::AuthHeaders,
    tenant_id: String,
    election_event_id: String,
    ballot_publication_id: String,
) -> Result<Response<get_publication_ballot_styles::ResponseData>> {
    let variables = get_publication_ballot_styles::Variables {
        ballot_publication_id: ballot_publication_id,
        election_event_id: election_event_id,
        tenant_id: tenant_id,
    };
    let hasura_endpoint =
        env::var("HASURA_ENDPOINT").expect(&format!("HASURA_ENDPOINT must be set"));
    let request_body = GetPublicationBallotStyles::build_query(variables);

    let client = reqwest::Client::new();
    let res = client
        .post(hasura_endpoint)
        .header(auth_headers.key, auth_headers.value)
        .json(&request_body)
        .send()
        .await?;
    let response_body: Response<get_publication_ballot_styles::ResponseData> = res.json().await?;
    response_body.ok()
}

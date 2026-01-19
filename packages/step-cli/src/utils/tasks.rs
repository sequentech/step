// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::{types::hasura_types::*, utils::read_config::read_config};
use ::uuid::Uuid;
use graphql_client::{GraphQLQuery, Response};

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.json",
    query_path = "src/graphql/get_task_execution.graphql",
    response_derives = "Debug,Clone,Deserialize,Serialize"
)]
pub struct GetTaskExecution;

pub fn get_task_status(task_execution_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let config = read_config()?;
    let client = reqwest::blocking::Client::new();

    let variables = get_task_execution::Variables {
        task_execution_id: Uuid::parse_str(task_execution_id)?.to_string(),
    };

    let request_body = GetTaskExecution::build_query(variables);

    let response = client
        .post(&config.endpoint_url)
        .bearer_auth(config.auth_token)
        .json(&request_body)
        .send()?;

    let response_body: Response<get_task_execution::ResponseData> = response.json()?;
    match (response_body.data, response_body.errors) {
        (Some(data), _) => data
            .sequent_backend_tasks_execution
            .first()
            .map(|task| task.execution_status.clone())
            .ok_or("No task execution found".into()),
        (None, Some(errors)) => {
            let messages = errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join(", ");
            Err(messages.into())
        }
        _ => Err("Unknown error: empty data and no GraphQL errors".into()),
    }
}

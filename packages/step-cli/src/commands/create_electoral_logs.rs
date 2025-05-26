// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use clap::Args;
use electoral_log::messages::message::{Message, Sender};
use electoral_log::messages::newtypes::EventIdString;
use electoral_log::messages::statement::{
    Statement, StatementBody, StatementEventType, StatementHead, StatementLogType, StatementType,
};
use electoral_log::ElectoralLogMessage;
use fake::faker::internet::raw::Username;
use fake::locales::EN;
use fake::Fake;
use immudb_rs::{sql_value::Value as ImmudbValue, Client as ImmudbClient, NamedParam, SqlValue};
use std::env;
use strand::signature::{StrandSignature, StrandSignaturePk};
use windmill::services::protocol_manager::get_event_board;
use windmill::services::providers::transactions_provider::provide_immudb_transaction;

#[derive(Args)]
#[command(about)]
pub struct CreateElectoralLogs {
    /// Working directory for input/output
    #[arg(long)]
    working_directory: String,

    #[arg(long)]
    num_logs: usize,
}

#[derive(Debug, Clone)]
struct Voter {
    id: Option<String>,
    username: Option<String>,
}

impl CreateElectoralLogs {
    /// Execute the rendering process
    pub fn run(&self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        match runtime
            .block_on(self.run_create_electoral_logs(&self.working_directory, self.num_logs))
        {
            Ok(_) => println!("Successfully created electoral logs."),
            Err(err) => eprintln!("Error! Failed to create electoral logs: {err:?}"),
        }
    }

    fn generate_log_message(
        &self,
        election_event_id: &str,
        election_id: &str,
        area_id: &str,
        user_id: Option<String>,
        username: Option<String>,
    ) -> Result<ElectoralLogMessage> {
        let created = Utc::now().timestamp();
        let statement_timestamp = created;

        let dummy_bytes = [0u8; 64];
        let sender_signature = StrandSignature::from_bytes(dummy_bytes.clone())
            .expect("Dummy signature conversion failed");
        let system_signature =
            StrandSignature::from_bytes(dummy_bytes).expect("Dummy signature conversion failed");

        let dummy_bytes = [0u8; 32];
        let sender_pk = StrandSignaturePk::from_bytes(dummy_bytes)
            .with_context(|| "Error in create Dummy StrandSignaturePk")?;

        let message = &Message {
            sender: Sender {
                name: username.clone().unwrap_or_default(),
                pk: sender_pk,
            },
            sender_signature: sender_signature,
            system_signature: system_signature,
            statement: Statement {
                head: StatementHead {
                    event: EventIdString(election_event_id.to_string()),
                    kind: StatementType::SendCommunications,
                    timestamp: statement_timestamp as u64,
                    event_type: StatementEventType::SYSTEM,
                    log_type: StatementLogType::INFO,
                    description: "Send Communications.".to_string(),
                },
                body: StatementBody::SendCommunications(None),
            },
            artifact: None,
            user_id: user_id,
            username: username.clone(),
            election_id: Some(election_id.to_string()),
            area_id: Some(area_id.to_string()),
            ballot_id: None,
        };

        let board_message: ElectoralLogMessage = message.try_into().with_context(|| "")?;
        Ok(board_message)
    }

    async fn run_create_electoral_logs(
        &self,
        working_dir: &str,
        num_logs: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_external_config(working_dir)?;
        let tenant_id = config.tenant_id;
        let election_event_id = config.election_event_id;
        let election_id = config.election_id;
        let immudb_db = get_event_board(&tenant_id, &election_event_id);
        let area_id = config.area_id;
        let realm_name = config.realm_name;

        println!("immudb_db: {}", &immudb_db);

        let kc_client = get_keyckloak_pool()
            .await?
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

        let keycloak_query = "\
            SELECT 
                ue.id,
                ue.username
            FROM user_entity AS ue \
            JOIN realm AS r ON ue.realm_id = r.id \
            JOIN user_attribute AS ua ON ue.id = ua.user_id \
            WHERE r.name = $1 AND ua.name = 'area-id' AND ua.value = $2 \
            LIMIT $3 OFFSET 0";

        let users = kc_client
            .query(keycloak_query, &[&realm_name, &area_id, &(num_logs as i64)])
            .await?;

        let existing_users: Vec<Voter> = users
            .iter()
            .filter_map(|row| {
                let id = row.get::<_, Option<String>>(0);
                let username = row.get::<_, Option<String>>(1);
                Some(Voter { id, username })
            })
            .collect();

        let mut logs_params: Vec<Vec<NamedParam>> = Vec::new();
        for user in existing_users {
            let username = Some(user.username.clone().unwrap_or_else(|| Username(EN).fake()));
            let user_id_cloned = user.id.clone();

            let message: ElectoralLogMessage = self
                .generate_log_message(
                    &election_event_id,
                    &election_id,
                    &area_id,
                    user_id_cloned.clone(),
                    username.clone(),
                )
                .with_context(|| "Error generating log message")?;

            let params = vec![
                NamedParam {
                    name: "created".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::Ts(message.created)),
                    }),
                },
                NamedParam {
                    name: "sender_pk".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::S(message.sender_pk)),
                    }),
                },
                NamedParam {
                    name: "statement_kind".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::S(message.statement_kind)),
                    }),
                },
                NamedParam {
                    name: "statement_timestamp".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::Ts(message.statement_timestamp)),
                    }),
                },
                NamedParam {
                    name: "message".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::Bs(message.message)),
                    }),
                },
                NamedParam {
                    name: "version".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::S(message.version)),
                    }),
                },
                NamedParam {
                    name: "user_id".to_string(),
                    value: Some(SqlValue {
                        value: user_id_cloned.map(ImmudbValue::S),
                    }),
                },
                NamedParam {
                    name: "username".to_string(),
                    value: Some(SqlValue {
                        value: username.map(ImmudbValue::S),
                    }),
                },
                NamedParam {
                    name: "election_id".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::S(election_id.to_string())),
                    }),
                },
                NamedParam {
                    name: "area_id".to_string(),
                    value: Some(SqlValue {
                        value: Some(ImmudbValue::S(area_id.to_string())),
                    }),
                },
            ];
            logs_params.push(params);
        }

        provide_immudb_transaction(
            |client, tx_id| {
                let logs_params = logs_params.clone();
                Box::pin(async move { insert_logs(client, &tx_id, logs_params).await })
            },
            immudb_db.as_str(),
        )
        .await?;

        println!("Inserted {} logs.", num_logs);

        Ok(())
    }
}

async fn insert_logs(
    client: &mut ImmudbClient,
    tx_id: &str,
    logs_params: Vec<Vec<NamedParam>>,
) -> Result<()> {
    let batch_size = env::var("DEFAULT_SQL_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);

    for batch in logs_params.chunks(batch_size) {
        let mut query = String::from("INSERT INTO electoral_log_messages (created, sender_pk, statement_kind, statement_timestamp, message, version, user_id, username, election_id, area_id) VALUES ");
        let mut values_clauses = Vec::new();
        let mut all_params: Vec<NamedParam> = Vec::new();
        let mut row_index = 1;

        for row in batch {
            let mut clause_parts = Vec::new();
            for param in row {
                let new_name = format!("{}{}", param.name, row_index);
                clause_parts.push(format!("@{}", new_name));
                all_params.push(NamedParam {
                    name: new_name,
                    value: param.value.clone(),
                });
            }
            row_index += 1;
            values_clauses.push(format!("({})", clause_parts.join(", ")));
        }

        query.push_str(&values_clauses.join(", "));
        client
            .tx_sql_exec(&query, &(tx_id.to_string()), all_params)
            .await
            .map_err(|e| anyhow!("Failed to execute query: {:?}", e))?;
    }

    Ok(())
}

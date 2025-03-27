// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use crate::utils::keycloak::get_keyckloak_pool;
use crate::utils::read_config::load_external_config;
use anyhow::Result;
use clap::Args;
use deadpool_postgres::Transaction;
use fake::faker::number::raw::NumberWithFormat;
use fake::locales::EN;
use fake::Fake;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use strum::VariantNames;
use tokio_postgres::types::ToSql;
use tokio_postgres::Row;
use uuid::Uuid;
use windmill::services::providers::transactions_provider::provide_hasura_transaction;
use windmill::types::application::{ApplicationStatus, ApplicationType};
#[derive(Args)]
#[command(about)]
pub struct CreateApplications {
    /// Working directory for input/output
    #[arg(long)]
    working_directory: String,

    #[arg(long)]
    num_applications: usize,

    /// Optional status: PENDING, ACCEPTED or REJECTED
    #[arg(long)]
    status: Option<ApplicationStatus>,

    /// Optional verification type: AUTOMATIC or MANUAL
    #[arg(long)]
    r#type: Option<ApplicationType>,
}

impl CreateApplications {
    /// Execute the rendering process
    pub fn run(&self) {
        let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        match runtime.block_on(self.run_create_applications(
            &self.working_directory,
            self.num_applications,
            self.status.clone(),
            self.r#type.clone(),
        )) {
            Ok(_) => println!("Successfully created applications"),
            Err(err) => eprintln!("Error! Failed to create applications: {err:?}"),
        }
    }

    pub async fn run_create_applications(
        &self,
        working_dir: &str,
        num_applications: usize,
        status: Option<ApplicationStatus>,
        verification_type: Option<ApplicationType>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_external_config(working_dir)?;
        let realm_name = config.realm_name;
        let tenant_id = config.tenant_id;
        let election_event_id = config.election_event_id;
        let generate_applications_config = config.generate_applications;
        let default_applicant_data = generate_applications_config.applicant_data;
        let annotations = serde_json::to_value(&generate_applications_config.annotations)?;

        let kc_client = get_keyckloak_pool()
            .await?
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

        // --- Query Keycloak for user details ---
        let query = "\
            SELECT 
                ue.id,
                ue.username,
                ue.email,
                ue.first_name,
                ue.last_name,
                (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'area-id' LIMIT 1) AS area_id,
                (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'country' LIMIT 1) AS country,
                (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'embassy' LIMIT 1) AS embassy,
                (SELECT ua.value FROM user_attribute ua WHERE ua.user_id = ue.id AND ua.name = 'dateOfBirth' LIMIT 1) AS dateOfBirth
            FROM user_entity ue
            JOIN realm r ON ue.realm_id = r.id
            WHERE r.name = $1 LIMIT $2 OFFSET 0;";
        let users = kc_client
            .query(query, &[&realm_name, &(num_applications as i64)])
            .await?;
        println!("Number of existing user rows: {}", users.len());

        provide_hasura_transaction(|hasura_transaction| {
            let existing_users = users.clone();
            let status = status.clone();
            let verification_type = verification_type.clone();
            let tenant_id = tenant_id.clone();
            let election_event_id = election_event_id.clone();
            let default_applicant_data = default_applicant_data.clone();
            let annotations = annotations.clone();

            Box::pin(async move {
                set_applications(
                    hasura_transaction,
                    existing_users,
                    status,
                    verification_type,
                    tenant_id,
                    election_event_id,
                    default_applicant_data,
                    annotations,
                )
                .await
            })
        })
        .await?;

        Ok(())
    }
}

async fn get_permission_label(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    area_id: &str,
) -> Result<String> {
    let permissions_label_query = r#"
        SELECT el.permission_label
        FROM sequent_backend.area a
            LEFT JOIN sequent_backend.area_contest ac ON a.id = ac.area_id
            LEFT JOIN sequent_backend.contest con ON ac.contest_id = con.id
            LEFT JOIN sequent_backend.election el ON con.election_id = el.id
        WHERE
            a.id = $1 AND
            a.tenant_id = $2 AND
            ac.tenant_id = $2 AND
            con.tenant_id = $2 AND
            el.tenant_id = $2 AND
            a.election_event_id = $3 AND
            ac.election_event_id = $3 AND
            con.election_event_id = $3 AND
            el.election_event_id = $3
        LIMIT 1
    "#;
    let rows = hasura_transaction
        .query(
            permissions_label_query,
            &[
                &Uuid::parse_str(area_id)?,
                &Uuid::parse_str(tenant_id)?,
                &Uuid::parse_str(election_event_id)?,
            ],
        )
        .await?;
    if let Some(row) = rows.get(0) {
        let permission_label: String = row.get(0);
        Ok(permission_label)
    } else {
        Ok("".to_string())
    }
}

async fn set_applications(
    hasura_transaction: &Transaction<'_>,
    users: Vec<Row>,
    status: Option<ApplicationStatus>,
    verification_type: Option<ApplicationType>,
    tenant_id: String,
    election_event_id: String,
    default_applicant_data: HashMap<String, Value>,
    annotations: Value,
) -> Result<()> {
    let mut permissions_labels: HashMap<String, String> = HashMap::new();
    // --- Build application rows as Vec<Vec<Box<dyn ToSql + Sync>>> ---
    // The columns are:
    // 0: applicant_id (String)
    // 1: status (String)
    // 2: verification_type (String)
    // 3: applicant_data (serde_json::Value)
    // 4: tenant_id (String)
    // 5: election_event_id (String)
    // 6: area_id (String)
    // 7: annotations (serde_json::Value)
    let mut users_params: Vec<Vec<Box<dyn ToSql + Send + Sync>>> = Vec::new();
    for user in users {
        let user_id: String = user.get(0);
        let username: String = user.get(1);
        let email: String = user.get(2);
        let first_name: String = user.get(3);
        let last_name: String = user.get(4);
        let area_id_opt: Option<String> = user.get(5);
        let country: Option<String> = user.get(6);
        let embassy: Option<String> = user.get(7);
        let date_of_birth: Option<String> = user.get(8);

        let application_status = match status.clone() {
            Some(status) => status.to_string(),
            None => ApplicationStatus::VARIANTS
                .choose(&mut thread_rng())
                .unwrap()
                .to_string(),
        };

        let verification_type = match verification_type.clone() {
            Some(vt) => vt.to_string(),
            None => {
                if application_status == ApplicationStatus::PENDING.to_string() {
                    println!("Setting verification type to MANUAL for PENDING application");
                    ApplicationType::MANUAL.to_string()
                } else {
                    println!("Setting verification no pendong");
                    ApplicationType::VARIANTS
                        .choose(&mut thread_rng())
                        .unwrap()
                        .to_string()
                }
            }
        };

        // Merge default applicant data with user details.
        let mut applicant_data = default_applicant_data.clone();
        applicant_data.insert("email".to_string(), Value::String(email.clone()));
        applicant_data.insert("firstName".to_string(), Value::String(first_name));
        applicant_data.insert("lastName".to_string(), Value::String(last_name));
        applicant_data.insert("username".to_string(), Value::String(username.clone()));
        applicant_data.insert(
            "country".to_string(),
            Value::String(country.unwrap_or_default()),
        );
        applicant_data.insert(
            "embassy".to_string(),
            Value::String(embassy.unwrap_or_default()),
        );
        applicant_data.insert(
            "dateOfBirth".to_string(),
            Value::String(date_of_birth.unwrap_or_default()),
        );
        let id_card_number = format!("C{}", NumberWithFormat(EN, "##########").fake::<String>());
        applicant_data.insert(
            "sequent.read-only.id-card-number".to_string(),
            Value::String(id_card_number),
        );
        let applicant_data_value = serde_json::to_value(&applicant_data)?;
        let area_id_value = area_id_opt.unwrap_or_default();
        let area_id = area_id_value.as_str();

        let permission_label = if let Some(label) = permissions_labels.get(area_id) {
            label.clone()
        } else {
            let label = get_permission_label(
                &hasura_transaction,
                &tenant_id,
                &election_event_id,
                &area_id,
            )
            .await?;
            permissions_labels.insert(area_id_value.clone(), label.clone());
            label
        };

        // Build the vector of parameters.
        let mut params: Vec<Box<dyn ToSql + Send + Sync>> = Vec::new();
        params.push(Box::new(user_id));
        params.push(Box::new(application_status));
        params.push(Box::new(verification_type.clone()));
        params.push(Box::new(applicant_data_value));
        params.push(Box::new(Uuid::parse_str(&tenant_id)?));
        params.push(Box::new(Uuid::parse_str(&election_event_id)?));
        params.push(Box::new(Uuid::parse_str(area_id)?));
        params.push(Box::new(&annotations));
        params.push(Box::new(permission_label.clone()));

        users_params.push(params);
    }

    println!(
        "Number of application rows to insert: {}",
        users_params.len()
    );

    let batch_size = env::var("DEFAULT_SQL_BATCH_SIZE")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(1000);

    for batch in users_params.chunks(batch_size) {
        // Build the INSERT query with multiple rows.
        // There are 8 columns per row.
        let mut query = String::from("INSERT INTO sequent_backend.applications (applicant_id, status, verification_type, applicant_data, tenant_id, election_event_id, area_id, annotations,permission_label) VALUES ");
        let mut placeholders = Vec::new();
        // Build a vector of parameters as trait objects.
        let mut flat_params: Vec<&(dyn ToSql + Sync)> = Vec::new();
        let mut param_index = 1;
        for appliciant in batch {
            let mut appliciant_placeholders = Vec::new();
            for (i, value) in appliciant.iter().enumerate() {
                // For applicant_data (index 3) and annotations (index 7), add an explicit cast to jsonb.
                if i == 3 || i == 7 {
                    appliciant_placeholders.push(format!("${}::jsonb", param_index));
                } else {
                    appliciant_placeholders.push(format!("${}", param_index));
                }
                param_index += 1;
                flat_params.push(&**value);
            }
            placeholders.push(format!("({})", appliciant_placeholders.join(", ")));
        }
        query.push_str(&placeholders.join(", "));
        hasura_transaction
            .execute(query.as_str(), &flat_params)
            .await?;
    }
    Ok(())
}

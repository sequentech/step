// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use chrono::Utc;
use clap::Args;
use csv::Writer;
use fake::faker::name::raw::{FirstName, LastName};
use fake::locales::EN;
use fake::Fake;
use rand::seq::IndexedRandom;
use rand::seq::SliceRandom;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use chrono::{Duration, NaiveDate};
use rand::Rng;

use crate::utils::read_config::load_external_config;

#[derive(Args)]
#[command(about)]
pub struct GenerateVoters {
    /// Working directory for input/output
    #[arg(long)]
    working_directory: String,

    #[arg(long)]
    num_users: usize,
}

impl GenerateVoters {
    /// Execute the rendering process
    pub fn run(&self) {
        match self.run_generate_voters(&self.working_directory, self.num_users) {
            Ok(_) => println!("Successfully generated voters into csv"),
            Err(err) => eprintln!("Error! Failed to generate voters: {err:?}"),
        }
    }

    fn generate_fake_dob(&self, min_age: i64, max_age: i64) -> NaiveDate {
        let today = Utc::now().date_naive();
        let max_date = today - Duration::days(min_age * 365);
        let min_date = today - Duration::days(max_age * 365);
        let days_diff = (max_date - min_date).num_days();
        let random_days = rand::thread_rng().gen_range(0..=days_diff);
        min_date + Duration::days(random_days)
    }

    /// Deduplicate items while preserving order.
    fn deduplicate_preserve_order<T: std::hash::Hash + Eq + Clone>(&self, items: &[T]) -> Vec<T> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for item in items {
            if seen.insert(item.clone()) {
                result.push(item.clone());
            }
        }
        result
    }

    fn run_generate_voters(
        &self,
        working_dir: &str,
        num_users: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let config = load_external_config(working_dir)?;

        // Get election event file path from config (or default).
        let election_event_file = config.election_event_json_file;
        let election_event_path = PathBuf::from(working_dir).join(election_event_file);
        let election_file = File::open(election_event_path)?;
        let election_data: Value = serde_json::from_reader(BufReader::new(election_file))?;

        // Get voters configuration with defaults.
        let voters_config = config.generate_voters;
        let csv_file_name = format!("{}_{}.csv", voters_config.csv_file_name, num_users);
        let csv_file_path = PathBuf::from(working_dir).join(&csv_file_name);

        let fields: Vec<String> = voters_config.fields;
        let excluded_columns: Vec<String> = voters_config.excluded_columns;

        let email_prefix = voters_config.email_prefix;
        let domain = voters_config.domain;
        let sequence_email_number = voters_config.sequence_email_number;
        let sequence_start_number = voters_config.sequence_start_number;
        let voter_password = voters_config.voter_password;
        let password_salt = voters_config.password_salt;
        let hashed_password = voters_config.hashed_password;
        let min_age = voters_config.min_age;
        let max_age = voters_config.max_age;
        let overseas_reference = voters_config.overseas_reference;

        // Parse election event file parts.
        let areas: &[serde_json::Value] = election_data
            .get("areas")
            .and_then(Value::as_array)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let area_contests: &[serde_json::Value] = election_data
            .get("area_contests")
            .and_then(Value::as_array)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let contests: &[serde_json::Value] = election_data
            .get("contests")
            .and_then(Value::as_array)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let elections: &[serde_json::Value] = election_data
            .get("elections")
            .and_then(Value::as_array)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        // Build election mapping.
        let mut election_map = std::collections::HashMap::new();
        for el in elections {
            if let Some(e_id) = el.get("id").and_then(Value::as_str) {
                let alias = el.get("alias").and_then(Value::as_str).unwrap_or("Unknown");
                let cluster_prec = el
                    .get("annotations")
                    .and_then(|ann| ann.get("clustered_precint_id"))
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown");
                election_map.insert(
                    e_id.to_string(),
                    (alias.to_string(), cluster_prec.to_string()),
                );
            }
        }

        // Build area -> contest mapping.
        let mut area_contest_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for ac in area_contests {
            if let (Some(a_id), Some(c_id)) = (
                ac.get("area_id").and_then(Value::as_str),
                ac.get("contest_id").and_then(Value::as_str),
            ) {
                area_contest_map
                    .entry(a_id.to_string())
                    .or_default()
                    .push(c_id.to_string());
            }
        }

        // Build contest to election mapping.
        let mut contest_election_map = std::collections::HashMap::new();
        for c in contests {
            if let Some(c_id) = c.get("id").and_then(Value::as_str) {
                let e_id = c
                    .get("election_id")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown");
                contest_election_map.insert(c_id.to_string(), e_id.to_string());
            }
        }

        // Parse Keycloak config for country/embassy.
        let mut cou_emb_dict = std::collections::HashMap::new();
        if let Some(kc_event) = election_data.get("keycloak_event_realm") {
            if let Some(components) = kc_event.get("components") {
                if let Some(uprovs) = components.get("org.keycloak.userprofile.UserProfileProvider")
                {
                    let uprovs_arr = if uprovs.is_array() {
                        uprovs.as_array().unwrap().clone()
                    } else {
                        vec![uprovs.clone()]
                    };
                    if let Some(first_uprov) = uprovs_arr.first() {
                        if let Some(conf) = first_uprov.get("config") {
                            if let Some(kc_conf_list) =
                                conf.get("kc.user.profile.config").and_then(Value::as_array)
                            {
                                if let Some(raw_json_str) =
                                    kc_conf_list.first().and_then(Value::as_str)
                                {
                                    if let std::result::Result::Ok(user_profile_config) =
                                        serde_json::from_str::<Value>(raw_json_str)
                                    {
                                        if let Some(attrs) = user_profile_config
                                            .get("attributes")
                                            .and_then(Value::as_array)
                                        {
                                            for at in attrs {
                                                if at.get("name").and_then(Value::as_str)
                                                    == Some("country")
                                                {
                                                    if let Some(validations) = at.get("validations")
                                                    {
                                                        if let Some(options) = validations
                                                            .get("options")
                                                            .and_then(|o| o.get("options"))
                                                            .and_then(Value::as_array)
                                                        {
                                                            for opt in options {
                                                                if let Some(opt_str) = opt.as_str()
                                                                {
                                                                    if opt_str.contains('/') {
                                                                        let parts: Vec<&str> =
                                                                            opt_str
                                                                                .splitn(2, '/')
                                                                                .collect();
                                                                        cou_emb_dict.insert(
                                                                            parts[1].to_lowercase(),
                                                                            (
                                                                                parts[0]
                                                                                    .trim()
                                                                                    .to_string(),
                                                                                parts[1]
                                                                                    .trim()
                                                                                    .to_string(),
                                                                            ),
                                                                        );
                                                                    } else {
                                                                        cou_emb_dict.insert(
                                                                            opt_str.to_lowercase(),
                                                                            (
                                                                                opt_str.to_string(),
                                                                                "Unknown"
                                                                                    .to_string(),
                                                                            ),
                                                                        );
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let final_fields: Vec<String> = fields
            .into_iter()
            .filter(|f| !excluded_columns.contains(f))
            .collect();
        let mut wtr = Writer::from_path(&csv_file_path)?;
        wtr.write_record(&final_fields)?;

        let mut username_counter = 0;
        let mut area_cycle = areas.iter().cycle();

        for i in 0..num_users {
            let area = area_cycle.next().unwrap_or(&Value::Null);
            let area_id = area.get("id").and_then(Value::as_str).unwrap_or("Unknown");
            let area_name = area
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Unknown");

            let assigned_cids = area_contest_map
                .get(area_id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            let mut election_aliases = Vec::new();
            let mut precincts = Vec::new();

            for cid in assigned_cids {
                let unknown_e_id = "Unknown".to_string();
                let e_id = contest_election_map
                    .get(cid)
                    .unwrap_or(&unknown_e_id)
                    .to_string();
                let default_value = (String::from("Unknown"), String::from("Unknown"));
                let (alias, cluster_prec) = election_map.get(&e_id).unwrap_or(&default_value);
                election_aliases.push(alias.clone());
                precincts.push(cluster_prec.clone());
            }
            election_aliases = self.deduplicate_preserve_order(&election_aliases);
            precincts = self.deduplicate_preserve_order(&precincts);

            let election_country_candidate = if let Some(first_alias) = election_aliases.first() {
                if first_alias.contains(" - ") {
                    first_alias
                        .splitn(2, " - ")
                        .next()
                        .unwrap_or("Unknown")
                        .trim()
                        .to_string()
                } else {
                    first_alias.trim().to_string()
                }
            } else {
                "Unknown".to_string()
            };

            let lookup_key = election_country_candidate.to_lowercase();
            let (official_country, official_embassy) = cou_emb_dict
                .get(&lookup_key)
                .cloned()
                .unwrap_or_else(|| (election_country_candidate.clone(), "Unknown".to_string()));
            let joined_aliases = if !election_aliases.is_empty() {
                election_aliases.join("|")
            } else {
                "Unknown".to_string()
            };
            let joined_precincts = if !precincts.is_empty() {
                precincts.join("|")
            } else {
                "Unknown".to_string()
            };

            let dob = self.generate_fake_dob(min_age, max_age);
            let dob_str = dob.format("%Y-%m-%d").to_string();

            let email = if sequence_email_number {
                format!(
                    "{}+{}@{}",
                    email_prefix,
                    i as i64 + sequence_start_number,
                    domain
                )
            } else {
                let random_num: u32 = rand::random::<u32>() % 900_000_000 + 100_000;
                format!("{}+{}@{}", email_prefix, random_num, domain)
            };

            // Instead of storing the user record in a vector, we build the CSV record directly.
            let mut record = Vec::with_capacity(final_fields.len());
            // For each expected field, extract its value from our generated data.
            for field in &final_fields {
                let value = match field.as_str() {
                    "username" => username_counter.to_string(),
                    "first_name" => FirstName(EN).fake(),
                    "last_name" => LastName(EN).fake(),
                    "middleName" => String::new(),
                    "dateOfBirth" => dob_str.clone(),
                    "sex" => {
                        if *[true, false].choose(&mut rand::rng()).unwrap() {
                            "M".to_string()
                        } else {
                            "F".to_string()
                        }
                    }
                    "country" => format!("{}/{}", official_country, official_embassy),
                    "embassy" => official_embassy.clone(),
                    "clusteredPrecinct" => joined_precincts.clone(),
                    "overseasReferences" => overseas_reference.to_string(),
                    "area_name" => area_name.to_string(),
                    "authorized-election-ids" => joined_aliases.clone(),
                    "password" => voter_password.to_string(),
                    "email" => email.clone(),
                    "password_salt" => password_salt.to_string(),
                    "hashed_password" => hashed_password.to_string(),
                    _ => "".to_string(), // default empty if field not recognized
                };
                record.push(value);
            }

            // Write the record to the CSV file.
            wtr.write_record(&record)?;
            username_counter += 1;

            // Optionally, log progress every so often rather than every record.
            if i % 10000 == 0 {
                println!("Generated {} users...", i);
            }
        }
        wtr.flush()?;

        println!(
            "Successfully generated {} users. CSV file created at: {}",
            num_users,
            csv_file_path.canonicalize()?.display()
        );
        Ok(())
    }
}

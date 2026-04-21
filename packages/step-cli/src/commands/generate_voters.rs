// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use chrono::Utc;
use chrono::{Duration, NaiveDate};
use clap::Args;
use colored::Colorize;
use csv::Writer;
use fake::faker::name::raw::{FirstName, LastName};
use fake::locales::EN;
use fake::Fake;
use rand::seq::IndexedRandom;
use rand::seq::SliceRandom;
use rand::Rng;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use tracing::{error, info};

use crate::utils::read_config::load_external_config;

#[derive(Args)]
#[command(about)]
/// Generate voters command arguments
pub struct GenerateVoters {
    /// Working directory for input/output
    #[arg(long)]
    working_directory: String,

    /// Number of users to generate
    #[arg(long)]
    num_users: usize,
}

impl GenerateVoters {
    /// Execute the rendering process
    pub fn run(&self) {
        match Self::run_generate_voters(&self.working_directory, self.num_users) {
            Ok(()) => info!("{}", "Successfully generated voters into csv".green()),
            Err(err) => error!("Error! Failed to generate voters: {err:?}"),
        }
    }

    /// Generate fake date of birth
    fn generate_fake_dob(min_age: i64, max_age: i64) -> NaiveDate {
        let today = Utc::now().date_naive();

        let min_age_days = min_age.checked_mul(365).unwrap_or(0);
        let max_age_days = max_age.checked_mul(365).unwrap_or(0);

        let max_date = today
            .checked_sub_signed(Duration::days(min_age_days))
            .unwrap_or(today);

        let min_date = today
            .checked_sub_signed(Duration::days(max_age_days))
            .unwrap_or(today);

        let days_diff = max_date.signed_duration_since(min_date).num_days();

        let days_diff = days_diff.max(0);

        let random_days = rand::thread_rng().gen_range(0..=days_diff);

        min_date
            .checked_add_signed(Duration::days(random_days))
            .unwrap_or(min_date)
    }

    /// Deduplicate items while preserving order.
    fn deduplicate_preserve_order<T: std::hash::Hash + Eq + Clone>(items: &[T]) -> Vec<T> {
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for item in items {
            if seen.insert(item.clone()) {
                result.push(item.clone());
            }
        }
        result
    }

    /// Generate voters
    #[allow(clippy::too_many_lines)]
    fn run_generate_voters(working_dir: &str, num_users: usize) -> Result<()> {
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
        let authorized_elections_count = voters_config.authorized_elections_count;
        let email_verified = voters_config.email_verified;

        // Parse election event file parts.
        let areas: &[serde_json::Value] = election_data
            .get("areas")
            .and_then(Value::as_array)
            .map_or(&[], |v| v.as_slice());

        let area_contests: &[serde_json::Value] = election_data
            .get("area_contests")
            .and_then(Value::as_array)
            .map_or(&[], |v| v.as_slice());

        let contests: &[serde_json::Value] = election_data
            .get("contests")
            .and_then(Value::as_array)
            .map_or(&[], |v| v.as_slice());

        let elections: &[serde_json::Value] = election_data
            .get("elections")
            .and_then(Value::as_array)
            .map_or(&[], |v| v.as_slice());

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
                        uprovs.as_array().cloned().unwrap_or_default()
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
                                                                        let mut iter =
                                                                            opt_str.splitn(2, '/');

                                                                        let first = iter
                                                                            .next()
                                                                            .unwrap_or("");
                                                                        let second = iter
                                                                            .next()
                                                                            .unwrap_or("");

                                                                        cou_emb_dict.insert(
                                                                            second.to_lowercase(),
                                                                            (
                                                                                first
                                                                                    .trim()
                                                                                    .to_string(),
                                                                                second
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

        let mut area_cycle = areas.iter().cycle();

        for (i, username_counter) in (0..num_users).enumerate() {
            let area = area_cycle.next().unwrap_or(&Value::Null);
            let area_id = area.get("id").and_then(Value::as_str).unwrap_or("Unknown");
            let area_name = area
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Unknown");

            let assigned_cids = area_contest_map
                .get(area_id)
                .map_or(&[][..], |v| v.as_slice());

            let mut election_aliases = Vec::new();
            let mut precincts = Vec::new();

            for cid in assigned_cids {
                let unknown_e_id = "Unknown".to_string();
                let e_id = contest_election_map.get(cid).unwrap_or(&unknown_e_id);
                let default_value = (String::from("Unknown"), String::from("Unknown"));
                let (alias, cluster_prec) = election_map.get(e_id).unwrap_or(&default_value);
                election_aliases.push(alias.clone());
                precincts.push(cluster_prec.clone());
            }
            election_aliases = Self::deduplicate_preserve_order(&election_aliases);
            precincts = Self::deduplicate_preserve_order(&precincts);

            let election_country_candidate = if let Some(first_alias) = election_aliases.first() {
                if first_alias.contains(" - ") {
                    first_alias
                        .split(" - ")
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
            let joined_aliases = if election_aliases.is_empty() {
                "Unknown".to_string()
            } else if authorized_elections_count > 0 {
                let amount = std::cmp::min(
                    usize::try_from(authorized_elections_count).unwrap_or(0),
                    election_aliases.len(),
                );
                election_aliases
                    .choose_multiple(&mut rand::thread_rng(), amount)
                    .cloned()
                    .collect::<Vec<String>>()
                    .join("|")
            } else {
                election_aliases.join("|")
            };
            let joined_precincts = if precincts.is_empty() {
                "Unknown".to_string()
            } else {
                precincts.join("|")
            };

            let dob = Self::generate_fake_dob(min_age, max_age);
            let dob_str = dob.format("%Y-%m-%d").to_string();

            let i_64 = i64::try_from(i).unwrap_or(0);
            let email = if sequence_email_number {
                format!(
                    "{}+{}@{}",
                    email_prefix,
                    i_64.checked_add(sequence_start_number).unwrap_or(0),
                    domain
                )
            } else {
                let random_num: u32 = rand::random::<u32>()
                    .wrapping_add(100_000)
                    .wrapping_rem(900_000_000);
                format!("{email_prefix}+{random_num}@{domain}")
            };

            // Instead of storing the user record in a vector, we build the CSV record directly.
            let mut record = Vec::with_capacity(final_fields.len());
            // For each expected field, extract its value from our generated data.
            for field in &final_fields {
                let value = match field.as_str() {
                    "username" => username_counter.to_string(),
                    "first_name" => FirstName(EN).fake(),
                    "last_name" => LastName(EN).fake(),
                    "dateOfBirth" => dob_str.clone(),
                    "sex" => {
                        if rand::random::<bool>() {
                            "M".to_string()
                        } else {
                            "F".to_string()
                        }
                    }
                    "country" => format!("{official_country}/{official_embassy}"),
                    "embassy" => official_embassy.clone(),
                    "clusteredPrecinct" => joined_precincts.clone(),
                    "overseasReferences" => overseas_reference.clone(),
                    "area_name" => area_name.to_string(),
                    "authorized-election-ids" => joined_aliases.clone(),
                    "password" => voter_password.clone(),
                    "email" => email.clone(),
                    "password_salt" => password_salt.clone(),
                    "hashed_password" => hashed_password.clone(),
                    "email_verified" => email_verified.to_string(),
                    _ => String::new(), // default empty if field not recognized or its middlename
                };
                record.push(value);
            }

            // Write the record to the CSV file.
            wtr.write_record(&record)?;

            // Optionally, log progress every so often rather than every record.
            if i % 10000 == 0 {
                info!("Generated {i} users...");
            }
        }
        wtr.flush()?;

        info!(
            "Successfully generated {} users. CSV file created at: {}",
            num_users,
            csv_file_path.canonicalize()?.display()
        );
        Ok(())
    }
}

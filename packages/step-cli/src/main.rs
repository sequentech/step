// // SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
// //
// // SPDX-License-Identifier: AGPL-3.0-only

// mod commands;
// mod tests;
// mod types;
// mod utils;

// use clap::{Parser, Subcommand};

// #[derive(Parser)]
// #[command(
//     name = "seq",
//     version = "1.0",
//     about = "CLI tool for managing Sequent tasks"
// )]
// struct Cli {
//     #[command(subcommand)]
//     command: MainCommand,
// }

// #[derive(Subcommand)]
// enum MainCommand {
//     #[command(subcommand)]
//     Step(StepCommands),
// }

// #[derive(Subcommand)]
// enum StepCommands {
//     Config(commands::configure::Config),
//     CreateElectionEvent(commands::create_election_event::CreateElectionEventCLI),
//     CreateElection(commands::create_election::CreateElection),
//     CreateContest(commands::create_contest::CreateContest),
//     CreateCandidate(commands::create_candidate::CreateCandidate),
//     CreateArea(commands::create_area::CreateArea),
//     CreateAreaContest(commands::create_area_contest::CreateAreaContest),
//     CreateVoter(commands::create_voter::CreateVoter),
//     UpdateVoter(commands::update_voter::UpdateVoter),
//     UpdateElectionEventStatus(commands::update_election_event_status::UpdateElectionEventStatus),
//     UpdateElectionStatus(commands::update_election_status::UpdateElectionStatus),
//     ImportElection(commands::import_election_event::ImportElectionEventFile),
//     Publish(commands::publish_changes::PublishChanges),
//     RefreshToken(commands::refresh_token::Refresh),
//     StartKeyCeremony(commands::start_key_ceremony::StartKeyCeremony),
//     CompleteKeyCeremony(commands::complete_key_ceremony::Complete),
//     StartTally(commands::start_tally::StartTallyCeremony),
//     UpdateTally(commands::update_tally_status::UpdateTallyStatus),
//     ConfirmKeyTally(commands::confirm_tally_ceremoney_key::ConfirmKeyForTally),
//     RenderTemplate(commands::render_template::RenderTemplate),
// }

// fn main() {
//     let cli = Cli::parse();

//     match &cli.command {
//         MainCommand::Step(step_cmd) => match step_cmd {
//             StepCommands::Config(cmd) => cmd.run(),
//             StepCommands::CreateElectionEvent(create_event) => create_event.run(),
//             StepCommands::CreateElection(create_election) => create_election.run(),
//             StepCommands::CreateContest(create_contest) => create_contest.run(),
//             StepCommands::CreateCandidate(create_candidate) => create_candidate.run(),
//             StepCommands::CreateArea(create_area) => create_area.run(),
//             StepCommands::CreateAreaContest(create_area_contest) => create_area_contest.run(),
//             StepCommands::UpdateElectionEventStatus(update_event) => update_event.run(),
//             StepCommands::UpdateElectionStatus(update_election) => update_election.run(),
//             StepCommands::ImportElection(import) => import.run(),
//             StepCommands::CreateVoter(create_voter) => create_voter.run(),
//             StepCommands::UpdateVoter(update_voter) => update_voter.run(),
//             StepCommands::Publish(publish_ballot) => publish_ballot.run(),
//             StepCommands::RefreshToken(refresh) => refresh.run(),
//             StepCommands::StartKeyCeremony(start) => start.run(),
//             StepCommands::CompleteKeyCeremony(complete) => complete.run(),
//             StepCommands::StartTally(start) => start.run(),
//             StepCommands::UpdateTally(update) => update.run(),
//             StepCommands::ConfirmKeyTally(confirm) => confirm.run(),
//             StepCommands::RenderTemplate(render) => render.run(),
//         },
//     }
// }

// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// mod commands;
// mod tests;
// mod types;
// mod utils;

// use clap::{Parser, Subcommand};

// #[derive(Parser)]
// #[command(
//     name = "seq",
//     version = "1.0",
//     about = "CLI tool for managing Sequent tasks"
// )]
// struct Cli {
//     #[command(subcommand)]
//     command: MainCommand,
// }

// #[derive(Subcommand)]
// enum MainCommand {
//     #[command(subcommand)]
//     Step(StepCommands),
// }

// #[derive(Subcommand)]
// enum StepCommands {
//     Config(commands::configure::Config),
//     CreateElectionEvent(commands::create_election_event::CreateElectionEventCLI),
//     CreateElection(commands::create_election::CreateElection),
//     CreateContest(commands::create_contest::CreateContest),
//     CreateCandidate(commands::create_candidate::CreateCandidate),
//     CreateArea(commands::create_area::CreateArea),
//     CreateAreaContest(commands::create_area_contest::CreateAreaContest),
//     CreateVoter(commands::create_voter::CreateVoter),
//     UpdateVoter(commands::update_voter::UpdateVoter),
//     UpdateElectionEventStatus(commands::update_election_event_status::UpdateElectionEventStatus),
//     UpdateElectionStatus(commands::update_election_status::UpdateElectionStatus),
//     ImportElection(commands::import_election_event::ImportElectionEventFile),
//     Publish(commands::publish_changes::PublishChanges),
//     RefreshToken(commands::refresh_token::Refresh),
//     StartKeyCeremony(commands::start_key_ceremony::StartKeyCeremony),
//     CompleteKeyCeremony(commands::complete_key_ceremony::Complete),
//     StartTally(commands::start_tally::StartTallyCeremony),
//     UpdateTally(commands::update_tally_status::UpdateTallyStatus),
//     ConfirmKeyTally(commands::confirm_tally_ceremoney_key::ConfirmKeyForTally),
//     RenderTemplate(commands::render_template::RenderTemplate),
// }

// fn main() {
//     let cli = Cli::parse();

//     match &cli.command {
//         MainCommand::Step(step_cmd) => match step_cmd {
//             StepCommands::Config(cmd) => cmd.run(),
//             StepCommands::CreateElectionEvent(create_event) => create_event.run(),
//             StepCommands::CreateElection(create_election) => create_election.run(),
//             StepCommands::CreateContest(create_contest) => create_contest.run(),
//             StepCommands::CreateCandidate(create_candidate) => create_candidate.run(),
//             StepCommands::CreateArea(create_area) => create_area.run(),
//             StepCommands::CreateAreaContest(create_area_contest) => create_area_contest.run(),
//             StepCommands::UpdateElectionEventStatus(update_event) => update_event.run(),
//             StepCommands::UpdateElectionStatus(update_election) => update_election.run(),
//             StepCommands::ImportElection(import) => import.run(),
//             StepCommands::CreateVoter(create_voter) => create_voter.run(),
//             StepCommands::UpdateVoter(update_voter) => update_voter.run(),
//             StepCommands::Publish(publish_ballot) => publish_ballot.run(),
//             StepCommands::RefreshToken(refresh) => refresh.run(),
//             StepCommands::StartKeyCeremony(start) => start.run(),
//             StepCommands::CompleteKeyCeremony(complete) => complete.run(),
//             StepCommands::StartTally(start) => start.run(),
//             StepCommands::UpdateTally(update) => update.run(),
//             StepCommands::ConfirmKeyTally(confirm) => confirm.run(),
//             StepCommands::RenderTemplate(render) => render.run(),
//         },
//     }
// }

// TODO: MOVE ACTIONS TO COMMANDS FOLDER

use anyhow::{Context, Ok, Result};
use chrono::{Local, Utc};
use clap::{Parser, Subcommand};
use csv::Writer;
use electoral_log::messages::message::{Message, Sender};
use electoral_log::messages::newtypes::EventIdString;
use electoral_log::messages::statement::{
    Statement, StatementBody, StatementEventType, StatementHead, StatementLogType, StatementType,
};
use electoral_log::ElectoralLogMessage;
use fake::faker::internet::raw::Username;
use fake::faker::name::raw::{FirstName, LastName};
use fake::faker::number::raw::NumberWithFormat;
use fake::locales::EN;
use fake::Fake;
use rand::seq::SliceRandom;
use serde_json::Value;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::{env, u8};

use strand::signature::{StrandSignature, StrandSignaturePk, StrandSignatureSk};

use chrono::{Duration, NaiveDate};
use rand::Rng;

use tokio::runtime::Runtime as TokioRuntime;

use deadpool_postgres::{Config as PgConfig, Pool, Runtime};
use tokio_postgres::NoTls;
use uuid::Uuid;

use immudb_rs::{
    sql_value::Value as ImmudbValue, Client as ImmudbClient, NamedParam, SqlValue, TxMode,
};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Working directory (input/output)
    #[arg(long)]
    working_directory: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate random voters CSV file
    GenerateVoters {
        #[arg(long)]
        num_users: usize,
    },
    /// Duplicate cast votes in the database
    DuplicateVotes {
        #[arg(long)]
        num_votes: usize,
    },
    /// Generate applications in different states
    GenerateApplications {
        #[arg(long)]
        num_applications: usize,
        #[arg(long, default_value = "PENDING")]
        status: String,
        /// Optional verification type: AUTOMATIC or MANUAL
        #[arg(long)]
        r#type: Option<String>,
    },
    /// Generate activity logs
    GenerateActivityLogs {
        #[arg(long)]
        num_logs: usize,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    // Create a new Tokio runtime.
    let rt = TokioRuntime::new().expect("Failed to create runtime");

    match cli.command {
        Commands::GenerateVoters { num_users } => {
            // run_generate_voters is synchronous.
            run_generate_voters(&cli.working_directory, num_users)?
        }
        Commands::DuplicateVotes { num_votes } => {
            // Use the runtime to block on the async function.
            rt.block_on(run_duplicate_votes(&cli.working_directory, num_votes))?;
        }
        Commands::GenerateApplications {
            num_applications,
            status,
            r#type,
        } => {
            rt.block_on(run_generate_applications(
                &cli.working_directory,
                num_applications,
                &status,
                r#type,
            ))?;
        }
        Commands::GenerateActivityLogs { num_logs } => {
            rt.block_on(run_generate_activity_logs(&cli.working_directory, num_logs))?;
        }
    }
    Ok(())
}

fn generate_fake_dob(min_age: i64, max_age: i64) -> NaiveDate {
    // Get today's date
    let today = Utc::today().naive_utc();
    // Calculate the latest possible DOB (i.e. the person is min_age today)
    let max_date = today - Duration::days(min_age * 365);
    // Calculate the earliest possible DOB (i.e. the person is max_age today)
    let min_date = today - Duration::days(max_age * 365);
    // Get the total number of days between the two dates
    let days_diff = (max_date - min_date).num_days();
    // Generate a random number of days to add to min_date
    let random_days = rand::thread_rng().gen_range(0..=days_diff);
    min_date + Duration::days(random_days)
}

/// Load a JSON config from the working directory.
fn load_config(working_dir: &str) -> Result<Value> {
    let config_path = PathBuf::from(working_dir).join("config.json");
    let file = File::open(config_path)?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;
    Ok(config)
}

async fn get_hasura_pool() -> Result<Pool> {
    let mut cfg = PgConfig::default();
    cfg.host = Some(std::env::var("HASURA_PG_HOST")?);
    cfg.port = Some(std::env::var("HASURA_PG_PORT")?.parse::<u16>()?);
    cfg.user = Some(std::env::var("HASURA_PG_USER")?);
    cfg.password = Some(std::env::var("HASURA_PG_PASSWORD")?);
    cfg.dbname = Some(std::env::var("HASURA_PG_DBNAME")?);
    Ok(cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
}

async fn get_keyckloak_pool() -> Result<Pool> {
    let mut kc_cfg = PgConfig::default();
    kc_cfg.host = Some(std::env::var("KC_DB_URL_HOST")?);
    kc_cfg.port = Some(std::env::var("KC_DB_URL_PORT")?.parse::<u16>()?);
    kc_cfg.user = Some(std::env::var("KC_DB_USERNAME")?);
    kc_cfg.password = Some(std::env::var("KC_DB_PASSWORD")?);
    kc_cfg.dbname = Some(std::env::var("KC_DB")?);
    Ok(kc_cfg.create_pool(Some(Runtime::Tokio1), NoTls)?)
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

fn run_generate_voters(working_dir: &str, num_users: usize) -> Result<()> {
    // Load config file.
    let config = load_config(working_dir)?;

    // Get election event file path from config (or default).
    let election_event_file = config
        .get("election_event_json_file")
        .and_then(Value::as_str)
        .unwrap_or("export_election_event.json");
    let election_event_path = PathBuf::from(working_dir).join(election_event_file);
    let election_file = File::open(election_event_path)?;
    let election_data: Value = serde_json::from_reader(BufReader::new(election_file))?;

    // Get voters configuration with defaults.
    let voters_config = config.get("generate_voters").unwrap_or(&Value::Null);
    let csv_file_name = format!(
        "{}_{}.csv",
        voters_config
            .get("csv_file_name")
            .and_then(Value::as_str)
            .unwrap_or("generated_users"),
        num_users
    );
    let csv_file_path = PathBuf::from(working_dir).join(&csv_file_name);

    // Fields and excluded columns.
    let fields: Vec<String> = voters_config
        .get("fields")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| Some(v.as_str().map(String::from)).unwrap())
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                "username",
                "last_name",
                "first_name",
                "middleName",
                "dateOfBirth",
                "sex",
                "country",
                "embassy",
                "clusteredPrecinct",
                "overseasReferences",
                "area_name",
                "authorized-election-ids",
                "password",
                "email",
                "password_salt",
                "hashed_password",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect()
        });
    let excluded_columns: HashSet<String> = voters_config
        .get("excluded_columns")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| Some(v.as_str().map(String::from)).unwrap())
                .collect()
        })
        .unwrap_or_else(|| ["password"].iter().map(|s| s.to_string()).collect());

    let email_prefix = voters_config
        .get("email_prefix")
        .and_then(Value::as_str)
        .unwrap_or("testsequent2025");
    let domain = voters_config
        .get("domain")
        .and_then(Value::as_str)
        .unwrap_or("mailinator.com");
    let sequence_email_number = voters_config
        .get("sequence_email_number")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let sequence_start_number = voters_config
        .get("sequence_start_number")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let voter_password = voters_config
        .get("voter_password")
        .and_then(Value::as_str)
        .unwrap_or("Qwerty1234!");
    let password_salt = voters_config
        .get("password_salt")
        .and_then(Value::as_str)
        .unwrap_or("sppXH6/iePtmIgcXfTHmjPS2QpLfILVMfmmVOLPKlic=");
    let hashed_password = voters_config
        .get("hashed_password")
        .and_then(Value::as_str)
        .unwrap_or("V0rb8+HmTneV64qto5f0G2+OY09x2RwPeqtK605EUz0=");
    let min_age = voters_config
        .get("min_age")
        .and_then(Value::as_u64)
        .unwrap_or(18) as i64;
    let max_age = voters_config
        .get("max_age")
        .and_then(Value::as_u64)
        .unwrap_or(90) as i64;
    let overseas_reference = voters_config
        .get("overseas_reference")
        .and_then(Value::as_str)
        .unwrap_or("B");

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

    // Build election map: election.id -> (alias, clusteredPrecinct)
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
            if let Some(uprovs) = components.get("org.keycloak.userprofile.UserProfileProvider") {
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
                            if let Some(raw_json_str) = kc_conf_list.first().and_then(Value::as_str)
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
                                                if let Some(validations) = at.get("validations") {
                                                    if let Some(options) = validations
                                                        .get("options")
                                                        .and_then(|o| o.get("options"))
                                                        .and_then(Value::as_array)
                                                    {
                                                        for opt in options {
                                                            if let Some(opt_str) = opt.as_str() {
                                                                if opt_str.contains('/') {
                                                                    let parts: Vec<&str> = opt_str
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
                                                                            "Unknown".to_string(),
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

    // Prepare to generate users.
    let mut users: Vec<serde_json::Map<String, Value>> = Vec::new();
    let mut username_counter = 20001;
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

        let mut election_aliases: Vec<String> = Vec::new();
        let mut precincts: Vec<String> = Vec::new();

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
        election_aliases = deduplicate_preserve_order(&election_aliases);
        precincts = deduplicate_preserve_order(&precincts);

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

        let dob = generate_fake_dob(18, 90);
        let dob_str = dob.format("%Y-%m-%d").to_string();

        let email = if sequence_email_number {
            format!(
                "{}+{}@{}",
                email_prefix,
                i as u64 + 20000 + sequence_start_number,
                domain
            )
        } else {
            let random_num: u32 = rand::random::<u32>() % 900_000_000 + 100_000;
            format!("{}+{}@{}", email_prefix, random_num, domain)
        };

        let mut user_record = serde_json::Map::new();
        user_record.insert(
            "username".to_string(),
            Value::String(username_counter.to_string()),
        );
        user_record.insert(
            "first_name".to_string(),
            Value::String(FirstName(EN).fake()),
        );
        user_record.insert("last_name".to_string(), Value::String(LastName(EN).fake()));
        user_record.insert("middleName".to_string(), Value::String(String::new()));
        user_record.insert("dateOfBirth".to_string(), Value::String(dob_str));
        let sex = if *[true, false].choose(&mut rand::thread_rng()).unwrap() {
            "M"
        } else {
            "F"
        };
        user_record.insert("sex".to_string(), Value::String(sex.to_string()));
        user_record.insert(
            "country".to_string(),
            Value::String(format!("{}/{}", official_country, official_embassy)),
        );
        user_record.insert("embassy".to_string(), Value::String(official_embassy));
        user_record.insert(
            "clusteredPrecinct".to_string(),
            Value::String(joined_precincts),
        );
        user_record.insert(
            "overseasReferences".to_string(),
            Value::String(overseas_reference.to_string()),
        );
        user_record.insert(
            "area_name".to_string(),
            Value::String(area_name.to_string()),
        );
        user_record.insert(
            "authorized-election-ids".to_string(),
            Value::String(joined_aliases),
        );
        user_record.insert(
            "password".to_string(),
            Value::String(voter_password.to_string()),
        );
        user_record.insert("email".to_string(), Value::String(email));
        user_record.insert(
            "password_salt".to_string(),
            Value::String(password_salt.to_string()),
        );
        user_record.insert(
            "hashed_password".to_string(),
            Value::String(hashed_password.to_string()),
        );

        users.push(user_record);
        username_counter += 1;
    }

    let final_fields: Vec<String> = fields
        .into_iter()
        .filter(|f| !excluded_columns.contains(f))
        .collect();
    let mut wtr = Writer::from_path(&csv_file_path)?;
    wtr.write_record(&final_fields)?;
    for user in users {
        let mut record = Vec::new();
        for field in &final_fields {
            let value = user.get(field).and_then(Value::as_str).unwrap_or("");
            record.push(value);
        }
        wtr.write_record(&record)?;
    }
    wtr.flush()?;

    println!(
        "Successfully generated {} users. CSV file created at: {}",
        num_users,
        csv_file_path.canonicalize()?.display()
    );
    Ok(())
}

pub async fn run_duplicate_votes(working_dir: &str, num_votes: usize) -> Result<()> {
    // --- Load configuration ---
    let config = load_config(working_dir)?;
    let realm_name = config
        .get("realm_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let duplicate_votes_config = config.get("duplicate_votes").unwrap_or(&Value::Null);
    let row_id_to_clone = duplicate_votes_config
        .get("row_id_to_clone")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("Missing row_id_to_clone in config"))?;

    let kc_client = get_keyckloak_pool()
        .await?
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

    let keycloak_query = "\
        SELECT ue.id FROM user_entity AS ue \
        JOIN realm AS r ON ue.realm_id = r.id \
        WHERE r.name = $1 LIMIT $2 OFFSET 0";

    let kc_rows = kc_client
        .query(keycloak_query, &[&realm_name, &(num_votes as i64)])
        .await?;
    let existing_user_ids: Vec<String> = kc_rows
        .iter()
        .filter_map(|row| row.get::<_, Option<String>>(0))
        .collect();
    println!("Number of existing user IDs::: {}", existing_user_ids.len());

    // --- Get Hasura client and begin a transaction ---
    let mut hasura_db_client = get_hasura_pool()
        .await?
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

    let mut hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| format!("Error starting hasura transaction: {}", e))
        .map_err(|e| anyhow::anyhow!(e))?;

    // --- Query the base vote to clone ---
    let base_query = "\
    SELECT tenant_id, election_event_id, election_id, area_id, annotations, content, cast_ballot_signature, ballot_id \
        FROM sequent_backend.cast_vote WHERE id = $1";
    let base_row = hasura_transaction
        .query_opt(base_query, &[&Uuid::parse_str(row_id_to_clone)?])
        .await?;
    if base_row.is_none() {
        println!("No row found to clone.");
        return Ok(());
    }
    let row = base_row.unwrap();
    let tenant_id = row.try_get::<_, Uuid>(0)?;
    let election_event_id = row.try_get::<_, Uuid>(1)?;
    let election_id = row.try_get::<_, Uuid>(2)?;
    let area_id = row.try_get::<_, Uuid>(3)?;
    let annotations: serde_json::Value = row.get(4);
    let content: &str = row.get(5);
    let cast_ballot_signature: Vec<u8> = row.get(6);
    let ballot_id: &str = row.get(7);

    println!("Start insetring votes at: {:?}", Local::now());
    // --- Build batched INSERT queries ---
    // Define a batch size (adjust as needed)
    let batch_size = 1000;
    for batch in existing_user_ids.chunks(batch_size) {
        let mut query = String::from("INSERT INTO sequent_backend.cast_vote (voter_id_string, election_id, tenant_id, area_id, annotations, content, cast_ballot_signature, election_event_id, ballot_id) VALUES ");
        let mut params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut placeholder_idx = 1;
        for (i, uid) in batch.iter().enumerate() {
            if i > 0 {
                query.push_str(", ");
            }
            // For each row, create placeholders for 9 parameters.
            let mut placeholders = Vec::new();
            for _ in 0..9 {
                placeholders.push(format!("${}", placeholder_idx));
                placeholder_idx += 1;
            }
            query.push_str("(");
            query.push_str(&placeholders.join(", "));
            query.push_str(")");

            params.push(uid);
            params.push(&election_id);
            params.push(&tenant_id);
            params.push(&area_id);
            params.push(&annotations);
            params.push(&content);
            params.push(&cast_ballot_signature);
            params.push(&election_event_id);
            params.push(&ballot_id);
        }
        hasura_transaction.execute(query.as_str(), &params).await?;
    }

    hasura_transaction.commit().await?;
    println!("End insetring votes at: {:?}", Local::now());
    println!("Inserted {} duplicate votes.", &existing_user_ids.len());
    Ok(())
}

pub async fn run_generate_applications(
    working_dir: &str,
    num_applications: usize,
    status: &str,
    verification_type: Option<String>,
) -> Result<()> {
    // --- Load configuration ---
    let config = load_config(working_dir)?;
    let realm_name = config
        .get("realm_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let tenant_id = config
        .get("tenant_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let election_event_id = config
        .get("election_event_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let generate_applications_config = config.get("generate_applications").unwrap_or(&Value::Null);
    let default_applicant_data = generate_applications_config
        .get("applicant_data")
        .cloned()
        .unwrap_or(Value::Object(serde_json::Map::new()));
    let annotations = generate_applications_config
        .get("annotations")
        .cloned()
        .unwrap_or(Value::Null);

    // --- Build Keycloak Pool ---
    let kc_client = get_keyckloak_pool()
        .await?
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

    // --- Get Hasura client and begin a transaction ---
    let mut hasura_db_client = get_hasura_pool()
        .await?
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

    let mut hasura_transaction = hasura_db_client
        .transaction()
        .await
        .map_err(|e| format!("Error starting hasura transaction: {}", e))
        .map_err(|e| anyhow::anyhow!(e))?;

    println!("Number of rows to clone: {}", num_applications);

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
    let rows = kc_client
        .query(query, &[&realm_name, &(num_applications as i64)])
        .await?;
    println!("Number of existing user rows: {}", rows.len());

    let verification_type = match verification_type {
        Some(vt) => vt,
        None => {
            if status == "PENDING" {
                "MANUAL".to_string()
            } else if *[true, false].choose(&mut rand::thread_rng()).unwrap() {
                "AUTOMATIC".to_string()
            } else {
                "MANUAL".to_string()
            }
        }
    };

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
    let mut rows_params: Vec<Vec<Box<dyn tokio_postgres::types::ToSql + Sync>>> = Vec::new();
    for row in rows {
        let user_id: String = row.get(0);
        let username: String = row.get(1);
        let email: String = row.get(2);
        let first_name: String = row.get(3);
        let last_name: String = row.get(4);
        let area_id_opt: Option<String> = row.get(5);
        let country: Option<String> = row.get(6);
        let embassy: Option<String> = row.get(7);
        let date_of_birth: Option<String> = row.get(8);

        // Merge default applicant data with user details.
        let mut applicant_data = if let Value::Object(map) = default_applicant_data.clone() {
            map
        } else {
            serde_json::Map::new()
        };
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
        // Generate a random ID card number using NumberWithFormat.
        let id_card_number = format!("C{}", NumberWithFormat(EN, "##########").fake::<String>());
        applicant_data.insert(
            "sequent.read-only.id-card-number".to_string(),
            Value::String(id_card_number),
        );
        // Instead of converting applicant_data to a String, we keep it as a serde_json::Value.
        let applicant_data_value = Value::Object(applicant_data);
        let area_id_value = area_id_opt.unwrap_or_default();
        let area_id = area_id_value.as_str();
        // For annotations, we already have a Value.
        rows_params.push(vec![
            Box::new(user_id) as Box<dyn tokio_postgres::types::ToSql + Sync>,
            Box::new(status.to_string()),
            Box::new(verification_type.clone()),
            Box::new(applicant_data_value),
            Box::new(Uuid::parse_str(tenant_id)?),
            Box::new(Uuid::parse_str(election_event_id)?),
            Box::new(Uuid::parse_str(area_id)?),
            Box::new(annotations.clone()),
        ]);
    }

    println!(
        "Number of application rows to insert: {}",
        rows_params.len()
    );

    // --- Batch the INSERTs ---
    let batch_size = 1000;
    for batch in rows_params.chunks(batch_size) {
        // Build the INSERT query with multiple rows.
        // There are 8 columns per row.
        let mut query = String::from("INSERT INTO sequent_backend.applications (applicant_id, status, verification_type, applicant_data, tenant_id, election_event_id, area_id, annotations) VALUES ");
        let mut placeholders = Vec::new();
        // Build a vector of parameters as trait objects.
        let mut flat_params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = Vec::new();
        let mut param_index = 1;
        for row in batch {
            let mut row_placeholders = Vec::new();
            for (i, value) in row.iter().enumerate() {
                // For applicant_data (index 3) and annotations (index 7), add an explicit cast to jsonb.
                if i == 3 || i == 7 {
                    row_placeholders.push(format!("${}::jsonb", param_index));
                } else {
                    row_placeholders.push(format!("${}", param_index));
                }
                param_index += 1;
                flat_params.push(&**value);
            }
            placeholders.push(format!("({})", row_placeholders.join(", ")));
        }
        query.push_str(&placeholders.join(", "));
        hasura_transaction
            .execute(query.as_str(), &flat_params)
            .await?;
    }

    hasura_transaction.commit().await?;
    println!(
        "Successfully inserted {} application rows.",
        rows_params.len()
    );
    Ok(())
}

pub async fn get_immudb_client() -> Result<ImmudbClient> {
    let username = env::var("IMMUDB_USER")?;
    let password = env::var("IMMUDB_PASSWORD")?;
    let server_url = env::var("IMMUDB_SERVER_URL")?;

    let mut client = ImmudbClient::new(&server_url, &username, &password).await?;
    client.login().await?;

    Ok(client)
}

pub fn get_event_board(tenant_id: &str, election_event_id: &str) -> String {
    let tenant: String = tenant_id
        .to_string()
        .chars()
        .filter(|&c| c != '-')
        .take(17)
        .collect();
    let event: String = election_event_id
        .to_string()
        .chars()
        .filter(|&c| c != '-')
        .collect();

    format!("tenant{}event{}", tenant, event)
        .chars()
        .filter(|&c| c != '-')
        .collect()
}

fn generate_log_message(
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
    };

    let board_message: ElectoralLogMessage = message.try_into().with_context(|| "")?;
    Ok(board_message)
}

async fn run_generate_activity_logs(working_dir: &str, num_logs: usize) -> Result<()> {
    // --- Load configuration ---
    let config = load_config(working_dir)?;
    let tenant_id = config
        .get("tenant_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let election_event_id = config
        .get("election_event_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let election_id = config
        .get("election_id")
        .and_then(Value::as_str)
        .unwrap_or("");
    let immudb_db = get_event_board(&tenant_id, &election_event_id);
    let area_id = config.get("area_id").and_then(Value::as_str).unwrap_or("");
    let realm_name = config
        .get("realm_name")
        .and_then(Value::as_str)
        .unwrap_or("");

    println!("immudb_db: {}", &immudb_db);

    let kc_client = get_keyckloak_pool()
        .await?
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Error getting hasura client: {}", e.to_string()))?;

    let keycloak_query = "\
        SELECT ue.id FROM user_entity AS ue \
        JOIN realm AS r ON ue.realm_id = r.id \
        JOIN user_attribute AS ua ON ue.id = ua.user_id \
        WHERE r.name = $1 AND ua.name = 'area-id' AND ua.value = $2 \
        LIMIT 1 OFFSET 0";

    let kc_row = kc_client
        .query_one(keycloak_query, &[&realm_name, &area_id])
        .await?;
    let existing_user_id = kc_row.get::<_, Option<String>>(0);

    let user_id: Option<String> =
        Some(existing_user_id.unwrap_or(uuid::Uuid::new_v4().to_string()));

    let mut logs_params: Vec<Vec<NamedParam>> = Vec::new();
    for _ in 0..num_logs {
        let username = Some(Username(EN).fake());
        let user_id_cloned = user_id.clone();

        let message: ElectoralLogMessage = generate_log_message(
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

    let mut client: ImmudbClient = get_immudb_client().await?;
    client.open_session(immudb_db.as_str()).await?;

    // We'll batch the logs into groups (e.g., 1000 per batch).
    // We'll use a batch size (e.g., 100 logs per batch)
    let batch_size = 1000;
    for batch in logs_params.chunks(batch_size) {
        // Start a new transaction for this batch.
        let tx_id = client.new_tx(TxMode::ReadWrite).await?;

        // Build the multi-row INSERT query.
        let mut query = String::from("INSERT INTO electoral_log_messages (created, sender_pk, statement_kind, statement_timestamp, message, version, user_id, username, election_id, area_id) VALUES ");
        let mut values_clauses = Vec::new();
        let mut all_params: Vec<NamedParam> = Vec::new();
        let mut row_index = 1;
        for row in batch {
            let mut clause_parts = Vec::new();
            for param in row {
                // Append the row index to the parameter name (e.g. created1, sender_pk1, etc.)
                let new_name = format!("{}{}", param.name, row_index);
                clause_parts.push(format!("@{}", new_name));
                // Add a new parameter with the renamed key.
                all_params.push(NamedParam {
                    name: new_name,
                    value: param.value.clone(),
                });
            }
            row_index += 1;
            values_clauses.push(format!("({})", clause_parts.join(", ")));
        }
        query.push_str(&values_clauses.join(", "));
        // Execute the batched INSERT in this transaction.
        client.tx_sql_exec(&query, &tx_id, all_params).await?;
        // Commit this batch's transaction.
        client.commit(&tx_id).await?;
    }

    client.close_session().await?;
    println!("Inserted {} logs.", num_logs);

    Ok(())
}

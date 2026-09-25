// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use chrono::{Duration, NaiveDate};
use fake::faker::name::raw::{FirstName, LastName};
use fake::locales::EN;
use fake::Fake;
use rand::seq::IndexedRandom;
use rand::Rng;
use sequent_core::util::external_config::{GenerateVoters as VotersConfig, VoterPasswordPolicy};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

const UNKNOWN: &str = "Unknown";
const LIST_SEPARATOR: &str = "|";
const USER_PROFILE_PROVIDER: &str = "org.keycloak.userprofile.UserProfileProvider";
const USER_PROFILE_CONFIG: &str = "kc.user.profile.config";
const COUNTRY_ATTRIBUTE: &str = "country";
const RANDOM_EMAIL_NUMBER_MODULUS: u32 = 900_000_000;
const RANDOM_EMAIL_NUMBER_OFFSET: u32 = 100_000;

/// Age (years above `min_age`) at which the population has halved, under the exponential
/// mortality-decay model `sample_dob` samples from - i.e. voters aged `min_age +
/// AGE_HALF_LIFE_YEARS` are half as common as voters aged exactly `min_age`, voters aged
/// `min_age + 2 * AGE_HALF_LIFE_YEARS` a quarter as common, and so on. Approximates a
/// realistic population pyramid (most populous at the youngest eligible age, tapering off
/// with age) without hard-coding any specific country's census data.
const AGE_HALF_LIFE_YEARS: f64 = 25.0;

static NO_AREA: Value = Value::Null;

/// Country options keyed by lowercased embassy, each with its country and embassy.
pub type CountryEmbassies = HashMap<String, (String, String)>;

struct ElectionLabels {
    alias: String,
    clustered_precinct: String,
}

/// The parts of an exported election event that decide each voter's area,
/// elections and embassy.
pub struct ElectionIndex {
    areas: Vec<Value>,
    area_contests: HashMap<String, Vec<String>>,
    contest_elections: HashMap<String, String>,
    elections: HashMap<String, ElectionLabels>,
    embassies: CountryEmbassies,
}

/// The distinct elections of an area's contests, in contest order. Each list
/// is deduplicated on its own.
pub struct AreaElections<'a> {
    pub aliases: Vec<&'a str>,
    pub ids: Vec<&'a str>,
    pub clustered_precincts: Vec<&'a str>,
}

impl ElectionIndex {
    pub fn from_event(event: &Value) -> Self {
        let list = |key: &str| {
            event
                .get(key)
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[])
        };

        let mut elections = HashMap::new();
        for election in list("elections") {
            if let Some(id) = election.get("id").and_then(Value::as_str) {
                let clustered_precinct = election
                    .get("annotations")
                    .and_then(|annotations| annotations.get("clustered_precint_id"))
                    .and_then(Value::as_str)
                    .unwrap_or(UNKNOWN);
                elections.insert(
                    id.to_string(),
                    ElectionLabels {
                        alias: election_alias(election),
                        clustered_precinct: clustered_precinct.to_string(),
                    },
                );
            }
        }

        let mut area_contests: HashMap<String, Vec<String>> = HashMap::new();
        for area_contest in list("area_contests") {
            if let (Some(area_id), Some(contest_id)) = (
                area_contest.get("area_id").and_then(Value::as_str),
                area_contest.get("contest_id").and_then(Value::as_str),
            ) {
                area_contests
                    .entry(area_id.to_string())
                    .or_default()
                    .push(contest_id.to_string());
            }
        }

        let mut contest_elections = HashMap::new();
        for contest in list("contests") {
            if let Some(id) = contest.get("id").and_then(Value::as_str) {
                let election_id = contest
                    .get("election_id")
                    .and_then(Value::as_str)
                    .unwrap_or(UNKNOWN);
                contest_elections.insert(id.to_string(), election_id.to_string());
            }
        }

        Self {
            areas: list("areas").to_vec(),
            area_contests,
            contest_elections,
            elections,
            embassies: country_embassy_options(event),
        }
    }

    /// The area of the `voter`-th voter: voters are spread over the areas
    /// round-robin, and get a null area when the event has none.
    pub fn area(&self, voter: usize) -> &Value {
        voter
            .checked_rem(self.areas.len())
            .and_then(|position| self.areas.get(position))
            .unwrap_or(&NO_AREA)
    }

    /// A contest missing from the event counts as an election with id, alias
    /// and clustered precinct "Unknown".
    pub fn area_elections(&self, area_id: &str) -> AreaElections<'_> {
        let mut aliases = Vec::new();
        let mut ids = Vec::new();
        let mut clustered_precincts = Vec::new();
        let contests = self
            .area_contests
            .get(area_id)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for contest_id in contests {
            let election_id = self
                .contest_elections
                .get(contest_id)
                .map(String::as_str)
                .unwrap_or(UNKNOWN);
            let (alias, clustered_precinct) = match self.elections.get(election_id) {
                Some(labels) => (labels.alias.as_str(), labels.clustered_precinct.as_str()),
                None => (UNKNOWN, UNKNOWN),
            };
            aliases.push(alias);
            ids.push(election_id);
            clustered_precincts.push(clustered_precinct);
        }
        AreaElections {
            aliases: unique_in_order(aliases),
            ids: unique_in_order(ids),
            clustered_precincts: unique_in_order(clustered_precincts),
        }
    }

    /// The country and embassy of a voter whose first election alias is
    /// `alias`. The alias's part before " - " is looked up, ignoring case,
    /// among the country options; without a match it is taken as the country
    /// and the embassy is "Unknown".
    pub fn country_and_embassy(&self, alias: Option<&str>) -> (String, String) {
        let candidate = alias
            .map(|alias| {
                alias
                    .split_once(" - ")
                    .map_or(alias, |(country, _)| country)
                    .trim()
            })
            .unwrap_or(UNKNOWN);
        self.embassies
            .get(&candidate.to_lowercase())
            .cloned()
            .unwrap_or_else(|| (candidate.to_string(), UNKNOWN.to_string()))
    }
}

/// An election's alias lives at `presentation.i18n.<lang>.alias`, not as a
/// top-level "alias" field — prefers "en", falls back to any other language
/// with an alias set, then to that language's name, then "Unknown".
pub fn election_alias(el: &Value) -> String {
    let Some(i18n) = el
        .get("presentation")
        .and_then(|p| p.get("i18n"))
        .and_then(Value::as_object)
    else {
        return UNKNOWN.to_string();
    };
    let field = |lang: &str, key: &str| {
        i18n.get(lang)
            .and_then(|v| v.get(key))
            .and_then(Value::as_str)
    };
    field("en", "alias")
        .or_else(|| field("en", "name"))
        .or_else(|| {
            i18n.values()
                .find_map(|v| v.get("alias").and_then(Value::as_str))
        })
        .or_else(|| {
            i18n.values()
                .find_map(|v| v.get("name").and_then(Value::as_str))
        })
        .unwrap_or(UNKNOWN)
        .to_string()
}

/// The options of the "country" attribute in the event realm's Keycloak user
/// profile. "Country/Embassy" is keyed by the lowercased embassy; an option
/// without a slash is keyed by itself, lowercased, with an "Unknown" embassy.
pub fn country_embassy_options(event: &Value) -> CountryEmbassies {
    let mut options = CountryEmbassies::new();
    let Some(providers) = event
        .get("keycloak_event_realm")
        .and_then(|realm| realm.get("components"))
        .and_then(|components| components.get(USER_PROFILE_PROVIDER))
    else {
        return options;
    };
    let provider = match providers {
        Value::Array(providers) => providers.first(),
        provider => Some(provider),
    };
    let Some(Ok(user_profile)) = provider
        .and_then(|provider| provider.get("config"))
        .and_then(|config| config.get(USER_PROFILE_CONFIG))
        .and_then(Value::as_array)
        .and_then(|values| values.first())
        .and_then(Value::as_str)
        .map(serde_json::from_str::<Value>)
    else {
        return options;
    };
    let attributes = user_profile
        .get("attributes")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    for attribute in attributes {
        if attribute.get("name").and_then(Value::as_str) != Some(COUNTRY_ATTRIBUTE) {
            continue;
        }
        let choices = attribute
            .get("validations")
            .and_then(|validations| validations.get("options"))
            .and_then(|options| options.get("options"))
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        for choice in choices.iter().filter_map(Value::as_str) {
            match choice.split_once('/') {
                Some((country, embassy)) => options.insert(
                    embassy.to_lowercase(),
                    (country.trim().to_string(), embassy.trim().to_string()),
                ),
                None => options.insert(
                    choice.to_lowercase(),
                    (choice.to_string(), UNKNOWN.to_string()),
                ),
            };
        }
    }
    options
}

/// Samples a date of birth in `[today - max_age years, today - min_age years]`, weighted so
/// younger ages (near `min_age`) are more common than older ones (near `max_age`) - see
/// `AGE_HALF_LIFE_YEARS`. Implemented as inverse-CDF sampling from a truncated exponential
/// distribution over the age range, rather than a uniform pick across the whole span. `u` is
/// a uniform sample from `[0, 1)`.
pub fn sample_dob(u: f64, today: NaiveDate, min_age: i64, max_age: i64) -> NaiveDate {
    let youngest_dob = today - Duration::days(min_age * 365);
    let oldest_dob = today - Duration::days(max_age * 365);
    let days_diff = (youngest_dob - oldest_dob).num_days();

    let lambda = std::f64::consts::LN_2 / (AGE_HALF_LIFE_YEARS * 365.0);
    let extra_age_days = if days_diff <= 0 {
        0
    } else {
        let cdf_at_max = 1.0 - (-lambda * days_diff as f64).exp();
        (-(1.0 - u * cdf_at_max).ln() / lambda) as i64
    };

    // extra_age_days == 0 is the youngest possible voter (DOB == youngest_dob); larger
    // values move further back toward oldest_dob, with the exponential weighting making
    // large values increasingly rare.
    youngest_dob - Duration::days(extra_age_days.min(days_diff))
}

/// The configured fields, in order, without the excluded ones.
pub fn output_columns(cfg: &VotersConfig) -> impl Iterator<Item = &str> {
    cfg.fields
        .iter()
        .filter(|field| !cfg.excluded_columns.contains(field))
        .map(String::as_str)
}

pub fn csv_file_name(cfg: &VotersConfig, num_users: usize) -> String {
    format!("{}_{}.csv", cfg.csv_file_name, num_users)
}

/// The CSV values of the `voter`-th generated voter (counting from zero), one
/// per output column. Unrecognised columns are left empty.
pub fn voter_record(
    voter: usize,
    area: &Value,
    index: &ElectionIndex,
    cfg: &VotersConfig,
    rng: &mut impl Rng,
    today: NaiveDate,
) -> Vec<String> {
    let area_id = area.get("id").and_then(Value::as_str).unwrap_or(UNKNOWN);
    let area_name = area.get("name").and_then(Value::as_str).unwrap_or(UNKNOWN);
    let elections = index.area_elections(area_id);
    let (official_country, official_embassy) =
        index.country_and_embassy(elections.aliases.first().copied());
    // Hasura's ballot-style/eligibility permissions filter
    // election_id _in X-Hasura-Authorized-Election-Ids (see
    // sequent_backend_ballot_style.yaml), so this needs actual
    // election ids — not the human-readable alias used for the
    // country/embassy lookup above.
    let election_ids = authorized_election_ids(&elections.ids, cfg.authorized_elections_count, rng);
    let clustered_precincts = joined_or_unknown(&elections.clustered_precincts);
    let dob = sample_dob(rng.random_range(0.0..1.0), today, cfg.min_age, cfg.max_age)
        .format("%Y-%m-%d")
        .to_string();
    let password = match &cfg.voter_password_policy {
        VoterPasswordPolicy::Fixed => cfg.voter_password.clone(),
        VoterPasswordPolicy::RandomNumeric { digits } => random_numeric_password(*digits, rng),
    };
    let email = voter_email(voter, cfg, rng);

    output_columns(cfg)
        .map(|column| match column {
            "username" => (cfg.username_start_number + voter as i64).to_string(),
            "first_name" => FirstName(EN).fake_with_rng(rng),
            "last_name" => LastName(EN).fake_with_rng(rng),
            "middleName" => String::new(),
            "dateOfBirth" => dob.clone(),
            "sex" => String::from(if rng.random_bool(0.5) { "M" } else { "F" }),
            "country" => format!("{}/{}", official_country, official_embassy),
            "embassy" => official_embassy.clone(),
            "clusteredPrecinct" => clustered_precincts.clone(),
            "overseasReferences" => cfg.overseas_reference.clone(),
            "area_name" => area_name.to_string(),
            "authorized-election-ids" => election_ids.clone(),
            "password" => password.clone(),
            "email" => email.clone(),
            "password_salt" => cfg.password_salt.clone(),
            "hashed_password" => cfg.hashed_password.clone(),
            "email_verified" => cfg.email_verified.to_string(),
            _ => String::new(),
        })
        .collect()
}

/// Up to `count` of the ids, chosen at random, or all of them in order when
/// `count` is not positive.
fn authorized_election_ids(ids: &[&str], count: i64, rng: &mut impl Rng) -> String {
    if ids.is_empty() {
        return UNKNOWN.to_string();
    }
    if count > 0 {
        let amount = std::cmp::min(count as usize, ids.len());
        ids.choose_multiple(rng, amount)
            .copied()
            .collect::<Vec<_>>()
            .join(LIST_SEPARATOR)
    } else {
        ids.join(LIST_SEPARATOR)
    }
}

fn joined_or_unknown(values: &[&str]) -> String {
    if values.is_empty() {
        UNKNOWN.to_string()
    } else {
        values.join(LIST_SEPARATOR)
    }
}

/// Generates a random numeric string of exactly `digits` digits (leading zeros allowed,
/// since a PIN is an opaque digit string, not a number).
fn random_numeric_password(digits: u32, rng: &mut impl Rng) -> String {
    (0..digits)
        .map(|_| char::from(b'0' + rng.random_range(0..10u8)))
        .collect()
}

fn voter_email(voter: usize, cfg: &VotersConfig, rng: &mut impl Rng) -> String {
    if cfg.sequence_email_number {
        format!(
            "{}+{}@{}",
            cfg.email_prefix,
            voter as i64 + cfg.sequence_start_number,
            cfg.domain
        )
    } else {
        let random_num =
            rng.random::<u32>() % RANDOM_EMAIL_NUMBER_MODULUS + RANDOM_EMAIL_NUMBER_OFFSET;
        format!("{}+{}@{}", cfg.email_prefix, random_num, cfg.domain)
    }
}

/// Deduplicate items while preserving order.
fn unique_in_order(items: Vec<&str>) -> Vec<&str> {
    let mut seen = HashSet::new();
    items
        .into_iter()
        .filter(|item| seen.insert(*item))
        .collect()
}

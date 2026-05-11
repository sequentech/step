// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Maps Velvet report rows and Miru-prefixed Hasura annotations into EML-shaped JSON.

use super::eml_types::*;
use crate::types::miru_plugin::*;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::{
    ballot::*,
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    types::{
        date_time::*,
        hasura::core::{self, ElectionEvent, Trustee},
    },
    util::date_time::generate_timestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use strum_macros::{Display, EnumString, ToString};
use tracing::{info, instrument};
use velvet::pipes::{do_tally::ContestResult, generate_reports::ReportData};

/// Namespace prefix for MIRU plugin keys in [`Annotations`] maps (`miru:…`).
pub const MIRU_PLUGIN_PREPEND: &str = "miru";
/// Annotation suffix: election event id string.
pub const MIRU_ELECTION_EVENT_ID: &str = "election-event-id";
/// Annotation suffix: election event display name.
pub const MIRU_ELECTION_EVENT_NAME: &str = "election-event-name";
/// Annotation suffix: election id (within event).
const MIRU_ELECTION_ID: &str = "election-id";
/// Annotation suffix: election display name.
const MIRU_ELECTION_NAME: &str = "election-name";
/// Annotation suffix: contest id.
const MIRU_CONTEST_ID: &str = "contest-id";
/// Annotation suffix: contest display name.
const MIRU_CONTEST_NAME: &str = "contest-name";
/// Annotation suffix: candidate id.
const MIRU_CANDIDATE_ID: &str = "candidate-id";
/// Annotation suffix: candidate display name.
const MIRU_CANDIDATE_NAME: &str = "candidate-name";
/// Annotation suffix: candidate status/setting code.
const MIRU_CANDIDATE_SETTING: &str = "candidate-setting";
/// Annotation suffix: affiliation id for the candidate’s party.
const MIRU_CANDIDATE_AFFILIATION_ID: &str = "candidate-affiliation-id";
/// Annotation suffix: registered affiliation name.
const MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME: &str = "candidate-affiliation-registered-name";
/// Annotation suffix: party label.
const MIRU_CANDIDATE_AFFILIATION_PARTY: &str = "candidate-affiliation-party";
/// Annotation suffix: JSON list of [`MiruCcsServer`] destinations.
pub const MIRU_AREA_CCS_SERVERS: &str = "area-ccs-servers";
/// Annotation suffix: precinct / station id string.
pub const MIRU_AREA_STATION_ID: &str = "area-station-id";
/// Annotation suffix: station display name.
pub const MIRU_AREA_STATION_NAME: &str = "area-station-name";
/// Annotation suffix: numeric threshold string for MIRU policy.
pub const MIRU_AREA_THRESHOLD: &str = "area-threshold";
/// Annotation suffix: JSON list of SBEI usernames allowed for this area.
pub const MIRU_AREA_TRUSTEE_USERS: &str = "area-trustee-users";
/// Annotation suffix: country code or name for the area.
pub const MIRU_AREA_COUNTRY: &str = "area-country";
/// Annotation suffix: registered voter count for the precinct.
pub const MIRU_AREA_REGISTERED_VOTERS: &str = "registered-voters";
/// Annotation suffix: JSON [`MiruTallySessionData`] blob on the tally session.
pub const MIRU_TALLY_SESSION_DATA: &str = "tally-session-data";
/// Annotation suffix: trustee id (legacy / display).
pub const MIRU_TRUSTEE_ID: &str = "trustee-id";
/// Annotation suffix: trustee display name.
pub const MIRU_TRUSTEE_NAME: &str = "trustee-name";
/// Annotation suffix: JSON list of [`MiruSbeiUser`] on the election event.
pub const MIRU_SBEI_USERS: &str = "sbei-users";
/// Annotation suffix: PEM root CA for optional client cert validation.
pub const MIRU_ROOT_CA: &str = "root-ca";
/// Annotation suffix: intermediate CA bundle text.
pub const MIRU_INTERMEDIATE_CAS: &str = "intermediate-cas";
/// Annotation suffix: `"true"` / `"false"` — validate client certs against CA store.
pub const MIRU_USE_ROOT_CA: &str = "use-root-ca";

/// `chrono`-style format for EML `issue_date`.
const ISSUE_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
/// Date-only format for official status timestamp in EML.
const OFFICIAL_STATUS_DATE_FORMAT: &str = "%Y-%m-%d";

// COMELEC-style geographic keys; adjust if a different jurisdiction’s EML mapping is needed.
/// Annotation suffix: geographical region label for the election post.
pub const MIRU_GEOGRAPHICAL_REGION: &str = "geographical-region";
/// Annotation suffix: voting center / post name.
pub const MIRU_VOTING_CENTER: &str = "voting-center";
/// Annotation suffix: precinct code.
pub const MIRU_PRECINCT_CODE: &str = "precinct-code";
/// Map key (no `miru:` prefix in code paths): poll center code.
pub const MIRU_POLLCENTER_CODE: &str = "pollcenter_code";

/// EML official-status enumeration (serialized lowercase).
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, EnumString, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OfficialStatus {
    /// Final official results (not provisional).
    OFFICIAL,
}

/// Builds COMELEC-style [`EMLCountMetric`] rows from a Velvet contest result.
pub trait GetMetrics {
    /// Fills standard metric ids (over/under votes, registered voters, etc.) for one contest.
    fn get_metrics(&self, registered_voters: i64) -> Vec<EMLCountMetric>;
}

impl GetMetrics for ContestResult {
    /// Builds the fixed set of COMELEC count metrics (over/under votes, RV, valid, etc.).
    #[instrument(skip_all, name = "ContestResult::get_metrics")]
    fn get_metrics(&self, registered_voters: i64) -> Vec<EMLCountMetric> {
        let extended_metrics = self.extended_metrics.unwrap_or_default();

        vec![
            EMLCountMetric {
                kind: "Total Number of Over Votes".into(),
                id: "OV".into(),
                datum: extended_metrics.over_votes.cast_signed(),
            },
            EMLCountMetric {
                kind: "Total Number of Under Votes".into(),
                id: "UV".into(),
                datum: extended_metrics.under_votes.cast_signed(),
            },
            EMLCountMetric {
                kind: "Total Number of Votes Actually".into(),
                id: "VV".into(),
                datum: extended_metrics.votes_actually.cast_signed(),
            },
            EMLCountMetric {
                kind: "Total Number of Registered Voters".into(),
                id: "RV".into(),
                datum: registered_voters,
            },
            EMLCountMetric {
                kind: "Total Number of Expected Votes".into(),
                id: "EV".into(),
                datum: extended_metrics.expected_votes.cast_signed(),
            },
            EMLCountMetric {
                kind: "Number of Zero Outs Executed".into(),
                id: "RZ".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Scanned Ballots".into(),
                id: "TB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Valid Ballots".into(),
                id: "VB".into(),
                datum: self.total_valid_votes.cast_signed(),
            },
            EMLCountMetric {
                kind: "Total Number of Stamped Ballots".into(),
                id: "SB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Ballots In Ballot Box".into(),
                id: "BB".into(),
                datum: self.total_votes.cast_signed(),
            },
            EMLCountMetric {
                kind: "Abstentions".into(),
                id: "AB".into(),
                datum: self.total_blank_votes.cast_signed(),
            },
            EMLCountMetric {
                kind: "Total Number of Invalid Ballots".into(),
                id: "IB".into(),
                datum: self.total_invalid_votes.cast_signed(),
            },
            EMLCountMetric {
                kind: "Total Number of Misread Ballots".into(),
                id: "MB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Fake Ballots".into(),
                id: "FB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Previously Casted Ballots".into(),
                id: "PB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Returned Ballots".into(),
                id: "RB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Rejected Ballots".into(),
                id: "JB".into(),
                datum: 0,
            },
        ]
    }
}

/// Parses MIRU-prefixed keys from a [`sequent_core::ballot::Annotations`] map into a typed `Item`.
pub trait ValidateAnnotations {
    /// Strongly typed view produced from annotations (event, area, election, …).
    type Item;

    /// Requires all expected MIRU keys; returns an error if any are missing or JSON is invalid.
    ///
    /// # Errors
    ///
    /// Missing annotation map, missing keys, or deserialize failures.
    fn get_annotations(&self) -> Result<Self::Item>;
    /// Like [`get_annotations`](Self::get_annotations) but fills defaults when the map or keys are absent.
    ///
    /// # Errors
    ///
    /// Parse errors on present-but-invalid values (implementation-defined).
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        self.get_annotations()
    }
}

/// Returns `Err` if any `keys` are absent from `annotations`.
///
/// # Errors
///
/// The first missing key produces an error.
#[instrument(err, skip(annotations))]
fn check_annotations_exist(keys: Vec<String>, annotations: &Annotations) -> Result<()> {
    for key in keys {
        if !annotations.contains_key(&key) {
            return Err(anyhow!("Annotation: missing key {key}"));
        }
    }
    Ok(())
}

/// MIRU fields stored on the election event (ids, SBEI roster, optional TLS trust material).
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruElectionEventAnnotations {
    /// Election event id (annotation `miru:election-event-id`).
    pub event_id: String,
    /// Election event display name.
    pub event_name: String,
    /// Configured SBEI users for signing and CCS workflow.
    pub sbei_users: Vec<MiruSbeiUser>,
    /// PEM root CA text when validating client certificates.
    pub root_ca: String,
    /// Intermediate CA PEM(s) or bundle text.
    pub intermediate_cas: String,
    /// Whether to enforce CA validation for P12 uploads.
    pub use_root_ca: bool,
}

impl ValidateAnnotations for ElectionEvent {
    type Item = MiruElectionEventAnnotations;

    /// # Errors
    ///
    /// Missing or invalid election-event annotations or embedded JSON lists.
    #[instrument(skip_all, err, name = "ElectionEvent::get_annotations")]
    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing election event annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_ELECTION_EVENT_ID),
                prepend_miru_annotation(MIRU_ELECTION_EVENT_NAME),
                prepend_miru_annotation(MIRU_SBEI_USERS),
                prepend_miru_annotation(MIRU_ROOT_CA),
                prepend_miru_annotation(MIRU_INTERMEDIATE_CAS),
                prepend_miru_annotation(MIRU_USE_ROOT_CA),
            ],
            &annotations,
        )
        .with_context(|| "Election Event: ")?;

        let event_id =
            find_miru_annotation(MIRU_ELECTION_EVENT_ID, &annotations).with_context(|| {
                format!(
                    "Missing election event annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_ELECTION_EVENT_ID}'"
                )
            })?;

        let event_name = find_miru_annotation(MIRU_ELECTION_EVENT_NAME, &annotations)
            .with_context(|| {
                format!(
                    "Missing election event annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_ELECTION_EVENT_NAME}'"
                )
            })?;

        let sbei_users_js =
            find_miru_annotation(MIRU_SBEI_USERS, &annotations).with_context(|| {
                format!(
                    "Missing election event annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_SBEI_USERS}::: {annotations:?}'",
                )
            })?;
        let sbei_users: Vec<MiruSbeiUser> = deserialize_str(&sbei_users_js)
            .map_err(|err| anyhow::Error::from(err).context("Can't parse sbei users"))?;

        let root_ca = find_miru_annotation(MIRU_ROOT_CA, &annotations).with_context(|| {
            format!("Missing election event annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_ROOT_CA}'",)
        })?;

        let intermediate_cas = find_miru_annotation(MIRU_INTERMEDIATE_CAS, &annotations)
            .with_context(|| {
                format!(
                    "Missing election event annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_INTERMEDIATE_CAS}'",
                )
            })?;

        let use_root_ca =
            find_miru_annotation(MIRU_USE_ROOT_CA, &annotations).with_context(|| {
                format!(
                    "Missing election event annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_ROOT_CA}'",
                )
            })?;

        Ok(MiruElectionEventAnnotations {
            event_id,
            event_name,
            sbei_users,
            root_ca,
            intermediate_cas,
            use_root_ca: "true" == use_root_ca.as_str(),
        })
    }

    /// # Errors
    ///
    /// Deserialize failures when partial annotation values are malformed.
    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(serde_json::Map::default()));

        let annotations: Annotations = deserialize_value(annotations_js).unwrap_or_default();

        let event_id = find_miru_annotation_opt(MIRU_ELECTION_EVENT_ID, &annotations)?
            .unwrap_or("-".to_string());

        let event_name = find_miru_annotation_opt(MIRU_ELECTION_EVENT_NAME, &annotations)?
            .unwrap_or("-".to_string());

        let sbei_users_js =
            find_miru_annotation_opt(MIRU_SBEI_USERS, &annotations)?.unwrap_or_default();
        let sbei_users: Vec<MiruSbeiUser> =
            deserialize_str(&sbei_users_js).unwrap_or_else(|_| Vec::new());

        let root_ca = find_miru_annotation_opt(MIRU_ROOT_CA, &annotations)?.unwrap_or_default();
        let intermediate_cas =
            find_miru_annotation_opt(MIRU_INTERMEDIATE_CAS, &annotations)?.unwrap_or_default();

        let use_root_ca =
            find_miru_annotation_opt(MIRU_USE_ROOT_CA, &annotations)?.unwrap_or_default();

        Ok(MiruElectionEventAnnotations {
            event_id,
            event_name,
            sbei_users,
            root_ca,
            intermediate_cas,
            use_root_ca: "true" == use_root_ca.as_str(),
        })
    }
}

/// Geographic and naming metadata for one election post (COMELEC-oriented fields).
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruElectionAnnotations {
    /// Election id within the event.
    pub election_id: String,
    /// Election display name.
    pub election_name: String,
    /// Region / geographical label.
    pub geographical_area: String,
    /// Voting center / post label.
    pub post: String,
    /// Precinct code string.
    pub precinct_code: String,
    /// Poll center code (unprefixed map key in annotations).
    pub pollcenter_code: String,
}

impl ValidateAnnotations for core::Election {
    type Item = MiruElectionAnnotations;

    /// # Errors
    ///
    /// Missing election annotations or required MIRU keys (including poll center).
    #[instrument(skip_all, err, name = "Election::get_annotations")]
    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing election event annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_ELECTION_ID),
                prepend_miru_annotation(MIRU_ELECTION_NAME),
                prepend_miru_annotation(MIRU_GEOGRAPHICAL_REGION),
                prepend_miru_annotation(MIRU_VOTING_CENTER),
                prepend_miru_annotation(MIRU_PRECINCT_CODE),
            ],
            &annotations,
        )
        .with_context(|| "Contest: ")?;

        let election_id =
            find_miru_annotation(MIRU_ELECTION_ID, &annotations).with_context(|| {
                format!("Missing election annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_ELECTION_ID}'",)
            })?;

        let election_name =
            find_miru_annotation(MIRU_ELECTION_NAME, &annotations).with_context(|| {
                format!(
                    "Missing election annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_ELECTION_NAME}'",
                )
            })?;

        let geographical_area = find_miru_annotation(MIRU_GEOGRAPHICAL_REGION, &annotations)
            .with_context(|| {
                format!(
                    "Missing election annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_GEOGRAPHICAL_REGION}'"
                )
            })?;

        let post = find_miru_annotation(MIRU_VOTING_CENTER, &annotations).with_context(|| {
            format!("Missing election annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_VOTING_CENTER}'",)
        })?;

        let precinct_code =
            find_miru_annotation(MIRU_PRECINCT_CODE, &annotations).with_context(|| {
                format!(
                    "Missing election annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_PRECINCT_CODE}'",
                )
            })?;

        let pollcenter_code = annotations
            .get(MIRU_POLLCENTER_CODE)
            .with_context(|| format!("Missing election annotation: {MIRU_POLLCENTER_CODE}"))
            .cloned()?;

        Ok(MiruElectionAnnotations {
            election_id,
            election_name,
            geographical_area,
            post,
            precinct_code,
            pollcenter_code,
        })
    }

    /// # Errors
    ///
    /// JSON deserialization errors when the annotation map is non-empty but invalid.
    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(serde_json::Map::default()));

        let annotations: Annotations = deserialize_value(annotations_js)?;

        let election_id =
            find_miru_annotation_opt(MIRU_ELECTION_ID, &annotations)?.unwrap_or("-".to_string());

        let election_name =
            find_miru_annotation_opt(MIRU_ELECTION_NAME, &annotations)?.unwrap_or("-".to_string());

        let geographical_area = find_miru_annotation_opt(MIRU_GEOGRAPHICAL_REGION, &annotations)?
            .unwrap_or("-".to_string());

        let post =
            find_miru_annotation_opt(MIRU_VOTING_CENTER, &annotations)?.unwrap_or("-".to_string());

        let precinct_code =
            find_miru_annotation_opt(MIRU_PRECINCT_CODE, &annotations)?.unwrap_or("-".to_string());

        let pollcenter_code = annotations
            .get(MIRU_POLLCENTER_CODE)
            .cloned()
            .unwrap_or_default();

        Ok(MiruElectionAnnotations {
            election_id,
            election_name,
            geographical_area,
            post,
            precinct_code,
            pollcenter_code,
        })
    }
}

/// Precinct-level MIRU config: CCS endpoints, station ids, SBEI allowlist, registered voter count.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruAreaAnnotations {
    /// Destinations for `er_` / `al_` uploads.
    pub ccs_servers: Vec<MiruCcsServer>,
    /// Station / precinct id string.
    pub station_id: String,
    /// Human-readable station name.
    pub station_name: String,
    /// Policy threshold parsed from annotations.
    pub threshold: i64,
    /// Miru ids of SBEI users permitted for this area (cross-reference event `sbei_users`).
    pub sbei_ids: Vec<String>,
    /// Country field for EML / reporting.
    pub country: String,
    /// Registered voters for metric generation in EML.
    pub registered_voters: i64,
}

impl ValidateAnnotations for core::Area {
    type Item = MiruAreaAnnotations;

    /// # Errors
    ///
    /// Missing area annotations, parse errors for numeric fields, or invalid embedded JSON lists.
    #[instrument(skip_all, err, name = "Area::get_annotations")]
    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing election event annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_AREA_CCS_SERVERS),
                prepend_miru_annotation(MIRU_AREA_STATION_ID),
                prepend_miru_annotation(MIRU_AREA_STATION_NAME),
                prepend_miru_annotation(MIRU_AREA_THRESHOLD),
                prepend_miru_annotation(MIRU_AREA_TRUSTEE_USERS),
                prepend_miru_annotation(MIRU_AREA_COUNTRY),
                prepend_miru_annotation(MIRU_AREA_REGISTERED_VOTERS),
            ],
            &annotations,
        )
        .with_context(|| "Area: ")?;

        let station_id =
            find_miru_annotation(MIRU_AREA_STATION_ID, &annotations).with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_AREA_STATION_ID}'")
            })?;

        let station_name = find_miru_annotation(MIRU_AREA_STATION_NAME, &annotations)
            .with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_AREA_STATION_NAME}'")
            })?;

        let threshold = find_miru_annotation(MIRU_AREA_THRESHOLD, &annotations)
            .with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_AREA_THRESHOLD}'")
            })?
            .parse::<i64>()
            .with_context(|| anyhow!("Can't parse threshold"))?;

        let ccs_servers_js = find_miru_annotation(MIRU_AREA_CCS_SERVERS, &annotations)
            .with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_AREA_CCS_SERVERS}'")
            })?;

        let ccs_servers: Vec<MiruCcsServer> =
            deserialize_str(&ccs_servers_js).map_err(|err| anyhow!("{err}"))?;

        let sbei_usernames_js = find_miru_annotation(MIRU_AREA_TRUSTEE_USERS, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_AREA_TRUSTEE_USERS}'"
                )
            })?;

        let sbei_usernames: Vec<String> =
            deserialize_str(&sbei_usernames_js).map_err(|err| anyhow!("{err}"))?;

        let country = find_miru_annotation(MIRU_AREA_COUNTRY, &annotations).with_context(|| {
            format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_AREA_COUNTRY}'")
        })?;

        let registered_voters: i64 =
            find_miru_annotation(MIRU_AREA_REGISTERED_VOTERS, &annotations)
                .with_context(|| {
                    format!(
                        "Missing election annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_AREA_REGISTERED_VOTERS}'"
                    )
                })?
                .parse::<i64>()
                .with_context(|| anyhow!("Can't parse registered_voters"))?;

        Ok(MiruAreaAnnotations {
            ccs_servers,
            station_id,
            station_name,
            threshold,
            sbei_ids: sbei_usernames,
            country,
            registered_voters,
        })
    }

    /// # Errors
    ///
    /// JSON or numeric parse errors when optional fields are present but malformed.
    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(serde_json::Map::default()));

        let annotations: Annotations = deserialize_value(annotations_js).unwrap_or_default();

        let station_id = find_miru_annotation_opt(MIRU_AREA_STATION_ID, &annotations)?
            .unwrap_or("-".to_string());

        let station_name = find_miru_annotation_opt(MIRU_AREA_STATION_NAME, &annotations)?
            .unwrap_or("-".to_string());

        let threshold = find_miru_annotation_opt(MIRU_AREA_THRESHOLD, &annotations)?
            .unwrap_or("0".to_string())
            .parse::<i64>()
            .with_context(|| anyhow!("Can't parse threshold"))?;

        let ccs_servers_js =
            find_miru_annotation_opt(MIRU_AREA_CCS_SERVERS, &annotations)?.unwrap_or_default();

        let ccs_servers: Vec<MiruCcsServer> =
            deserialize_str(&ccs_servers_js).unwrap_or_else(|_| Vec::new());

        let sbei_usernames_js =
            find_miru_annotation_opt(MIRU_AREA_TRUSTEE_USERS, &annotations)?.unwrap_or_default();
        let sbei_usernames: Vec<String> =
            deserialize_str(&sbei_usernames_js).unwrap_or_else(|_| Vec::new());

        let country =
            find_miru_annotation_opt(MIRU_AREA_COUNTRY, &annotations)?.unwrap_or("-".to_string());

        let registered_voters: i64 =
            find_miru_annotation_opt(MIRU_AREA_REGISTERED_VOTERS, &annotations)?
                .and_then(|val| val.parse::<i64>().ok())
                .unwrap_or(-1); //TODO: fix

        Ok(MiruAreaAnnotations {
            ccs_servers,
            station_id,
            station_name,
            threshold,
            sbei_ids: sbei_usernames,
            country,
            registered_voters,
        })
    }
}

impl ValidateAnnotations for core::TallySession {
    type Item = MiruTallySessionData;

    /// # Errors
    ///
    /// Missing tally annotations or invalid `miru:tally-session-data` JSON.
    #[instrument(skip_all, err, name = "TallySession::get_annotations")]
    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing tally session annotations"))?;

        let annotations: Annotations = deserialize_value(annotations_js)?;

        let tally_session_data_js = find_miru_annotation(MIRU_TALLY_SESSION_DATA, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_TALLY_SESSION_DATA}'"
                )
            })?;

        let tally_session_data: MiruTallySessionData =
            deserialize_str(&tally_session_data_js).map_err(|err| anyhow!("{}", err))?;

        Ok(tally_session_data)
    }

    /// # Errors
    ///
    /// Rare: fails if an empty-default path deserializes invalidly (normally returns an empty vec).
    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(serde_json::Map::default()));
        let annotations: Annotations = deserialize_value(annotations_js).unwrap_or_default();
        let tally_session_data_js =
            find_miru_annotation_opt(MIRU_TALLY_SESSION_DATA, &annotations)?.unwrap_or_default();

        let tally_session_data: MiruTallySessionData =
            deserialize_str(&tally_session_data_js).unwrap_or_else(|_| Vec::new());
        Ok(tally_session_data)
    }
}

/// Display id and title for a contest row in EML.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruContestAnnotations {
    /// Contest title from MIRU annotations.
    pub contest_name: String,
    /// Contest id from MIRU annotations.
    pub contest_id: String,
}

impl ValidateAnnotations for Contest {
    type Item = MiruContestAnnotations;

    /// # Errors
    ///
    /// Missing contest annotations or required MIRU keys.
    #[instrument(skip_all, err, name = "Contest::get_annotations")]
    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing contest annotations"))?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_CONTEST_NAME),
                prepend_miru_annotation(MIRU_CONTEST_ID),
            ],
            &annotations,
        )
        .with_context(|| "Contest: ")?;

        let contest_name =
            find_miru_annotation(MIRU_CONTEST_NAME, &annotations).with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CONTEST_NAME}'")
            })?;

        let contest_id =
            find_miru_annotation(MIRU_CONTEST_ID, &annotations).with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CONTEST_ID}'")
            })?;
        Ok(MiruContestAnnotations {
            contest_name,
            contest_id,
        })
    }
}

/// Candidate and party fields mirrored into EML from MIRU contest annotations.
#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruCandidateAnnotations {
    /// Display name.
    pub candidate_name: String,
    /// Candidate id string.
    pub candidate_id: String,
    /// Status / setting code for the row.
    pub candidate_setting: String,
    /// Party id string.
    pub candidate_affiliation_id: String,
    /// Registered party name.
    pub candidate_affiliation_registered_name: String,
    /// Party label / acronym.
    pub candidate_affiliation_party: String,
}

impl ValidateAnnotations for Candidate {
    type Item = MiruCandidateAnnotations;

    /// # Errors
    ///
    /// Missing candidate annotations or any required affiliation keys.
    #[instrument(skip_all, err, name = "Candidate::get_annotations")]
    fn get_annotations(&self) -> Result<Self::Item> {
        let annotations = self
            .annotations
            .clone()
            .ok_or_else(|| anyhow!("Missing candidate annotations"))?;

        check_annotations_exist(
            vec![
                prepend_miru_annotation(MIRU_CANDIDATE_ID),
                prepend_miru_annotation(MIRU_CANDIDATE_NAME),
                prepend_miru_annotation(MIRU_CANDIDATE_SETTING),
                prepend_miru_annotation(MIRU_CANDIDATE_AFFILIATION_ID),
                prepend_miru_annotation(MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME),
                prepend_miru_annotation(MIRU_CANDIDATE_AFFILIATION_PARTY),
            ],
            &annotations,
        )
        .with_context(|| "Candidate: ")?;

        let candidate_name =
            find_miru_annotation(MIRU_CANDIDATE_NAME, &annotations).with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CANDIDATE_NAME}'")
            })?;

        let candidate_id =
            find_miru_annotation(MIRU_CANDIDATE_ID, &annotations).with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CANDIDATE_ID}'")
            })?;

        let candidate_setting = find_miru_annotation(MIRU_CANDIDATE_SETTING, &annotations)
            .with_context(|| {
                format!("Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CANDIDATE_SETTING}'")
            })?;

        let candidate_affiliation_id =
            find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_ID, &annotations).with_context(
                || {
                    format!(
                "Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CANDIDATE_AFFILIATION_ID}'"
            )
                },
            )?;

        let candidate_affiliation_registered_name =
            find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME, &annotations)
                .with_context(|| {
                    format!(
                        "Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME}'"
                    )
                })?;

        let candidate_affiliation_party =
            find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_PARTY, &annotations).with_context(
                || {
                    format!(
                        "Missing area annotation: '{MIRU_PLUGIN_PREPEND}:{MIRU_CANDIDATE_AFFILIATION_PARTY}'"
                    )
                },
            )?;

        Ok(MiruCandidateAnnotations {
            candidate_name,
            candidate_id,
            candidate_setting,
            candidate_affiliation_id,
            candidate_affiliation_registered_name,
            candidate_affiliation_party,
        })
    }
}

/// Returns `miru:{data}` for use as an [`Annotations`] map key.
#[instrument]
pub fn prepend_miru_annotation(data: &str) -> String {
    format!("{MIRU_PLUGIN_PREPEND}:{data}")
}

/// Looks up `miru:{data}`.
/// # Errors
///
/// Missing key in `annotations`.
#[instrument(err, skip(annotations))]
pub fn find_miru_annotation(data: &str, annotations: &Annotations) -> Result<String> {
    let key = prepend_miru_annotation(data);
    annotations
        .get(&key)
        .ok_or(anyhow!("Can't find annotation key {key}"))
        .cloned()
}

/// Looks up `miru:{data}` and returns `None` if absent.
///
/// # Errors
///
/// None — always returns `Ok`.
#[instrument(err, skip(annotations))]
pub fn find_miru_annotation_opt(data: &str, annotations: &Annotations) -> Result<Option<String>> {
    let key = prepend_miru_annotation(data);
    Ok(annotations.get(&key).cloned())
}

/// Maps one Velvet [`ReportData`] row into an [`EMLContest`] (metrics + selections).
///
/// # Panics
///
/// If `report.contest` or `report.contest_result` is `None` (caller must supply a complete report).
///
/// # Errors
///
/// Contest or candidate annotation validation failures, or errors mapping candidate rows.
#[instrument(err, skip_all)]
pub fn render_eml_contest(
    report: &ReportData,
    area_annotations: &MiruAreaAnnotations,
) -> Result<EMLContest> {
    let contest = report.contest.as_ref().expect("report is missing contest");
    let contest_result = report
        .contest_result
        .as_ref()
        .expect("report is missing contest_result");

    // Extract contest annotations
    let contest_annotations = contest
        .get_annotations()
        .with_context(|| "render_eml_contest: ")?;

    let registered_voters = area_annotations.registered_voters;

    let count_metrics = contest_result.get_metrics(registered_voters);

    let selections: Vec<EMLSelection> = contest_result
        .candidate_result
        .iter()
        .map(|candidate_result| -> Result<EMLSelection> {
            // Retrieve candidate annotations
            let candidate_annotations = candidate_result
                .candidate
                .get_annotations()
                .with_context(|| "render_eml_contest: ")?;

            let candidate = EMLCandidate {
                identifier: EMLIdentifier {
                    id_number: candidate_annotations.candidate_id.clone(),
                    name: candidate_annotations.candidate_name.clone(),
                },
                status_details: vec![EMLStatusItem {
                    setting: candidate_annotations.candidate_setting.clone(),
                }],
                affiliation: EMLAffiliation {
                    identifier: EMLIdentifier {
                        id_number: candidate_annotations.candidate_affiliation_id.clone(),
                        name: candidate_annotations
                            .candidate_affiliation_registered_name
                            .clone(),
                    },
                    party: candidate_annotations.candidate_affiliation_party.clone(),
                },
            };
            Ok(EMLSelection {
                candidates: vec![candidate.clone()],
                valid_votes: candidate_result.total_count.cast_signed(),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let contests = EMLContest {
        identifier: EMLIdentifier {
            id_number: contest_annotations.contest_id.clone(),
            name: contest_annotations.contest_name.clone(),
        },
        total_votes: EMLTotalVotes {
            count_metrics,
            selections,
        },
    };

    Ok(contests)
}

/// Assembles the root [`EMLFile`] for a tally: header timestamps, event/election ids, and all contests.
///
/// # Errors
///
/// Failures from [`render_eml_contest`] when iterating `reports`.
#[instrument(err, skip(election_event_annotations, election_annotations, reports))]
pub fn render_eml_file(
    tally_id: &str,
    transaction_id: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &MiruElectionEventAnnotations,
    election_annotations: &MiruElectionAnnotations,
    area_annotations: &MiruAreaAnnotations,
    reports: &[ReportData],
) -> Result<EMLFile> {
    let issue_date = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(ISSUE_DATE_FORMAT.to_string())),
        Some(date_time),
    );
    let official_status_date = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(OFFICIAL_STATUS_DATE_FORMAT.to_string())),
        Some(date_time),
    );

    let eml_file = EMLFile {
        id: tally_id.to_string(),
        header: EMLHeader {
            transaction_id: transaction_id.to_string(),
            issue_date,
            official_status_detail: EMLOfficialStatusDetail {
                official_status: OfficialStatus::OFFICIAL.to_string(),
                status_date: official_status_date,
            },
        },
        counts: vec![EMLCount {
            identifier: EMLIdentifier {
                id_number: election_event_annotations.event_id.clone(),
                name: election_event_annotations.event_name.clone(),
            },
            elections: vec![EMLElection {
                identifier: EMLIdentifier {
                    id_number: election_annotations.election_id.clone(),
                    name: election_annotations.election_name.clone(),
                },
                contests: reports
                    .iter()
                    .map(|report| render_eml_contest(report, area_annotations))
                    .collect::<Result<Vec<_>>>()
                    .with_context(|| "Error rendering EML Contest")?,
            }],
        }],
    };
    Ok(eml_file)
}

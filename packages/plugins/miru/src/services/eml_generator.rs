// SPDX-FileCopyrightText: 2025 Felix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
use super::eml_types::*;
use crate::services::miru_plugin_types::*;
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use sequent_core::{
    ballot::*,
    serialization::deserialize_with_path::{deserialize_str, deserialize_value},
    types::{
        date_time::*,
        hasura::core::{self, ElectionEvent},
        velvet::{ContestResult, ReportData},
    },
    util::date_time::generate_timestamp,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use tracing::instrument;

pub const MIRU_PLUGIN_PREPEND: &str = "miru";
pub const MIRU_ELECTION_EVENT_ID: &str = "election-event-id";
pub const MIRU_ELECTION_EVENT_NAME: &str = "election-event-name";
const MIRU_ELECTION_ID: &str = "election-id";
const MIRU_ELECTION_NAME: &str = "election-name";
const MIRU_CONTEST_ID: &str = "contest-id";
const MIRU_CONTEST_NAME: &str = "contest-name";
const MIRU_CANDIDATE_ID: &str = "candidate-id";
const MIRU_CANDIDATE_NAME: &str = "candidate-name";
const MIRU_CANDIDATE_SETTING: &str = "candidate-setting";
const MIRU_CANDIDATE_AFFILIATION_ID: &str = "candidate-affiliation-id";
const MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME: &str = "candidate-affiliation-registered-name";
const MIRU_CANDIDATE_AFFILIATION_PARTY: &str = "candidate-affiliation-party";
pub const MIRU_AREA_CCS_SERVERS: &str = "area-ccs-servers";
pub const MIRU_AREA_STATION_ID: &str = "area-station-id";
pub const MIRU_AREA_STATION_NAME: &str = "area-station-name";
pub const MIRU_AREA_THRESHOLD: &str = "area-threshold";
pub const MIRU_AREA_TRUSTEE_USERS: &str = "area-trustee-users";
pub const MIRU_AREA_COUNTRY: &str = "area-country";
pub const MIRU_AREA_REGISTERED_VOTERS: &str = "registered-voters";
pub const MIRU_TALLY_SESSION_DATA: &str = "tally-session-data";
pub const MIRU_TRUSTEE_ID: &str = "trustee-id";
pub const MIRU_TRUSTEE_NAME: &str = "trustee-name";
pub const MIRU_SBEI_USERS: &str = "sbei-users";
pub const MIRU_ROOT_CA: &str = "root-ca";
pub const MIRU_INTERMEDIATE_CAS: &str = "intermediate-cas";
pub const MIRU_USE_ROOT_CA: &str = "use-root-ca";

const ISSUE_DATE_FORMAT: &str = "%Y-%m-%dT%H:%M:%S";
const OFFICIAL_STATUS_DATE_FORMAT: &str = "%Y-%m-%d";

/*COMELEC ELECTION DATA -> to be change if revice different keys  */
pub const MIRU_GEOGRAPHICAL_REGION: &str = "geographical-region";
pub const MIRU_VOTING_CENTER: &str = "voting-center";
pub const MIRU_PRECINCT_CODE: &str = "precinct-code";
pub const MIRU_POLLCENTER_CODE: &str = "pollcenter_code";
/**/

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, EnumString, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum OfficialStatus {
    OFFICIAL,
}

pub trait GetMetrics {
    fn get_metrics(&self, registered_voters: i64) -> Vec<EMLCountMetric>;
}

// TODO: review
impl GetMetrics for ContestResult {
    #[instrument(skip_all, name = "ContestResult::get_metrics")]
    fn get_metrics(&self, registered_voters: i64) -> Vec<EMLCountMetric> {
        let extended_metrics = self.extended_metrics.clone().unwrap_or_default();

        vec![
            EMLCountMetric {
                kind: "Total Number of Over Votes".into(),
                id: "OV".into(),
                datum: extended_metrics.over_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Under Votes".into(),
                id: "UV".into(),
                datum: extended_metrics.under_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Votes Actually".into(),
                id: "VV".into(),
                datum: extended_metrics.votes_actually as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Registered Voters".into(),
                id: "RV".into(),
                datum: registered_voters,
            },
            EMLCountMetric {
                kind: "Total Number of Expected Votes".into(),
                id: "EV".into(),
                datum: extended_metrics.expected_votes as i64,
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
                datum: self.total_valid_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Stamped Ballots".into(),
                id: "SB".into(),
                datum: 0,
            },
            EMLCountMetric {
                kind: "Total Number of Ballots In Ballot Box".into(),
                id: "BB".into(),
                datum: self.total_votes as i64,
            },
            EMLCountMetric {
                kind: "Abstentions".into(),
                id: "AB".into(),
                datum: self.total_blank_votes as i64,
            },
            EMLCountMetric {
                kind: "Total Number of Invalid Ballots".into(),
                id: "IB".into(),
                datum: self.total_invalid_votes as i64,
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

pub trait ValidateAnnotations {
    type Item;

    fn get_annotations(&self) -> Result<Self::Item>;
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        self.get_annotations()
    }
}

#[instrument(err, skip(annotations))]
fn check_annotations_exist(keys: Vec<String>, annotations: &Annotations) -> Result<()> {
    for key in keys {
        if !annotations.contains_key(&key) {
            return Err(anyhow!("Annotation: missing key {}", key));
        }
    }
    Ok(())
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruElectionEventAnnotations {
    pub event_id: String,
    pub event_name: String,
    pub sbei_users: Vec<MiruSbeiUser>,
    pub root_ca: String,
    pub intermediate_cas: String,
    pub use_root_ca: bool,
}

impl ValidateAnnotations for ElectionEvent {
    type Item = MiruElectionEventAnnotations;

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
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_ID
                )
            })?;

        let event_name = find_miru_annotation(MIRU_ELECTION_EVENT_NAME, &annotations)
            .with_context(|| {
                format!(
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_EVENT_NAME
                )
            })?;

        let sbei_users_js =
            find_miru_annotation(MIRU_SBEI_USERS, &annotations).with_context(|| {
                format!(
                    "Missing election event annotation: '{}:{}::: {:?}'",
                    MIRU_PLUGIN_PREPEND, MIRU_SBEI_USERS, &annotations
                )
            })?;
        let sbei_users: Vec<MiruSbeiUser> = deserialize_str(&sbei_users_js)
            .map_err(|err| anyhow::Error::from(err).context("Can't parse sbei users"))?;

        let root_ca = find_miru_annotation(MIRU_ROOT_CA, &annotations).with_context(|| {
            format!(
                "Missing election event annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_ROOT_CA
            )
        })?;

        let intermediate_cas = find_miru_annotation(MIRU_INTERMEDIATE_CAS, &annotations)
            .with_context(|| {
                format!(
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_INTERMEDIATE_CAS
                )
            })?;

        let use_root_ca =
            find_miru_annotation(MIRU_USE_ROOT_CA, &annotations).with_context(|| {
                format!(
                    "Missing election event annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ROOT_CA
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
    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(Default::default()));

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

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruElectionAnnotations {
    pub election_id: String,
    pub election_name: String,
    pub geographical_area: String,
    pub post: String,
    pub precinct_code: String,
    pub pollcenter_code: String,
}

impl ValidateAnnotations for core::Election {
    type Item = MiruElectionAnnotations;

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
                format!(
                    "Missing election annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_ID
                )
            })?;

        let election_name =
            find_miru_annotation(MIRU_ELECTION_NAME, &annotations).with_context(|| {
                format!(
                    "Missing election annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_ELECTION_NAME
                )
            })?;

        let geographical_area = find_miru_annotation(MIRU_GEOGRAPHICAL_REGION, &annotations)
            .with_context(|| {
                format!(
                    "Missing election annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_GEOGRAPHICAL_REGION
                )
            })?;

        let post = find_miru_annotation(MIRU_VOTING_CENTER, &annotations).with_context(|| {
            format!(
                "Missing election annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_VOTING_CENTER
            )
        })?;

        let precinct_code =
            find_miru_annotation(MIRU_PRECINCT_CODE, &annotations).with_context(|| {
                format!(
                    "Missing election annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_PRECINCT_CODE
                )
            })?;

        let pollcenter_code = annotations
            .get(MIRU_POLLCENTER_CODE)
            .with_context(|| format!("Missing election annotation: {}", MIRU_POLLCENTER_CODE))
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

    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(Default::default()));

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

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruAreaAnnotations {
    pub ccs_servers: Vec<MiruCcsServer>,
    pub station_id: String,
    pub station_name: String,
    pub threshold: i64,
    pub sbei_ids: Vec<String>, // the miru id of the sbei user, the election event has their annotations
    pub country: String,
    pub registered_voters: i64, // registered voters at a given precinct id
}

impl ValidateAnnotations for core::Area {
    type Item = MiruAreaAnnotations;

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
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_AREA_STATION_ID
                )
            })?;

        let station_name = find_miru_annotation(MIRU_AREA_STATION_NAME, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_AREA_STATION_NAME
                )
            })?;

        let threshold = find_miru_annotation(MIRU_AREA_THRESHOLD, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_AREA_THRESHOLD
                )
            })?
            .parse::<i64>()
            .with_context(|| anyhow!("Can't parse threshold"))?;

        let ccs_servers_js = find_miru_annotation(MIRU_AREA_CCS_SERVERS, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_AREA_CCS_SERVERS
                )
            })?;

        let ccs_servers: Vec<MiruCcsServer> =
            deserialize_str(&ccs_servers_js).map_err(|err| anyhow!("{}", err))?;

        let sbei_usernames_js = find_miru_annotation(MIRU_AREA_TRUSTEE_USERS, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_AREA_TRUSTEE_USERS
                )
            })?;

        let sbei_usernames: Vec<String> =
            deserialize_str(&sbei_usernames_js).map_err(|err| anyhow!("{}", err))?;

        let country = find_miru_annotation(MIRU_AREA_COUNTRY, &annotations).with_context(|| {
            format!(
                "Missing area annotation: '{}:{}'",
                MIRU_PLUGIN_PREPEND, MIRU_AREA_COUNTRY
            )
        })?;

        let registered_voters: i64 =
            find_miru_annotation(MIRU_AREA_REGISTERED_VOTERS, &annotations)
                .with_context(|| {
                    format!(
                        "Missing election annotation: '{}:{}'",
                        MIRU_PLUGIN_PREPEND, MIRU_AREA_REGISTERED_VOTERS
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

    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(Default::default()));

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
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_TALLY_SESSION_DATA
                )
            })?;

        let tally_session_data: MiruTallySessionData =
            deserialize_str(&tally_session_data_js).map_err(|err| anyhow!("{}", err))?;

        Ok(tally_session_data)
    }

    #[instrument(err, skip_all)]
    fn get_annotations_or_empty_values(&self) -> Result<Self::Item> {
        let annotations_js = self
            .annotations
            .clone()
            .unwrap_or_else(|| Value::Object(Default::default()));
        let annotations: Annotations = deserialize_value(annotations_js).unwrap_or_default();
        let tally_session_data_js =
            find_miru_annotation_opt(MIRU_TALLY_SESSION_DATA, &annotations)?.unwrap_or_default();

        let tally_session_data: MiruTallySessionData =
            deserialize_str(&tally_session_data_js).unwrap_or_else(|_| Vec::new());
        Ok(tally_session_data)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruContestAnnotations {
    pub contest_name: String,
    pub contest_id: String,
}

impl ValidateAnnotations for Contest {
    type Item = MiruContestAnnotations;

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
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_CONTEST_NAME
                )
            })?;

        let contest_id =
            find_miru_annotation(MIRU_CONTEST_ID, &annotations).with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_CONTEST_ID
                )
            })?;
        Ok(MiruContestAnnotations {
            contest_name,
            contest_id,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct MiruCandidateAnnotations {
    pub candidate_name: String,
    pub candidate_id: String,
    pub candidate_setting: String,
    pub candidate_affiliation_id: String,
    pub candidate_affiliation_registered_name: String,
    pub candidate_affiliation_party: String,
}

impl ValidateAnnotations for Candidate {
    type Item = MiruCandidateAnnotations;

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
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_NAME
                )
            })?;

        let candidate_id =
            find_miru_annotation(MIRU_CANDIDATE_ID, &annotations).with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_ID
                )
            })?;

        let candidate_setting = find_miru_annotation(MIRU_CANDIDATE_SETTING, &annotations)
            .with_context(|| {
                format!(
                    "Missing area annotation: '{}:{}'",
                    MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_SETTING
                )
            })?;

        let candidate_affiliation_id =
            find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_ID, &annotations).with_context(
                || {
                    format!(
                        "Missing area annotation: '{}:{}'",
                        MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_AFFILIATION_ID
                    )
                },
            )?;

        let candidate_affiliation_registered_name =
            find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME, &annotations)
                .with_context(|| {
                    format!(
                        "Missing area annotation: '{}:{}'",
                        MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_AFFILIATION_REGISTERED_NAME
                    )
                })?;

        let candidate_affiliation_party =
            find_miru_annotation(MIRU_CANDIDATE_AFFILIATION_PARTY, &annotations).with_context(
                || {
                    format!(
                        "Missing area annotation: '{}:{}'",
                        MIRU_PLUGIN_PREPEND, MIRU_CANDIDATE_AFFILIATION_PARTY
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

#[instrument]
pub fn prepend_miru_annotation(data: &str) -> String {
    format!("{}:{}", MIRU_PLUGIN_PREPEND, data)
}

#[instrument(err, skip(annotations))]
pub fn find_miru_annotation(data: &str, annotations: &Annotations) -> Result<String> {
    let key = prepend_miru_annotation(data);
    annotations
        .get(&key)
        .ok_or(anyhow!("Can't find annotation key {}", key))
        .cloned()
}

#[instrument(err, skip(annotations))]
pub fn find_miru_annotation_opt(data: &str, annotations: &Annotations) -> Result<Option<String>> {
    let key = prepend_miru_annotation(data);
    Ok(annotations.get(&key).cloned())
}

// TODO: Uncomment
#[instrument(err, skip_all)]
pub fn render_eml_contest(
    report: &ReportData,
    area_annotations: &MiruAreaAnnotations,
) -> Result<EMLContest> {
    // Extract contest annotations
    let contest_annotations = report
        .contest
        .get_annotations()
        .with_context(|| "render_eml_contest: ")?;

    let registered_voters = area_annotations.registered_voters;

    let count_metrics = report.contest_result.get_metrics(registered_voters);

    let selections: Vec<EMLSelection> = report
        .contest_result
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
                valid_votes: candidate_result.total_count as i64,
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

#[instrument(err, skip(election_event_annotations, election_annotations, reports))]
pub fn render_eml_file(
    tally_id: &str,
    transaction_id: &str,
    time_zone: TimeZone,
    date_time: DateTime<Utc>,
    election_event_annotations: &MiruElectionEventAnnotations,
    election_annotations: &MiruElectionAnnotations,
    area_annotations: &MiruAreaAnnotations,
    reports: &Vec<ReportData>,
) -> Result<EMLFile> {
    let issue_date = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(ISSUE_DATE_FORMAT.to_string())),
        Some(date_time.clone()),
    );
    let official_status_date = generate_timestamp(
        Some(time_zone.clone()),
        Some(DateFormat::Custom(OFFICIAL_STATUS_DATE_FORMAT.to_string())),
        Some(date_time.clone()),
    );

    let eml_file = EMLFile {
        id: tally_id.to_string(),
        header: EMLHeader {
            transaction_id: transaction_id.to_string(),
            issue_date: issue_date,
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
                    .into_iter()
                    .map(|report| Ok(render_eml_contest(report, area_annotations)?))
                    .collect::<Result<Vec<_>>>()
                    .with_context(|| "Error rendering EML Contest")?,
            }],
        }],
    };
    Ok(eml_file)
}

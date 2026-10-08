// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The timezone variables every report and notification template gets
//! (VOTE-LIFECYCLE design §7): `electionEventTimezone`, `electionTimezone`
//! and the `timezoneTexts` the helpers print with. See
//! `sequent_core::services::reports::template_time_variables`.

use super::utils::get_public_assets_path_env_var;
use crate::postgres::area::get_elections_by_area;
use crate::postgres::election::{get_display_voting_closes, get_election_by_id, get_elections};
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::find_scheduled_event_by_election_event_id;
use crate::services::election_dates::{apply_display_voting_close, get_election_dates};
use crate::services::temp_path::PUBLIC_ASSETS_I18N_DEFAULTS;
use crate::services::time_zones::parse_zone;
use anyhow::{Context, Result};
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use once_cell::sync::Lazy;
use sequent_core::ballot::{ElectionEventPresentation, ElectionPresentation};
use sequent_core::services::reports::{template_time_variables, ELECTION_TIMEZONE_VAR};
use sequent_core::services::s3::get_minio_url;
use sequent_core::services::translations::Name;
use sequent_core::time_zones::{effective_time_zone, primary_time_zone};
use sequent_core::types::hasura::core::ElectionEvent;
use sequent_core::types::scheduled_event::EventProcessors;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::{error, instrument, warn};

/// How long a loaded `i18n_defaults.json` serves before it is fetched again.
const I18N_DEFAULTS_TTL: Duration = Duration::from_secs(10 * 60);
/// How long one fetch of it may take.
const I18N_DEFAULTS_TIMEOUT: Duration = Duration::from_secs(10);

struct CachedDefaults {
    url: String,
    fetched_at: Instant,
    value: Arc<Value>,
}

/// One copy per process (keyed by its URL), refreshed after the TTL.
static I18N_DEFAULTS: Lazy<Mutex<Option<CachedDefaults>>> = Lazy::new(|| Mutex::new(None));

fn i18n_defaults_url() -> Result<String> {
    Ok(format!(
        "{}/{}/{}",
        get_minio_url().context("Error getting the MinIO endpoint")?,
        get_public_assets_path_env_var()?,
        PUBLIC_ASSETS_I18N_DEFAULTS
    ))
}

async fn fetch_i18n_defaults(url: &str) -> Result<Value> {
    let response = reqwest::Client::builder()
        .timeout(I18N_DEFAULTS_TIMEOUT)
        .build()?
        .get(url)
        .send()
        .await
        .with_context(|| format!("Error requesting {url}"))?
        .error_for_status()
        .with_context(|| format!("Error status for {url}"))?;
    let text = response
        .text()
        .await
        .context("Error reading i18n_defaults.json")?;
    serde_json::from_str(&text).context("Error parsing i18n_defaults.json")
}

/// `i18n_defaults.json` from the public assets, loaded once per process and
/// refreshed every [`I18N_DEFAULTS_TTL`]. A template still renders without
/// it (the helpers fall back to the tz database): a failed fetch is logged
/// as an error, serves the previous copy if there is one, and is retried on
/// the next call (a failure is never cached).
#[instrument]
pub async fn load_i18n_defaults() -> Arc<Value> {
    let url = match i18n_defaults_url() {
        Ok(url) => url,
        Err(err) => {
            error!("Timezone texts unavailable, templates use fallbacks: {err:?}");
            return Arc::new(Value::Object(Map::new()));
        }
    };
    let mut cache = I18N_DEFAULTS.lock().await;
    if let Some(cached) = cache.as_ref() {
        if cached.url == url && cached.fetched_at.elapsed() < I18N_DEFAULTS_TTL {
            return cached.value.clone();
        }
    }
    match fetch_i18n_defaults(&url).await {
        Ok(value) => {
            let value = Arc::new(value);
            *cache = Some(CachedDefaults {
                url,
                fetched_at: Instant::now(),
                value: value.clone(),
            });
            value
        }
        Err(err) => {
            error!("Timezone texts unavailable: {err:?}");
            match cache.as_ref() {
                Some(cached) if cached.url == url => {
                    warn!("Using the timezone texts loaded earlier until a fetch succeeds");
                    cached.value.clone()
                }
                _ => Arc::new(Value::Object(Map::new())),
            }
        }
    }
}

/// An event's presentation; an unreadable one is logged and read as empty
/// (so the zone is UTC), as for every other reader of the presentation.
pub fn event_presentation(presentation: Option<&Value>) -> ElectionEventPresentation {
    presentation
        .cloned()
        .map(serde_json::from_value::<ElectionEventPresentation>)
        .transpose()
        .unwrap_or_else(|error| {
            warn!("Unreadable election event presentation: {error:?}");
            None
        })
        .unwrap_or_default()
}

/// An election's presentation (see [`event_presentation`]).
pub fn election_presentation(presentation: Option<&Value>) -> ElectionPresentation {
    presentation
        .cloned()
        .map(serde_json::from_value::<ElectionPresentation>)
        .transpose()
        .unwrap_or_else(|error| {
            warn!("Unreadable election presentation: {error:?}");
            None
        })
        .unwrap_or_default()
}

/// A configured zone for arithmetic; an unknown one is logged and read as
/// UTC (the resolver's last fallback).
pub fn zone_or_utc(name: &str) -> Tz {
    parse_zone(name).unwrap_or_else(|error| {
        warn!("{error:?}; using UTC");
        Tz::UTC
    })
}

/// The timezone variables for a template of `election_event_id` (and
/// `election_id`).
#[instrument(skip(hasura_transaction), err)]
pub async fn load_template_time_variables(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    election_id: Option<&str>,
) -> Result<Map<String, Value>> {
    let election_event = get_election_event_by_id(hasura_transaction, tenant_id, election_event_id)
        .await
        .context("Error loading the election event for the template's timezone")?;
    let event = event_presentation(election_event.presentation.as_ref());
    let election = match election_id {
        Some(election_id) => get_election_by_id(
            hasura_transaction,
            tenant_id,
            election_event_id,
            election_id,
        )
        .await
        .context("Error loading the election for the template's timezone")?
        .map(|election| election_presentation(election.presentation.as_ref())),
        None => None,
    };
    let defaults = load_i18n_defaults().await;
    Ok(template_time_variables(
        Some(&event),
        election.as_ref(),
        &defaults,
    ))
}

/// Adds the timezone variables to a template's variables. A template's own
/// value of the same name wins (preview data may set one on purpose).
pub fn insert_template_time_variables(
    variables: &mut Map<String, Value>,
    time_variables: Map<String, Value>,
) {
    for (key, value) in time_variables {
        variables.entry(key).or_insert(value);
    }
}

/// One election (Post) as a notification names it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct NotificationElection {
    pub id: String,
    pub name: String,
    /// IANA zone of the election (its own, else the primary).
    pub timezone: String,
    /// RFC 3339 instants of its START_VOTING_PERIOD and END_VOTING_PERIOD
    /// (the event-wide close when it has none of its own).
    pub opening: Option<String>,
    pub close: Option<String>,
    /// Zone of the authoritative close; legacy signed dates use the primary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_timezone: Option<String>,
}

/// What a notification template knows about time, loaded once per batch:
/// the event's zone variables, and each voter's election (by area) with its
/// name, opening and close and its zone (design §7).
#[derive(Debug, Clone, Default)]
pub struct NotificationTimeContext {
    event_variables: Map<String, Value>,
    elections: HashMap<String, NotificationElection>,
    elections_by_area: HashMap<String, Vec<String>>,
}

impl NotificationTimeContext {
    pub fn new(
        event_variables: Map<String, Value>,
        elections: Vec<NotificationElection>,
        elections_by_area: HashMap<String, Vec<String>>,
    ) -> Self {
        let elections: HashMap<String, NotificationElection> = elections
            .into_iter()
            .map(|election| (election.id.clone(), election))
            .collect();
        // `get_elections_by_area` has a row per area and contest, in no set
        // order: keep each known election once, ordered by name, then id.
        let elections_by_area = elections_by_area
            .into_iter()
            .map(|(area_id, ids)| {
                let mut known: Vec<&NotificationElection> =
                    ids.iter().filter_map(|id| elections.get(id)).collect();
                known.sort_by(|a, b| (&a.name, &a.id).cmp(&(&b.name, &b.id)));
                known.dedup_by(|a, b| a.id == b.id);
                let ids = known
                    .into_iter()
                    .map(|election| election.id.clone())
                    .collect();
                (area_id, ids)
            })
            .collect();
        NotificationTimeContext {
            event_variables,
            elections,
            elections_by_area,
        }
    }

    /// Loads the context of `election_event`.
    #[instrument(skip_all, err)]
    pub async fn load(
        hasura_transaction: &Transaction<'_>,
        tenant_id: &str,
        election_event: &ElectionEvent,
    ) -> Result<Self> {
        let event = event_presentation(election_event.presentation.as_ref());
        let language = election_event.get_default_language();
        let defaults = load_i18n_defaults().await;
        let event_variables = template_time_variables(Some(&event), None, &defaults);
        let scheduled_events = find_scheduled_event_by_election_event_id(
            hasura_transaction,
            tenant_id,
            &election_event.id,
        )
        .await?;
        let elections = get_elections(hasura_transaction, tenant_id, &election_event.id).await?;
        let closes = get_display_voting_closes(
            hasura_transaction,
            tenant_id,
            &election_event.id,
            &elections
                .iter()
                .map(|election| election.id.clone())
                .collect::<Vec<_>>(),
        )
        .await?;
        let elections = elections
            .into_iter()
            .map(|election| {
                let mut dates =
                    get_election_dates(&election, scheduled_events.clone()).unwrap_or_default();
                apply_display_voting_close(&mut dates, closes.get(&election.id));
                let scheduled_at = |processor: EventProcessors| {
                    dates
                        .scheduled_event_dates
                        .as_ref()
                        .and_then(|dates| dates.get(&processor.to_string()))
                        .and_then(|date| date.scheduled_at.clone())
                };
                NotificationElection {
                    id: election.id.clone(),
                    name: election.get_name(&language),
                    timezone: effective_time_zone(
                        Some(&event),
                        Some(&election_presentation(election.presentation.as_ref())),
                    ),
                    opening: scheduled_at(EventProcessors::START_VOTING_PERIOD),
                    close: scheduled_at(EventProcessors::END_VOTING_PERIOD),
                    close_timezone: Some(
                        closes
                            .get(&election.id)
                            .and_then(|close| close.timezone.clone())
                            .unwrap_or_else(|| primary_time_zone(Some(&event))),
                    ),
                }
            })
            .collect();
        let elections_by_area =
            get_elections_by_area(hasura_transaction, tenant_id, &election_event.id).await?;
        Ok(NotificationTimeContext::new(
            event_variables,
            elections,
            elections_by_area,
        ))
    }

    /// The variables for a voter of `area_id`: `electionEventTimezone`,
    /// `electionTimezone` (the voter's election, else the primary),
    /// `timezoneTexts`, `elections` (the elections of the voter's area, once
    /// each, by name then id) and `election`, the first of them: an area
    /// belongs to one Post in practice, and with several the choice is the
    /// same for every send.
    pub fn variables_for(&self, area_id: Option<&str>) -> Map<String, Value> {
        let mut variables = self.event_variables.clone();
        let elections: Vec<&NotificationElection> = area_id
            .and_then(|area_id| self.elections_by_area.get(area_id))
            .map(|ids| ids.iter().filter_map(|id| self.elections.get(id)).collect())
            .unwrap_or_default();
        if let Some(election) = elections.first() {
            variables.insert(
                ELECTION_TIMEZONE_VAR.to_string(),
                Value::String(election.timezone.clone()),
            );
            variables.insert(
                "election".to_string(),
                serde_json::to_value(election).unwrap_or(Value::Null),
            );
        }
        if !elections.is_empty() {
            variables.insert(
                "elections".to_string(),
                serde_json::to_value(&elections).unwrap_or(Value::Null),
            );
        }
        variables
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::ballot::{ElectionEventTimeZones, LogTimeZonePolicy};
    use sequent_core::services::reports::render_template_text;
    use serde_json::json;

    /// The two configurations of design §10.
    fn configuration(primary: &str, office_zone: &str) -> NotificationTimeContext {
        let mut event = ElectionEventPresentation::default();
        event.timezones = Some(ElectionEventTimeZones {
            configured: vec![primary.to_string(), office_zone.to_string()],
            primary: primary.to_string(),
            logs: LogTimeZonePolicy::ELECTION,
        });
        let defaults = json!({"en": {"timezones": {
            "voterDateTimeZone": "{{dateTime}} {{zoneName}}",
            "name": {
                "Asia/Manila": "Philippine Standard Time",
                "Asia/Dubai": "Gulf Standard Time",
                "Europe/Madrid": "Central European Standard Time",
                "Atlantic/Canary": "Western European Standard Time"
            },
            "nameDaylight": {
                "Europe/Madrid": "Central European Summer Time",
                "Atlantic/Canary": "Western European Summer Time"
            }
        }}});
        NotificationTimeContext::new(
            template_time_variables(Some(&event), None, &defaults),
            vec![NotificationElection {
                close_timezone: None,
                id: "office".to_string(),
                name: "Office".to_string(),
                timezone: office_zone.to_string(),
                opening: Some("2028-04-08T20:00:00Z".to_string()),
                close: Some("2028-05-08T11:00:00Z".to_string()),
            }],
            HashMap::from([("area".to_string(), vec!["office".to_string()])]),
        )
    }

    #[test]
    fn a_voter_gets_the_election_opening_and_close_in_both_zones() {
        for (primary, office_zone) in [
            ("Asia/Manila", "Asia/Dubai"),
            ("Europe/Madrid", "Atlantic/Canary"),
        ] {
            let context = configuration(primary, office_zone);
            let variables = context.variables_for(Some("area"));
            let template = r#"{{election.name}}: {{datetime_zone election.opening output_format="%d %B %Y, %H:%M" style="voter"}} | {{datetime_zone election.close electionEventTimezone output_format="%d %B %Y, %H:%M" style="voter"}} ({{datetime_zone election.close output_format="%H:%M" style="voter"}})"#;
            let texts = sequent_core::services::reports::TimeZoneTexts::from_variables(&variables);
            let at = |instant: &str| {
                chrono::DateTime::parse_from_rfc3339(instant)
                    .unwrap()
                    .with_timezone(&chrono::Utc)
            };
            let voter = sequent_core::services::reports::DateTimeZoneStyle::Voter;
            let expected = format!(
                "Office: {} | {} ({})",
                texts.date_time_zone(
                    at("2028-04-08T20:00:00Z"),
                    office_zone,
                    "%d %B %Y, %H:%M",
                    voter
                ),
                texts.date_time_zone(
                    at("2028-05-08T11:00:00Z"),
                    primary,
                    "%d %B %Y, %H:%M",
                    voter
                ),
                texts.date_time_zone(at("2028-05-08T11:00:00Z"), office_zone, "%H:%M", voter),
            );

            assert_eq!(variables[ELECTION_TIMEZONE_VAR], json!(office_zone));
            assert_eq!(render_template_text(template, variables).unwrap(), expected);
        }
    }

    #[test]
    fn the_comelec_preset_email_reads_like_the_draft() {
        let variables = configuration("Asia/Manila", "Asia/Dubai").variables_for(Some("area"));

        assert_eq!(
            render_template_text(
                r#"{{datetime_zone election.opening output_format="%-d %B %Y, %H:%M" style="voter"}} / {{datetime_zone election.close electionEventTimezone output_format="%-d %B %Y, %H:%M" style="voter"}} / {{datetime_zone election.close output_format="%H:%M" style="voter"}}"#,
                variables
            )
            .unwrap(),
            "9 April 2028, 00:00 Gulf Standard Time / 8 May 2028, 19:00 Philippine Standard Time / 15:00 Gulf Standard Time"
        );
    }

    #[test]
    fn an_area_lists_each_election_once_in_a_stable_order() {
        let election = |id: &str, name: &str| NotificationElection {
            close_timezone: None,
            id: id.to_string(),
            name: name.to_string(),
            timezone: "Asia/Dubai".to_string(),
            opening: None,
            close: None,
        };
        let ids = |list: &[&str]| list.iter().map(|id| id.to_string()).collect::<Vec<_>>();
        for rows in [
            ids(&["b", "a", "b", "a", "missing"]),
            ids(&["a", "a", "b", "missing", "b"]),
        ] {
            let context = NotificationTimeContext::new(
                Map::new(),
                vec![election("b", "Abu Dhabi PE"), election("a", "Dubai PCG")],
                HashMap::from([("area".to_string(), rows)]),
            );
            let variables = context.variables_for(Some("area"));

            assert_eq!(variables["election"]["id"], json!("b"));
            assert_eq!(
                variables["elections"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|election| election["id"].clone())
                    .collect::<Vec<_>>(),
                vec![json!("b"), json!("a")]
            );
        }
    }

    #[test]
    fn a_voter_without_an_election_gets_the_primary() {
        let context = configuration("Europe/Madrid", "Atlantic/Canary");
        let variables = context.variables_for(None);

        assert_eq!(variables[ELECTION_TIMEZONE_VAR], json!("Europe/Madrid"));
        assert!(variables.get("election").is_none());
    }
}

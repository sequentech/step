// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The schedule as a CSV file (VOTE-LIFECYCLE): one row per scheduled event
//! and election, in local time. The preview reads every row and says what it
//! would schedule, or why it can't; the import writes the rows only when none
//! has an error, updating the events it finds by election and type; the
//! export writes the same columns, so an exported file imports back to the
//! same schedule.

use crate::postgres::document::get_document;
use crate::postgres::election_event::get_election_event_by_id;
use crate::postgres::scheduled_event::{
    find_scheduled_event_by_election_event_id, insert_scheduled_event, update_scheduled_event,
};
use crate::services::database::get_hasura_pool;
use crate::services::documents::{get_document_as_temp_file, upload_and_return_document};
use crate::services::enrollment_windows;
use crate::services::scheduled_outcome::{self, PendingChange};
use crate::services::signing::log::{stage, Actor, LogScope, LogStep, SystemOutcome};
use crate::services::time_zone_links::canonical_time_zone;
use crate::services::time_zones::{local_in, parse_local, resolve_local, Resolved};
use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, SecondsFormat, Timelike, Utc};
use chrono_tz::Tz;
use deadpool_postgres::Transaction;
use electoral_log::messages::newtypes::SigningStatementKind;
use sequent_core::ballot::{
    ElectionEventPresentation, ElectionEventTimeZones, ElectionPresentation, VotingStatusChannel,
};
use sequent_core::services::uuid_validation::parse_uuid_v4;
use sequent_core::time_zones::{effective_time_zone, primary_time_zone};
use sequent_core::types::scheduled_event::{
    generate_manage_date_task_name, validate_scheduled_voting_channels, CronConfig,
    EventProcessors, ManageElectionDatePayload, ScheduledEvent,
};
use sequent_core::util::temp_path::write_into_named_temp_file;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek};
use std::str::FromStr;
use tracing::{event, instrument, Level};

/// The `election_alias` of a row that applies to the whole event.
pub const ALL_ELECTIONS: &str = "ALL";

pub const COLUMN_ELECTION_ALIAS: &str = "election_alias";
pub const COLUMN_EVENT_TYPE: &str = "event_type";
pub const COLUMN_LOCAL_DATE_TIME: &str = "local_date_time";
pub const COLUMN_TIMEZONE: &str = "timezone";
pub const COLUMN_VOTING_CHANNELS: &str = "voting_channels";

/// The columns, in the order the export writes them.
pub const COLUMNS: [&str; 5] = [
    COLUMN_ELECTION_ALIAS,
    COLUMN_EVENT_TYPE,
    COLUMN_LOCAL_DATE_TIME,
    COLUMN_TIMEZONE,
    COLUMN_VOTING_CHANNELS,
];

const REQUIRED_COLUMNS: [&str; 3] = [
    COLUMN_ELECTION_ALIAS,
    COLUMN_EVENT_TYPE,
    COLUMN_LOCAL_DATE_TIME,
];

/// Separates the channels of `voting_channels`.
pub const CHANNEL_SEPARATOR: char = '|';

/// Why a row can't be imported. One per row: the first that applies, in
/// this order.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleRowError {
    /// The row has more cells than the header, and one of them isn't empty.
    ExtraCells,
    /// No election of the event has this alias (or name, when it has no
    /// alias), and it isn't `ALL`.
    UnknownElection,
    /// More than one election of the event has this alias.
    AmbiguousElection,
    /// Not an event type, or one that isn't a date (reports, templates).
    UnknownEventType,
    /// The type applies to the whole event only (`ALL`), such as the end of
    /// enrollment.
    EventWideOnly,
    /// The type applies to one election only, such as a test-voting window.
    ElectionOnly,
    /// `local_date_time` isn't `YYYY-MM-DD HH:MM` (seconds optional).
    InvalidDateTime,
    /// `timezone` isn't an IANA timezone (abbreviations like `EST` aren't).
    InvalidTimeZone,
    /// An unknown channel, or a start that opens online and early voting.
    InvalidVotingChannels,
    /// The local time doesn't exist in the zone: clocks go forward then.
    /// The row's `instant` is when it would run instead.
    DstGap,
    /// An earlier row already schedules this type for this election.
    Duplicate,
    /// The import found the event but couldn't change it: it has already
    /// run. Only the import reports it.
    NotUpdated,
}

/// Something about a row that isn't an error but changes what it means.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ScheduleRowNote {
    /// The local time happens twice (clocks go back); the first is used.
    DstOverlap,
}

/// One row of the file as the preview shows it.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PreviewRow {
    /// The file's line number; the header is line 1.
    pub row: u64,
    pub election_alias: String,
    /// The election the alias names; `None` for `ALL` or an unknown alias.
    pub election_id: Option<String>,
    pub election_name: Option<String>,
    pub event_type: String,
    /// The wall time, `YYYY-MM-DDTHH:MM`, or the cell as written when it
    /// can't be read.
    pub local: String,
    /// The zone of `local`: the row's own, else the election's.
    pub time_zone: String,
    /// When it runs, RFC 3339 with the offset of `time_zone`.
    pub instant: Option<String>,
    /// `instant` as wall time in the event's primary zone.
    pub primary_local: Option<String>,
    /// The channels a voting period row opens or closes once imported: the
    /// file's, the defaults for an empty cell, or the event's current ones
    /// when the file has no `voting_channels` column. Empty for other types.
    pub voting_channels: Vec<VotingStatusChannel>,
    pub error_code: Option<ScheduleRowError>,
    pub note_code: Option<ScheduleRowNote>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct SchedulePreview {
    pub rows: Vec<PreviewRow>,
    /// Rows without an error.
    pub ok: u64,
    /// Elections the error-free rows schedule (`ALL` rows aren't counted).
    pub posts: u64,
    /// Rows with an error.
    pub errors: u64,
    pub primary_time_zone: String,
    #[serde(default)]
    pub outcome_changes: Option<Value>,
}

impl SchedulePreview {
    pub fn is_clean(&self) -> bool {
        self.errors == 0
    }

    fn count(&mut self) {
        self.errors = self
            .rows
            .iter()
            .filter(|row| row.error_code.is_some())
            .count() as u64;
        self.ok = self.rows.len() as u64 - self.errors;
    }
}

/// An election as the schedule names it.
#[derive(Debug, Clone)]
pub struct ScheduleElection {
    pub id: String,
    /// What the file calls it: its alias, else its name.
    pub alias: Option<String>,
    pub name: Option<String>,
    /// Only the zone matters here.
    pub time_zone: Option<String>,
}

impl ScheduleElection {
    fn presentation(&self) -> ElectionPresentation {
        ElectionPresentation {
            timezone: self.time_zone.clone(),
            ..Default::default()
        }
    }
}

/// What the schedule needs of the event: its zones and its current
/// scheduled events.
#[derive(Debug, Clone, Default)]
pub struct ScheduleEvent {
    pub tenant_id: String,
    pub election_event_id: String,
    pub time_zones: Option<ElectionEventTimeZones>,
    pub scheduled_events: Vec<ScheduledEvent>,
}

impl ScheduleEvent {
    fn presentation(&self) -> ElectionEventPresentation {
        ElectionEventPresentation {
            timezones: self.time_zones.clone(),
            ..Default::default()
        }
    }

    fn task_id(&self, election_id: Option<&str>, processor: &EventProcessors) -> String {
        generate_manage_date_task_name(
            &self.tenant_id,
            &self.election_event_id,
            election_id,
            processor,
        )
    }

    /// The active scheduled events by task id; the first of a task id wins,
    /// as when an admin saves one.
    fn active_by_task(&self) -> HashMap<&str, &ScheduledEvent> {
        let mut by_task = HashMap::new();
        for scheduled in self
            .scheduled_events
            .iter()
            .filter(|scheduled| scheduled.archived_at.is_none())
        {
            if let Some(task_id) = scheduled.task_id.as_deref() {
                by_task.entry(task_id).or_insert(scheduled);
            }
        }
        by_task
    }
}

/// The channels a row sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleChannels {
    /// The file has no `voting_channels` column: an existing event keeps its
    /// channels; a new one gets the defaults.
    Unchanged,
    /// An empty cell: the defaults.
    Default,
    /// These channels.
    Set(Vec<VotingStatusChannel>),
}

impl ScheduleChannels {
    /// What the payload's `voting_channels` gets on an update (`None`:
    /// left as it is) and on an insert.
    fn for_update(&self) -> Option<Vec<VotingStatusChannel>> {
        match self {
            ScheduleChannels::Unchanged => None,
            ScheduleChannels::Default => Some(vec![]),
            ScheduleChannels::Set(channels) => Some(channels.clone()),
        }
    }

    fn for_insert(&self) -> Option<Vec<VotingStatusChannel>> {
        match self {
            ScheduleChannels::Set(channels) => Some(channels.clone()),
            _ => None,
        }
    }
}

/// A row that can be written: everything the preview checked, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleEntry {
    pub row: u64,
    pub election_id: Option<String>,
    pub event_processor: EventProcessors,
    /// `YYYY-MM-DDTHH:MM`.
    pub local: String,
    pub time_zone: String,
    /// RFC 3339 with the zone's offset.
    pub scheduled_date: String,
    pub voting_channels: ScheduleChannels,
}

impl ScheduleEntry {
    pub fn cron_config(&self) -> CronConfig {
        CronConfig {
            cron: None,
            scheduled_date: Some(self.scheduled_date.clone()),
            local: Some(self.local.clone()),
            timezone: Some(self.time_zone.clone()),
        }
    }

    pub fn task_id(&self, tenant_id: &str, election_event_id: &str) -> String {
        generate_manage_date_task_name(
            tenant_id,
            election_event_id,
            self.election_id.as_deref(),
            &self.event_processor,
        )
    }
}

/// The file read against the event: what the preview shows, and the rows
/// the import would write (those without an error).
#[derive(Debug, Clone)]
pub struct ParsedSchedule {
    pub preview: SchedulePreview,
    pub entries: Vec<ScheduleEntry>,
}

/// Event types that are dates: everything but reports and templates, as
/// `manage_election_dates` accepts.
pub fn is_schedulable(processor: &EventProcessors) -> bool {
    !matches!(
        processor,
        EventProcessors::CREATE_REPORT | EventProcessors::SEND_TEMPLATE
    )
}

/// Types that apply to the whole event only (design §5: enrollment ends
/// for the event at once).
pub fn is_event_wide_only(processor: &EventProcessors) -> bool {
    matches!(processor, EventProcessors::END_ENROLLMENT_PERIOD)
}

/// Types that apply to one election only (the lifecycle windows).
pub fn is_election_only(processor: &EventProcessors) -> bool {
    matches!(
        processor,
        EventProcessors::START_READINESS_TEST
            | EventProcessors::END_READINESS_TEST
            | EventProcessors::START_FINAL_TESTING
            | EventProcessors::END_FINAL_TESTING
            | EventProcessors::START_TEST_VOTING
            | EventProcessors::END_TEST_VOTING
    )
}

fn opens_or_closes_voting(processor: &EventProcessors) -> bool {
    matches!(
        processor,
        EventProcessors::START_VOTING_PERIOD | EventProcessors::END_VOTING_PERIOD
    )
}

fn format_local(local: NaiveDateTime) -> String {
    if local.second() == 0 {
        local.format("%Y-%m-%dT%H:%M").to_string()
    } else {
        local.format("%Y-%m-%dT%H:%M:%S").to_string()
    }
}

/// `instant` in RFC 3339 with the offset `zone` has then.
pub fn format_instant(instant: DateTime<Utc>, zone: Tz) -> String {
    instant
        .with_timezone(&zone)
        .fixed_offset()
        .to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// A wall time as spreadsheets write it: a space or `T` between the date and
/// the time.
fn read_local(cell: &str) -> Option<NaiveDateTime> {
    let cell = cell.trim();
    parse_local(cell)
        .or_else(|_| parse_local(&cell.replacen(' ', "T", 1)))
        .ok()
}

fn read_channels(cell: &str) -> Option<Result<Vec<VotingStatusChannel>, ()>> {
    let names: Vec<&str> = cell
        .split(CHANNEL_SEPARATOR)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect();
    if names.is_empty() {
        return None;
    }
    Some(
        names
            .into_iter()
            .map(|name| VotingStatusChannel::from_str(name).map_err(|_| ()))
            .collect(),
    )
}

/// The channels in the order they are applied, defaults for none.
fn effective_channels(channels: Option<Vec<VotingStatusChannel>>) -> Vec<VotingStatusChannel> {
    ManageElectionDatePayload {
        election_id: None,
        voting_channels: channels,
    }
    .channels()
}

fn payload_of(scheduled: &ScheduledEvent) -> ManageElectionDatePayload {
    scheduled
        .event_payload
        .clone()
        .and_then(|payload| serde_json::from_value(payload).ok())
        .unwrap_or_default()
}

/// The file can't be read as a schedule: no header, a missing or unknown
/// column, broken CSV. Nothing in it is previewed.
#[derive(Debug)]
pub struct ScheduleFileError(pub String);

impl std::fmt::Display for ScheduleFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ScheduleFileError {}

fn file_error(message: String) -> anyhow::Error {
    ScheduleFileError(message).into()
}

/// Keeps the first error of a row.
fn fail(code: ScheduleRowError, error: &mut Option<ScheduleRowError>) {
    if error.is_none() {
        *error = Some(code);
    }
}

/// Column positions by header name.
struct Header {
    /// How many cells a row may have.
    width: usize,
    election_alias: usize,
    event_type: usize,
    local_date_time: usize,
    timezone: Option<usize>,
    voting_channels: Option<usize>,
}

fn read_header(header: &csv::StringRecord) -> Result<Header> {
    let mut names: Vec<String> = header
        .iter()
        .map(|name| name.trim_start_matches('\u{feff}').trim().to_string())
        .collect();
    // A trailing comma (spreadsheets write one): one empty last column.
    if names.len() > 1 && names.last().is_some_and(String::is_empty) {
        names.pop();
    }
    if let Some(unknown) = names.iter().find(|name| !COLUMNS.contains(&name.as_str())) {
        return Err(file_error(format!(
            "Unknown column {unknown:?}; the columns are {}",
            COLUMNS.join(", ")
        )));
    }
    let position = |column: &str| names.iter().position(|name| name == column);
    for column in COLUMNS {
        if names.iter().filter(|name| *name == column).count() > 1 {
            return Err(file_error(format!("Column {column:?} appears twice")));
        }
    }
    for column in REQUIRED_COLUMNS {
        if position(column).is_none() {
            return Err(file_error(format!("Missing column {column:?}")));
        }
    }
    Ok(Header {
        width: names.len(),
        election_alias: position(COLUMN_ELECTION_ALIAS).unwrap_or_default(),
        event_type: position(COLUMN_EVENT_TYPE).unwrap_or_default(),
        local_date_time: position(COLUMN_LOCAL_DATE_TIME).unwrap_or_default(),
        timezone: position(COLUMN_TIMEZONE),
        voting_channels: position(COLUMN_VOTING_CHANNELS),
    })
}

/// The line a record starts on (the header is line 1), whatever the line
/// endings and however many empty lines come before it.
fn line_of(csv_bytes: &[u8], byte: u64) -> u64 {
    let mut start = (byte as usize).min(csv_bytes.len());
    while start < csv_bytes.len() && matches!(csv_bytes[start], b'\r' | b'\n') {
        start += 1;
    }
    1 + csv_bytes[..start]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count() as u64
}

/// Reads the file against the event and its elections. An `Err` is a file
/// that isn't a schedule at all (no header, a missing or unknown column,
/// broken CSV); a problem with a row is that row's `error_code`.
pub fn parse_schedule(
    csv_bytes: &[u8],
    event: &ScheduleEvent,
    elections: &[ScheduleElection],
) -> Result<ParsedSchedule> {
    let event_presentation = event.presentation();
    let primary_name = primary_time_zone(Some(&event_presentation));
    let primary = canonical_time_zone(&primary_name)
        .ok_or_else(|| anyhow!("The event's primary timezone {primary_name:?} is unknown"))?;
    let existing = event.active_by_task();

    let mut by_alias: HashMap<&str, Vec<&ScheduleElection>> = HashMap::new();
    for election in elections {
        if let Some(alias) = election.alias.as_deref() {
            by_alias.entry(alias.trim()).or_default().push(election);
        }
    }

    let mut reader = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .trim(csv::Trim::None)
        .from_reader(csv_bytes);
    let header = read_header(
        reader
            .headers()
            .map_err(|e| file_error(format!("Error reading the CSV header: {e}")))?,
    )?;

    let mut rows = Vec::new();
    let mut entries = Vec::new();
    let mut seen: HashSet<(Option<String>, String)> = HashSet::new();

    for record in reader.records() {
        let record = record.map_err(|e| file_error(format!("Error reading the CSV: {e}")))?;
        if record.iter().all(|cell| cell.trim().is_empty()) {
            continue;
        }
        let row = record
            .position()
            .map(|position| line_of(csv_bytes, position.byte()))
            .unwrap_or_default();
        let cell = |position: Option<usize>| {
            position
                .and_then(|position| record.get(position))
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let election_alias = cell(Some(header.election_alias));
        let event_type = cell(Some(header.event_type));
        let local_cell = cell(Some(header.local_date_time));
        let zone_cell = cell(header.timezone);
        let channels_cell = cell(header.voting_channels);

        let mut error: Option<ScheduleRowError> = None;
        let mut note = None;

        if record
            .iter()
            .skip(header.width)
            .any(|extra| !extra.trim().is_empty())
        {
            fail(ScheduleRowError::ExtraCells, &mut error);
        }

        // Election: `ALL` is the event; anything else is an alias.
        let event_wide = election_alias == ALL_ELECTIONS;
        let election: Option<&ScheduleElection> = if event_wide {
            None
        } else {
            match by_alias.get(election_alias.as_str()).map(Vec::as_slice) {
                Some([election]) => Some(*election),
                Some([_, _, ..]) => {
                    fail(ScheduleRowError::AmbiguousElection, &mut error);
                    None
                }
                _ => {
                    fail(ScheduleRowError::UnknownElection, &mut error);
                    None
                }
            }
        };
        let election_known = event_wide || election.is_some();

        let processor = EventProcessors::from_str(&event_type)
            .ok()
            .filter(is_schedulable);
        match processor.as_ref() {
            None => fail(ScheduleRowError::UnknownEventType, &mut error),
            Some(processor) if election.is_some() && is_event_wide_only(processor) => {
                fail(ScheduleRowError::EventWideOnly, &mut error)
            }
            Some(processor) if event_wide && is_election_only(processor) => {
                fail(ScheduleRowError::ElectionOnly, &mut error)
            }
            Some(_) => {}
        }

        let local = read_local(&local_cell);
        if local.is_none() {
            fail(ScheduleRowError::InvalidDateTime, &mut error);
        }

        let zone_name = if zone_cell.is_empty() {
            effective_time_zone(
                Some(&event_presentation),
                election.map(ScheduleElection::presentation).as_ref(),
            )
        } else {
            zone_cell.clone()
        };
        let zone = canonical_time_zone(&zone_name);
        if zone.is_none() {
            fail(ScheduleRowError::InvalidTimeZone, &mut error);
        }
        let time_zone = zone
            .map(|zone| zone.name().to_string())
            .unwrap_or(zone_name);

        let voting_channels = if header.voting_channels.is_none() {
            ScheduleChannels::Unchanged
        } else {
            match read_channels(&channels_cell) {
                None => ScheduleChannels::Default,
                Some(Ok(channels)) => {
                    let valid = processor.as_ref().map_or(true, |processor| {
                        validate_scheduled_voting_channels(processor, Some(&channels)).is_ok()
                    });
                    if !valid {
                        fail(ScheduleRowError::InvalidVotingChannels, &mut error);
                    }
                    ScheduleChannels::Set(channels)
                }
                Some(Err(())) => {
                    fail(ScheduleRowError::InvalidVotingChannels, &mut error);
                    ScheduleChannels::Default
                }
            }
        };

        let mut instant = None;
        if let (Some(local), Some(zone)) = (local, zone) {
            let resolved = resolve_local(local, zone);
            match resolved {
                Resolved::Exact(_) => {}
                Resolved::Gap { .. } => fail(ScheduleRowError::DstGap, &mut error),
                Resolved::Overlap { .. } => note = Some(ScheduleRowNote::DstOverlap),
            }
            instant = Some(resolved.instant());
        }

        // A second row for the same election and type is a duplicate, even
        // when the first has an error of its own: the file says it twice.
        if let (true, Some(processor)) = (election_known, processor.as_ref()) {
            let key = (
                election.map(|election| election.id.clone()),
                processor.to_string(),
            );
            if !seen.insert(key) {
                fail(ScheduleRowError::Duplicate, &mut error);
            }
        }

        let shown_channels = match processor.as_ref() {
            Some(processor) if opens_or_closes_voting(processor) => {
                let current = || {
                    existing
                        .get(
                            event
                                .task_id(election.map(|e| e.id.as_str()), processor)
                                .as_str(),
                        )
                        .and_then(|scheduled| payload_of(scheduled).voting_channels)
                };
                effective_channels(match &voting_channels {
                    ScheduleChannels::Unchanged => current(),
                    ScheduleChannels::Default => None,
                    ScheduleChannels::Set(channels) => Some(channels.clone()),
                })
            }
            _ => vec![],
        };

        let preview_row = PreviewRow {
            row,
            election_alias,
            election_id: election.map(|election| election.id.clone()),
            election_name: election.and_then(|election| election.name.clone()),
            event_type,
            local: local.map(format_local).unwrap_or(local_cell),
            time_zone: time_zone.clone(),
            instant: instant.zip(zone).map(|(at, zone)| format_instant(at, zone)),
            primary_local: instant.map(|at| format_local(local_in(primary, at))),
            voting_channels: shown_channels,
            error_code: error,
            note_code: note,
        };

        if error.is_none() {
            if let (Some(processor), Some(local), Some(instant), Some(zone)) =
                (processor, local, instant, zone)
            {
                entries.push(ScheduleEntry {
                    row,
                    election_id: election.map(|election| election.id.clone()),
                    event_processor: processor,
                    local: format_local(local),
                    time_zone,
                    scheduled_date: format_instant(instant, zone),
                    voting_channels,
                });
            }
        }
        rows.push(preview_row);
    }

    let posts = entries
        .iter()
        .filter_map(|entry| entry.election_id.as_deref())
        .collect::<HashSet<_>>()
        .len() as u64;
    let mut preview = SchedulePreview {
        rows,
        ok: 0,
        posts,
        errors: 0,
        primary_time_zone: primary.name().to_string(),
        outcome_changes: None,
    };
    preview.count();
    Ok(ParsedSchedule { preview, entries })
}

/// What the import does with an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Upsert {
    Create,
    /// The id of the scheduled event it replaces the date of.
    Update(String),
}

/// Pairs each entry with the active scheduled event of the same election and
/// type (same task id), if there is one.
pub fn plan_import(
    entries: &[ScheduleEntry],
    event: &ScheduleEvent,
) -> Vec<(ScheduleEntry, Upsert)> {
    let by_task = event.active_by_task();
    entries
        .iter()
        .map(|entry| {
            let task_id = entry.task_id(&event.tenant_id, &event.election_event_id);
            let action = match by_task.get(task_id.as_str()) {
                Some(scheduled) => Upsert::Update(scheduled.id.clone()),
                None => Upsert::Create,
            };
            (entry.clone(), action)
        })
        .collect()
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportOutcome {
    pub created: u64,
    pub updated: u64,
}

/// The language aliases and names are read in when an election and its
/// event say none.
const FALLBACK_LANGUAGE: &str = "en";

fn default_language(presentation: &Value) -> Option<String> {
    presentation
        .pointer("/language_conf/default_language_code")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(str::to_string)
}

fn i18n_text(presentation: &Value, language: &str, field: &str) -> Option<String> {
    presentation
        .get("i18n")
        .and_then(|i18n| i18n.get(language))
        .and_then(|texts| texts.get(field))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// An election as the schedule names it, as the Admin Portal shows it: its
/// alias, else its name, in its default language (else the event's), else
/// in English; and its zone.
pub fn schedule_election(
    id: String,
    presentation: Option<&Value>,
    event_language: &str,
) -> ScheduleElection {
    let empty = Value::Null;
    let presentation = presentation.unwrap_or(&empty);
    let language = default_language(presentation).unwrap_or_else(|| event_language.to_string());
    let name = i18n_text(presentation, &language, "name")
        .or_else(|| i18n_text(presentation, FALLBACK_LANGUAGE, "name"));
    let alias = i18n_text(presentation, &language, "alias")
        .or_else(|| i18n_text(presentation, &language, "name"))
        .or_else(|| i18n_text(presentation, FALLBACK_LANGUAGE, "alias"))
        .or_else(|| i18n_text(presentation, FALLBACK_LANGUAGE, "name"));
    ScheduleElection {
        id,
        alias,
        name,
        time_zone: presentation
            .get("timezone")
            .and_then(Value::as_str)
            .map(str::to_string),
    }
}

/// The event, its elections and its scheduled events as the schedule reads
/// them.
#[instrument(skip(hasura_transaction), err)]
pub async fn load_schedule_context(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<(ScheduleEvent, Vec<ScheduleElection>)> {
    let election_event =
        get_election_event_by_id(hasura_transaction, tenant_id, election_event_id).await?;
    let time_zones = election_event
        .presentation
        .as_ref()
        .and_then(|presentation| presentation.get("timezones"))
        .filter(|value| !value.is_null())
        .map(|value| serde_json::from_value::<ElectionEventTimeZones>(value.clone()))
        .transpose()
        .context("Error reading the event's timezones")?;

    let event_language = election_event
        .presentation
        .as_ref()
        .and_then(default_language)
        .unwrap_or_else(|| FALLBACK_LANGUAGE.to_string());

    let tenant_uuid = parse_uuid_v4(tenant_id).context("Error parsing tenant_id")?;
    let event_uuid = parse_uuid_v4(election_event_id).context("Error parsing election_event_id")?;
    let statement = hasura_transaction
        .prepare(
            r#"
            SELECT id, presentation
            FROM sequent_backend.election
            WHERE tenant_id = $1 AND election_event_id = $2
            "#,
        )
        .await?;
    let elections = hasura_transaction
        .query(&statement, &[&tenant_uuid, &event_uuid])
        .await
        .context("Error reading the event's elections")?
        .into_iter()
        .map(|row| -> Result<ScheduleElection> {
            let presentation: Option<Value> = row.try_get("presentation")?;
            Ok(schedule_election(
                row.try_get::<_, uuid::Uuid>("id")?.to_string(),
                presentation.as_ref(),
                &event_language,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    let scheduled_events =
        find_scheduled_event_by_election_event_id(hasura_transaction, tenant_id, election_event_id)
            .await?;
    Ok((
        ScheduleEvent {
            tenant_id: tenant_id.to_string(),
            election_event_id: election_event_id.to_string(),
            time_zones,
            scheduled_events,
        },
        elections,
    ))
}

/// Reads the file against the event as it is in the database.
#[instrument(skip(hasura_transaction, csv_bytes), err)]
pub async fn preview_schedule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    csv_bytes: &[u8],
) -> Result<ParsedSchedule> {
    let (event, elections) =
        load_schedule_context(hasura_transaction, tenant_id, election_event_id).await?;
    let mut parsed = parse_schedule(csv_bytes, &event, &elections)?;
    if parsed.preview.is_clean() {
        let changes = plan_import(&parsed.entries, &event)
            .into_iter()
            .map(|(entry, upsert)| {
                let (id, mut payload) = match upsert {
                    Upsert::Update(id) => {
                        let payload = event
                            .scheduled_events
                            .iter()
                            .find(|row| row.id == id)
                            .and_then(|row| row.event_payload.clone())
                            .unwrap_or_else(|| json!({}));
                        (id, payload)
                    }
                    Upsert::Create => (
                        uuid::Uuid::new_v4().to_string(),
                        serde_json::to_value(ManageElectionDatePayload {
                            election_id: entry.election_id.clone(),
                            voting_channels: entry.voting_channels.for_insert(),
                        })?,
                    ),
                };
                if let Some(channels) = entry.voting_channels.for_update() {
                    payload["voting_channels"] = serde_json::to_value(channels)?;
                }
                Ok(PendingChange::ScheduledEvent {
                    id: Some(id),
                    event_processor: entry.event_processor.to_string(),
                    cron_config: Some(serde_json::to_value(entry.cron_config())?),
                    event_payload: Some(payload),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        parsed.preview.outcome_changes = Some(serde_json::to_value(
            scheduled_outcome::preview_changes(
                hasura_transaction,
                parse_uuid_v4(tenant_id)?,
                parse_uuid_v4(election_event_id)?,
                &changes,
            )
            .await?,
        )?);
    }
    Ok(parsed)
}

/// The import refused: the file has rows with errors. Nothing was written.
#[derive(Debug)]
pub struct RowsWithErrors(pub SchedulePreview);

impl std::fmt::Display for RowsWithErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let rows: Vec<String> = self
            .0
            .rows
            .iter()
            .filter(|row| row.error_code.is_some())
            .map(|row| row.row.to_string())
            .collect();
        write!(
            f,
            "{} rows need attention (rows {}); nothing was imported",
            self.0.errors,
            rows.join(", ")
        )
    }
}

impl std::error::Error for RowsWithErrors {}

/// Writes the file's rows: creates the events that don't exist and updates
/// the date, timezone and channels of those that do. Refuses when any row
/// has an error, or when an event it found can't be changed (it has already
/// run, `not-updated`); then the caller must not commit. It takes the
/// event's signing lock first, so imports (and signing steps) of the event
/// run one after another and the plan sees the rows a previous import wrote.
#[instrument(skip(hasura_transaction, csv_bytes), err)]
pub async fn import_schedule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    csv_bytes: &[u8],
) -> Result<(ImportOutcome, Vec<ScheduleEntry>)> {
    crate::postgres::scheduled_event::lock_scheduling_event(
        hasura_transaction,
        tenant_id,
        election_event_id,
    )
    .await?;
    let (event, elections) =
        load_schedule_context(hasura_transaction, tenant_id, election_event_id).await?;
    let mut parsed = parse_schedule(csv_bytes, &event, &elections)?;
    if !parsed.preview.is_clean() {
        return Err(RowsWithErrors(parsed.preview).into());
    }
    let mut outcome = ImportOutcome::default();
    let mut not_updated = HashSet::new();
    for (entry, action) in plan_import(&parsed.entries, &event) {
        match action {
            Upsert::Update(id) => {
                let changed = update_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    &id,
                    entry.cron_config(),
                    entry.voting_channels.for_update().as_ref(),
                )
                .await
                .with_context(|| format!("Error updating the event of row {}", entry.row))?;
                if changed == 0 {
                    not_updated.insert(entry.row);
                } else {
                    outcome.updated += 1;
                }
            }
            Upsert::Create => {
                let payload = ManageElectionDatePayload {
                    election_id: entry.election_id.clone(),
                    voting_channels: entry.voting_channels.for_insert(),
                };
                insert_scheduled_event(
                    hasura_transaction,
                    tenant_id,
                    election_event_id,
                    entry.event_processor.clone(),
                    &entry.task_id(tenant_id, election_event_id),
                    entry.cron_config(),
                    serde_json::to_value(payload)?,
                )
                .await
                .with_context(|| format!("Error creating the event of row {}", entry.row))?;
                outcome.created += 1;
            }
        }
    }
    if !not_updated.is_empty() {
        for row in parsed.preview.rows.iter_mut() {
            if not_updated.contains(&row.row) {
                row.error_code = Some(ScheduleRowError::NotUpdated);
            }
        }
        parsed.preview.count();
        return Err(RowsWithErrors(parsed.preview).into());
    }
    Ok((outcome, parsed.entries))
}

/// One exported row, in [`COLUMNS`] order.
fn export_row(
    scheduled: &ScheduledEvent,
    event: &ScheduleEvent,
    elections: &HashMap<&str, &ScheduleElection>,
) -> Option<[String; 5]> {
    let processor = scheduled
        .event_processor
        .as_ref()
        .filter(|p| is_schedulable(p))?;
    let cron_config = scheduled.cron_config.as_ref()?;
    let scheduled_date = cron_config.scheduled_date.as_deref()?;
    let payload = payload_of(scheduled);
    let election = match payload.election_id.as_deref() {
        None => None,
        Some(id) => Some(*elections.get(id)?),
    };
    let alias = match election {
        None => ALL_ELECTIONS.to_string(),
        Some(election) => election.alias.clone()?,
    };
    let (local, time_zone) = match (
        cron_config.local.as_deref(),
        cron_config.timezone.as_deref(),
    ) {
        (Some(local), Some(zone)) => (local.to_string(), zone.to_string()),
        _ => {
            // Saved before the wall time was kept: the instant, read in the
            // row's zone.
            let instant = DateTime::parse_from_rfc3339(scheduled_date).ok()?;
            let zone_name = effective_time_zone(
                Some(&event.presentation()),
                election.map(ScheduleElection::presentation).as_ref(),
            );
            let zone = canonical_time_zone(&zone_name)?;
            (
                format_local(local_in(zone, instant.with_timezone(&Utc))),
                zone.name().to_string(),
            )
        }
    };
    let channels = payload
        .voting_channels
        .unwrap_or_default()
        .iter()
        .map(|channel| channel.as_ref().to_string())
        .collect::<Vec<_>>()
        .join(&CHANNEL_SEPARATOR.to_string());
    Some([
        alias,
        processor.to_string(),
        // `T` keeps spreadsheets from turning the cell into a date of their own.
        local,
        time_zone,
        channels,
    ])
}

/// The schedule as CSV: event-wide rows first, then by election alias and
/// type. Rows that can't be written as a wall time (no date, an unknown
/// election, a date without an offset) are left out and logged.
pub fn export_schedule_csv(
    event: &ScheduleEvent,
    elections: &[ScheduleElection],
) -> Result<String> {
    let by_id: HashMap<&str, &ScheduleElection> = elections
        .iter()
        .map(|election| (election.id.as_str(), election))
        .collect();
    let mut rows: Vec<[String; 5]> = event
        .scheduled_events
        .iter()
        .filter(|scheduled| scheduled.archived_at.is_none())
        .filter(|scheduled| {
            scheduled
                .event_processor
                .as_ref()
                .is_some_and(is_schedulable)
        })
        .filter_map(|scheduled| {
            let row = export_row(scheduled, event, &by_id);
            if row.is_none() {
                event!(
                    Level::WARN,
                    "Scheduled event {} left out of the schedule export: no date with an offset, \
                     or an election without an alias or name",
                    scheduled.id
                );
            }
            row
        })
        .collect();
    rows.sort_by(|a, b| {
        (a[0] != ALL_ELECTIONS, &a[0], &a[1]).cmp(&(b[0] != ALL_ELECTIONS, &b[0], &b[1]))
    });
    let mut writer = csv::Writer::from_writer(vec![]);
    writer.write_record(COLUMNS)?;
    for row in rows {
        writer.write_record(&row)?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| anyhow!("Error writing the schedule CSV: {e:?}"))?;
    Ok(String::from_utf8(bytes)?)
}

/// The event's schedule as CSV, read from the database.
#[instrument(skip(hasura_transaction), err)]
pub async fn export_schedule(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
) -> Result<String> {
    let (event, elections) =
        load_schedule_context(hasura_transaction, tenant_id, election_event_id).await?;
    export_schedule_csv(&event, &elections)
}

/// A scheduled event's cron config as an election event import brings it:
/// a `scheduled_date` with an offset is kept; one without an offset is
/// recomputed from `local` + `timezone` when both are there, and refused
/// otherwise (the instant it means can't be known).
pub fn checked_import_cron_config(mut cron_config: CronConfig) -> Result<CronConfig> {
    // Metadata must be valid even when the archive already supplies an instant.
    // Keep that instant: older tzdata and a selected overlap occurrence are
    // legitimate reasons it may differ from today's wall-time resolution.
    let zone = cron_config
        .timezone
        .as_deref()
        .map(|name| {
            canonical_time_zone(name)
                .ok_or_else(|| anyhow!("Unknown timezone {name:?} of the scheduled event"))
        })
        .transpose()?;
    let local_time = cron_config
        .local
        .as_deref()
        .map(|local| {
            read_local(local)
                .ok_or_else(|| anyhow!("Invalid local time {local:?} of the scheduled event"))
        })
        .transpose()?;
    anyhow::ensure!(
        local_time.is_none() || zone.is_some(),
        "A scheduled event's local time requires a timezone"
    );
    if let Some(zone) = zone {
        cron_config.timezone = Some(zone.name().to_string());
    }
    if let Some(local_time) = local_time {
        cron_config.local = Some(format_local(local_time));
    }
    let Some(scheduled_date) = cron_config.scheduled_date.as_deref() else {
        return Ok(cron_config);
    };
    if DateTime::parse_from_rfc3339(scheduled_date.trim()).is_ok() {
        // The scheduler parses the stored string directly, without trimming.
        cron_config.scheduled_date = Some(scheduled_date.trim().to_string());
        return Ok(cron_config);
    }
    let (Some(local_time), Some(zone)) = (local_time, zone) else {
        return Err(anyhow!(
            "The scheduled date {scheduled_date:?} has no UTC offset, and no local time and \
             timezone to compute it from"
        ));
    };
    let instant = match resolve_local(local_time, zone) {
        Resolved::Gap { .. } => {
            return Err(anyhow!(
                "The local time {local_time} doesn't exist in {}: clocks go forward then",
                zone.name()
            ))
        }
        resolved => resolved.instant(),
    };
    Ok(CronConfig {
        scheduled_date: Some(format_instant(instant, zone)),
        local: Some(format_local(local_time)),
        timezone: Some(zone.name().to_string()),
        ..cron_config
    })
}

/// Who imports or exports the schedule.
#[derive(Debug, Clone)]
pub struct ScheduleAuthor {
    pub user_id: String,
    pub username: Option<String>,
}

/// The electoral log entry of an import: the file, its digest and the
/// counts. Staged in the import's transaction, so a committed import always
/// has its entry.
pub fn schedule_import_log_step(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    csv_bytes: &[u8],
    outcome: &ImportOutcome,
    author: &ScheduleAuthor,
) -> Result<LogStep> {
    let file_sha256 = hex::encode(Sha256::digest(csv_bytes));
    Ok(LogStep {
        kind: SigningStatementKind::ScheduleImported,
        user: Actor {
            user_id: author.user_id.clone(),
            username: author
                .username
                .clone()
                .unwrap_or_else(|| author.user_id.clone()),
        },
        system: SystemOutcome::Info,
        scope: LogScope {
            tenant_id: parse_uuid_v4(tenant_id).context("Error parsing tenant_id")?,
            election_event_id: parse_uuid_v4(election_event_id)
                .context("Error parsing election_event_id")?,
            election_id: None,
            area_id: None,
        },
        description: format!(
            "Imported the schedule: {} scheduled events created and {} updated (file {file_sha256}).",
            outcome.created, outcome.updated
        ),
        details: json!({
            "document_id": document_id,
            "file_sha256": file_sha256,
            "created": outcome.created,
            "updated": outcome.updated,
        }),
    })
}

/// The bytes of an uploaded document of the event.
#[instrument(skip(hasura_transaction), err)]
pub async fn read_schedule_document(
    hasura_transaction: &Transaction<'_>,
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
) -> Result<Vec<u8>> {
    let document = get_document(
        hasura_transaction,
        tenant_id,
        Some(election_event_id.to_string()),
        document_id,
    )
    .await?
    .ok_or_else(|| anyhow!("Document {document_id} not found"))?;
    let mut file = get_document_as_temp_file(tenant_id, &document).await?;
    file.rewind()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// The preview of an uploaded schedule file. Writes nothing.
#[instrument(err)]
pub async fn preview_schedule_document(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
) -> Result<SchedulePreview> {
    let mut client = get_hasura_pool().await.get().await?;
    let hasura_transaction = client.transaction().await?;
    let bytes = read_schedule_document(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        document_id,
    )
    .await?;
    let parsed =
        preview_schedule(&hasura_transaction, tenant_id, election_event_id, &bytes).await?;
    hasura_transaction.rollback().await?;
    Ok(parsed.preview)
}

/// Imports an uploaded schedule file with its electoral log entry, in one
/// transaction. A file with row errors is refused ([`RowsWithErrors`]) and
/// nothing is written.
#[instrument(err)]
pub async fn import_schedule_document(
    tenant_id: &str,
    election_event_id: &str,
    document_id: &str,
    author: &ScheduleAuthor,
) -> Result<ImportOutcome> {
    let mut client = get_hasura_pool().await.get().await?;
    let hasura_transaction = client.transaction().await?;
    let bytes = read_schedule_document(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        document_id,
    )
    .await?;
    let (outcome, entries) =
        import_schedule(&hasura_transaction, tenant_id, election_event_id, &bytes).await?;

    let refresh_enrollment = entries.iter().any(|entry| {
        matches!(
            entry.event_processor,
            EventProcessors::START_ENROLLMENT_PERIOD | EventProcessors::END_ENROLLMENT_PERIOD
        )
    });

    let step = schedule_import_log_step(
        tenant_id,
        election_event_id,
        document_id,
        &bytes,
        &outcome,
        author,
    )?;
    stage(&hasura_transaction, &step).await?;
    scheduled_outcome::recompute_predictions(
        &hasura_transaction,
        tenant_id,
        election_event_id,
        &step.user,
    )
    .await?;
    if refresh_enrollment {
        enrollment_windows::begin_synchronization(
            &hasura_transaction,
            tenant_id,
            election_event_id,
        )
        .await?;
    }
    hasura_transaction.commit().await?;
    if refresh_enrollment {
        enrollment_windows::complete_synchronization(tenant_id, election_event_id).await?;
    }
    Ok(outcome)
}

/// Exports the schedule into a new document of the event and returns its id.
#[instrument(err)]
pub async fn export_schedule_document(tenant_id: &str, election_event_id: &str) -> Result<String> {
    let mut client = get_hasura_pool().await.get().await?;
    let hasura_transaction = client.transaction().await?;
    let csv = export_schedule(&hasura_transaction, tenant_id, election_event_id).await?;
    let name = format!("schedule-{election_event_id}");
    let (_temp_path, temp_path_string, file_size) =
        write_into_named_temp_file(&csv.into_bytes(), &name, ".csv")
            .context("Error writing the schedule into a temporary file")?;
    let document = upload_and_return_document(
        &hasura_transaction,
        &temp_path_string,
        file_size,
        "text/csv",
        tenant_id,
        Some(election_event_id.to_string()),
        &format!("{name}.csv"),
        None,
        false,
    )
    .await?;
    hasura_transaction.commit().await?;
    Ok(document.id)
}

#[cfg(test)]
#[path = "schedule_csv_tests.rs"]
mod schedule_csv_tests;

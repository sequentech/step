// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Reviewed historical seat/precinct SQL plus a supplemental workbook.
//! This versioned profile is deterministic, local and data-only. It deliberately
//! requires decisions the historical files cannot establish: ballot policy,
//! census eligibility and which records outside configured seats are in scope.

use super::{
    problem::{Code, Problem, Report},
    sheet::{Row, Sheet, Workbook},
    sql_dump::{read_mysql_dump, SqlDump, SqlTable, SqlValue},
    xlsx::read_xlsx,
};
use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const PROFILE: &str = "legacy-seat-v1";
/// Where the importer keeps a name: event, election, contest and candidate rows
/// have no plain `name` column since names moved into `presentation`.
const NAME: &str = "presentation.i18n.en.name";
const TABLES: &[&str] = &[
    "allbgy",
    "allmun",
    "allprov",
    "candidate",
    "canvass_center",
    "contest",
    "district_type",
    "geographical",
    "party_list",
    "polling_centers",
    "pop",
    "ref_typeboard",
    "seat",
    "trans_route",
    "voters",
];

#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "kebab-case")]
pub enum CensusPolicy {
    #[default]
    RequireReviewedMapping,
    ConfigurationOnly,
}
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "kebab-case")]
pub enum BallotPolicy {
    #[default]
    RequireReview,
    SeatPluralityAtLarge,
}
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "kebab-case")]
pub enum ScopePolicy {
    #[default]
    RequireCompleteCoverage,
    ConfiguredSeatsOnly,
}
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "kebab-case")]
pub enum PostCoverage {
    #[default]
    RequireCompleteCoverage,
    WorkbookPostsOnly,
}
#[derive(
    Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq,
)]
#[serde(rename_all = "kebab-case")]
pub enum AffiliationPolicy {
    #[default]
    RequireKnownParty,
    PreserveUnresolvedCode,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct SourceDigests {
    pub sql_sha256: String,
    pub workbook_sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ElectionMapping {
    pub workbook_row: usize,
    pub seat_rows: Vec<usize>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AreaMapping {
    pub workbook_row: usize,
    pub precinct_rows: Vec<usize>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ElectionContext {
    pub workbook_row: usize,
    pub center_rows: Vec<usize>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrecinctSummary {
    pub source_row: usize,
    pub center_row: usize,
    pub workbook_area_rows: Vec<usize>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeatSummary {
    pub source_row: usize,
    pub candidate_rows: usize,
    pub eligible_amount: String,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct HistoricalOptions {
    #[serde(default)]
    pub census: CensusPolicy,
    #[serde(default)]
    pub ballot: BallotPolicy,
    #[serde(default)]
    pub scope: ScopePolicy,
    #[serde(default)]
    pub post_coverage: PostCoverage,
    #[serde(default)]
    pub precinct_coverage: PostCoverage,
    #[serde(default)]
    pub election_mappings: Vec<ElectionMapping>,
    #[serde(default)]
    pub area_mappings: Vec<AreaMapping>,
    pub source_digests: Option<SourceDigests>,
    #[serde(default)]
    pub affiliation: AffiliationPolicy,
    /// Offset for timezone-less historical Excel dates. No implicit local timezone.
    pub schedule_offset_minutes: Option<i32>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub profile: String,
    pub sql_sha256: String,
    pub workbook_sha256: String,
    pub source_rows: BTreeMap<String, usize>,
    pub generated_rows: BTreeMap<String, usize>,
    pub selected_candidates: usize,
    pub outside_scope_candidates: usize,
    pub excluded_voters: usize,
    pub excluded_polling_centers: usize,
    pub excluded_precinct_rows: usize,
    pub election_contexts: Vec<ElectionContext>,
    pub seats: Vec<SeatSummary>,
    pub precincts: Vec<PrecinctSummary>,
    pub area_workbook_rows: Vec<usize>,
    pub decisions_sha256: String,
    pub unresolved_affiliations: usize,
    /// Binary cells remain in the unmodified source; never coerced to text.
    pub opaque_binary_values: usize,
}
#[derive(Debug)]
pub struct Adapted {
    pub workbook: Workbook,
    pub summary: Summary,
}
#[derive(Debug)]
pub struct Refused {
    pub summary: Summary,
    pub report: Report,
}

fn problem(
    table: &str,
    row: usize,
    column: &str,
    code: Code,
    message: &str,
) -> Problem {
    Problem::error(
        code,
        format!("SQL table {table} row {} column {column}", row + 1),
        message,
    )
}
fn fail(summary: Summary, p: Problem) -> Refused {
    let mut report = Report::default();
    report.push(p);
    Refused { summary, report }
}

/// Adapt the declared profile into the existing authoring model. Callers use the
/// same `build`, `validate` and `archive::zip` as every other native import.
pub fn adapt(
    sql: &[u8],
    xlsx: &[u8],
    options: &HistoricalOptions,
) -> Result<Adapted, Refused> {
    let mut summary = Summary {
        profile: PROFILE.into(),
        sql_sha256: format!("{:x}", Sha256::digest(sql)),
        workbook_sha256: format!("{:x}", Sha256::digest(xlsx)),
        ..Summary::default()
    };
    let dump = read_mysql_dump(sql).map_err(|e| {
        fail(
            summary.clone(),
            Problem::error(
                Code::Unreadable,
                format!("SQL line {}", e.line),
                e.code,
            ),
        )
    })?;
    summary.source_rows = dump
        .tables
        .iter()
        .map(|(name, table)| (name.clone(), table.rows.len()))
        .collect();
    summary.opaque_binary_values = dump
        .tables
        .values()
        .map(|t| {
            t.rows
                .iter()
                .flat_map(|r| r.iter())
                .filter(|v| matches!(v, SqlValue::Bytes(b) if std::str::from_utf8(b).is_err()))
                .count()
        })
        .sum();
    if xlsx.len() > 32 * 1024 * 1024 {
        return Err(fail(
            summary,
            Problem::error(
                Code::Unreadable,
                "workbook",
                "workbook exceeds the 32 MiB source limit",
            ),
        ));
    }
    let mut zipped =
        zip::ZipArchive::new(std::io::Cursor::new(xlsx)).map_err(|_| {
            fail(
                summary.clone(),
                Problem::error(
                    Code::Unreadable,
                    "workbook",
                    "workbook is not a readable ZIP container",
                ),
            )
        })?;
    if zipped.len() > 1024 {
        return Err(fail(
            summary,
            Problem::error(
                Code::Unreadable,
                "workbook",
                "workbook exceeds the member limit",
            ),
        ));
    }
    let mut expanded = 0u64;
    for i in 0..zipped.len() {
        let member = zipped.by_index(i).map_err(|_| {
            fail(
                summary.clone(),
                Problem::error(
                    Code::Unreadable,
                    "workbook",
                    "workbook ZIP metadata is invalid",
                ),
            )
        })?;
        expanded = expanded
            .checked_add(member.size())
            .filter(|n| *n <= 64 * 1024 * 1024)
            .ok_or_else(|| {
                fail(
                    summary.clone(),
                    Problem::error(
                        Code::Unreadable,
                        "workbook",
                        "workbook exceeds the 64 MiB expanded limit",
                    ),
                )
            })?;
    }
    let workbook = read_xlsx(xlsx).map_err(|p| fail(summary.clone(), p))?;
    summary.decisions_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(options).map_err(|_| fail(
            summary.clone(),
            Problem::error(
                Code::InvalidValue,
                "historical decisions",
                "could not encode decisions"
            )
        ))?)
    );
    let result = map(&dump, &workbook, xlsx, options, &mut summary);
    result
        .map(|workbook| Adapted {
            workbook,
            summary: summary.clone(),
        })
        .map_err(|p| fail(summary, p))
}

struct Data<'a> {
    name: &'a str,
    table: &'a SqlTable,
}
impl<'a> Data<'a> {
    fn new(
        dump: &'a SqlDump,
        name: &'a str,
        columns: &[&str],
    ) -> Result<Self, Problem> {
        let table = dump.tables.get(name).ok_or_else(|| {
            problem(
                name,
                0,
                "schema",
                Code::MissingField,
                "required source table is absent",
            )
        })?;
        for column in columns {
            if !table.columns.iter().any(|c| c == column) {
                return Err(problem(
                    name,
                    0,
                    column,
                    Code::MissingField,
                    "required source column is absent",
                ));
            }
        }
        Ok(Self { name, table })
    }
    fn text(&self, row: usize, column: &str) -> Result<String, Problem> {
        let index = self
            .table
            .columns
            .iter()
            .position(|c| c == column)
            .ok_or_else(|| {
                problem(
                    self.name,
                    row,
                    column,
                    Code::MissingField,
                    "source column is absent",
                )
            })?;
        let value = self.table.rows.get(row).and_then(|r| r.get(index));
        let Some(SqlValue::Bytes(bytes)) = value else {
            return Err(problem(
                self.name,
                row,
                column,
                Code::MissingField,
                "source value is null",
            ));
        };
        let charset = self
            .table
            .column_charsets
            .get(column)
            .or(self.table.charset.as_ref())
            .map(String::as_str)
            .unwrap_or("ascii");
        let text = match charset.to_ascii_lowercase().as_str() {
            "utf8" | "utf8mb3" | "utf8mb4" | "ascii" => {
                std::str::from_utf8(bytes).map(str::to_owned).map_err(|_| {
                    problem(
                        self.name,
                        row,
                        column,
                        Code::InvalidValue,
                        "source text does not match its declared charset",
                    )
                })?
            }
            "latin1" => bytes.iter().map(|&b| latin1(b)).collect(),
            _ => {
                return Err(problem(
                    self.name,
                    row,
                    column,
                    Code::InvalidValue,
                    "source charset is unsupported by this profile",
                ))
            }
        };
        Ok(text)
    }
    fn optional(
        &self,
        row: usize,
        column: &str,
    ) -> Result<Option<String>, Problem> {
        let index = self
            .table
            .columns
            .iter()
            .position(|c| c == column)
            .ok_or_else(|| {
                problem(
                    self.name,
                    row,
                    column,
                    Code::MissingField,
                    "source column is absent",
                )
            })?;
        if self.table.rows[row][index] == SqlValue::Null {
            return Ok(None);
        }
        self.text(row, column).map(Some)
    }
    fn index(&self, column: &str) -> Result<BTreeMap<String, usize>, Problem> {
        let mut index = BTreeMap::new();
        for row in 0..self.table.rows.len() {
            if index.insert(self.text(row, column)?, row).is_some() {
                return Err(problem(
                    self.name,
                    row,
                    column,
                    Code::DuplicateId,
                    "source lookup key is duplicated",
                ));
            }
        }
        Ok(index)
    }
    fn lookup(
        &self,
        index: &BTreeMap<String, usize>,
        key: &str,
        row: usize,
        column: &str,
    ) -> Result<usize, Problem> {
        index.get(key).copied().ok_or_else(|| {
            problem(
                self.name,
                row,
                column,
                Code::DanglingReference,
                "source reference has no matching row",
            )
        })
    }
}
fn latin1(byte: u8) -> char {
    // MySQL's latin1 uses Windows-1252 in the 0x80–0x9f range.
    const C1: [char; 32] = [
        '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ',
        '\u{8d}', 'Ž', '\u{8f}', '\u{90}', '‘', '’', '“', '”', '•', '–', '—',
        '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
    ];
    if (0x80..=0x9f).contains(&byte) {
        C1[(byte - 0x80) as usize]
    } else {
        char::from(byte)
    }
}
fn rows_sheet(name: &str, rows: Vec<Row>) -> Sheet {
    Sheet {
        name: name.into(),
        key: super::sheet::normalise_sheet_name(name),
        headers: rows.first().map_or(Vec::new(), |r| {
            r.cells.iter().map(|(k, _)| k.clone()).collect()
        }),
        rows,
    }
}
fn row(name: &str, number: usize, cells: Vec<(&str, Value)>) -> Row {
    Row {
        sheet: name.into(),
        number: number + 2,
        cells: cells.into_iter().map(|(k, v)| (k.into(), v)).collect(),
    }
}
fn set(row: &mut Row, column: &str, value: Value) {
    row.cells.retain(|(c, _)| c != column);
    row.cells.push((column.into(), value));
}
fn text(row: &Row, column: &str) -> Result<String, Problem> {
    match row.get(column) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.clone()),
        Some(Value::Number(n)) => Ok(n.to_string()),
        _ => Err(Problem::error(
            Code::MissingField,
            row.origin(Some(column)).to_string(),
            "required workbook value is absent",
        )
        .at(&row.origin(Some(column)))),
    }
}
/// Moves a workbook row's plain `name` to [`NAME`]. A row may carry both only
/// when they agree.
fn localized_name(row: &mut Row) -> Result<(), Problem> {
    let plain = text(row, "name").ok();
    let origin = row.origin(Some("name"));
    row.cells.retain(|(column, _)| column != "name");
    let Some(plain) = plain else {
        return Ok(());
    };
    match text(row, NAME) {
        Ok(localized) if localized != plain => Err(Problem::error(
            Code::ConflictingColumns,
            origin.to_string(),
            "name and presentation.i18n.en.name differ; keep only one",
        )
        .at(&origin)),
        Ok(_) => Ok(()),
        Err(_) => {
            set(row, NAME, json!(plain));
            Ok(())
        }
    }
}
fn workbook_index(
    rows: &[Row],
    column: &str,
) -> Result<BTreeMap<String, usize>, Problem> {
    let mut index = BTreeMap::new();
    for (i, row) in rows.iter().enumerate() {
        if index.insert(text(row, column)?, i).is_some() {
            return Err(Problem::error(
                Code::DuplicateId,
                row.origin(Some(column)).to_string(),
                "workbook lookup value is duplicated",
            )
            .at(&row.origin(Some(column))));
        }
    }
    Ok(index)
}
fn historical_annotations(
    row: &mut Row,
    columns: &[(&str, &str)],
) -> Result<(), Problem> {
    for (source, target) in columns {
        if row.get(source).is_some() {
            let value = text(row, source)?;
            row.cells.retain(|(key, _)| key != source);
            set(row, target, json!(value));
        }
    }
    Ok(())
}
fn sheet_row<'a>(
    rows: &'a [Row],
    index: &BTreeMap<String, usize>,
    key: &str,
    sheet: &str,
    column: &str,
) -> Result<&'a Row, Problem> {
    index.get(key).and_then(|i| rows.get(*i)).ok_or_else(|| {
        Problem::error(
            Code::DanglingReference,
            format!("sheet {sheet} column {column}"),
            "source value has no supplemental workbook match",
        )
    })
}

fn map(
    dump: &SqlDump,
    workbook: &Workbook,
    xlsx: &[u8],
    options: &HistoricalOptions,
    summary: &mut Summary,
) -> Result<Workbook, Problem> {
    for name in TABLES {
        Data::new(dump, name, &[])?;
    }
    if dump.tables.keys().any(|n| !TABLES.contains(&n.as_str())) {
        return Err(Problem::error(
            Code::InvalidValue,
            "SQL schema",
            "unrecognized table in the declared source profile",
        ));
    }
    // Verify every declared primary/unique key without emitting its contents.
    for (name, table) in &dump.tables {
        for column in &table.primary_key {
            let index =
                table.columns.iter().position(|c| c == column).ok_or_else(
                    || {
                        problem(
                            name,
                            0,
                            column,
                            Code::MissingField,
                            "primary key column is absent",
                        )
                    },
                )?;
            for (row, values) in table.rows.iter().enumerate() {
                if values.get(index) == Some(&SqlValue::Null) {
                    return Err(problem(
                        name,
                        row,
                        column,
                        Code::MissingField,
                        "primary key is null",
                    ));
                }
            }
        }
        for key in &table.unique_keys {
            let columns = key
                .iter()
                .map(|k| {
                    table.columns.iter().position(|c| c == k).ok_or_else(|| {
                        problem(
                            name,
                            0,
                            k,
                            Code::MissingField,
                            "key names an absent column",
                        )
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            let mut seen = BTreeSet::new();
            for (i, row) in table.rows.iter().enumerate() {
                let values: Vec<Vec<u8>> = columns
                    .iter()
                    .filter_map(|&c| match &row[c] {
                        SqlValue::Null => None,
                        SqlValue::Bytes(b) => Some(b.clone()),
                    })
                    .collect();
                if values.len() == columns.len() && !seen.insert(values) {
                    return Err(problem(
                        name,
                        i,
                        "key",
                        Code::DuplicateId,
                        "declared source key is duplicated",
                    ));
                }
            }
        }
    }
    let voters = dump.tables.get("voters").map_or(0, |t| t.rows.len());
    if voters > 0 && options.census != CensusPolicy::ConfigurationOnly {
        return Err(Problem::error(Code::NotApproved,"SQL table voters","this profile does not define historical eligibility; explicitly choose configuration-only and use an independently reviewed census workflow"));
    }
    summary.excluded_voters = voters;
    if options.ballot != BallotPolicy::SeatPluralityAtLarge {
        return Err(Problem::error(Code::NotApproved,"historical ballot policy","review and explicitly select seat-plurality-at-large semantics before generating ballots"));
    }
    for sheet in workbook.sheets() {
        if ![
            "electionevent",
            "elections",
            "areas",
            "scheduledevents",
            "ccsservers",
        ]
        .contains(&sheet.key.as_str())
        {
            return Err(Problem::error(
                Code::InvalidValue,
                format!("sheet {}", sheet.name),
                "unrecognized supplemental sheet",
            ));
        }
    }
    if workbook.rows("electionevent").len() != 1 {
        return Err(Problem::error(
            Code::InvalidValue,
            "sheet ElectionEvent",
            "exactly one event row is required",
        ));
    }
    let mut event = workbook.rows("electionevent")[0].clone();
    localized_name(&mut event)?;
    let event_key = text(&event, "annotations.miru_election_event_id")?;
    set(
        &mut event,
        "external_id",
        json!(format!("legacy-seat-v1:{event_key}")),
    );
    historical_annotations(
        &mut event,
        &[
            (
                "annotations.miru_election_event_id",
                "annotations.miru:election-event-id",
            ),
            (
                "annotations.miru_election_event_name",
                "annotations.miru:election-event-name",
            ),
        ],
    )?;
    if let Some(logo) = event.get("logo_url").cloned() {
        event.cells.retain(|(c, _)| c != "logo_url");
        set(&mut event, "presentation.logo_url", logo);
    }
    let populations = Data::new(
        dump,
        "pop",
        &[
            "CLUSTERPOLLCENTER",
            "POLLCENTER_CODE",
            "PROV_CODE",
            "MUN_CODE",
            "BRGY_CODE",
        ],
    )?;
    let centers = Data::new(dump, "polling_centers", &["ID", "POLLING_PLACE"])?;
    let barangays = Data::new(dump, "allbgy", &["ID_BARANGAY", "AREANAME"])?;
    let municipalities = Data::new(dump, "allmun", &["ID_CITY", "AREANAME"])?;
    let routes =
        Data::new(dump, "trans_route", &["TRANS_SOURCE_ID", "TRANS_DEST_ID"])?;
    let seats = Data::new(
        dump,
        "seat",
        &["DISTRICTCODE", "CONTEST", "ELIGIBLEAMOUNT", "GEOCODE"],
    )?;
    let contests =
        Data::new(dump, "contest", &["POSTCODE", "POSTNAME", "SORT_ORDER"])?;
    let candidates = Data::new(
        dump,
        "candidate",
        &[
            "CAND_CODE",
            "NAMEONBALLOT",
            "POST_CODE",
            "DISTRICTCODE",
            "NOMINATEDBY",
        ],
    )?;
    let parties = Data::new(
        dump,
        "party_list",
        &["CODE_PARTY", "SHORT_NAME", "NAME_PARTY"],
    )?;
    let geography = Data::new(dump, "geographical", &["GEOCODE"])?;
    let ci = centers.index("ID")?;
    let bi = barangays.index("ID_BARANGAY")?;
    let mi = municipalities.index("ID_CITY")?;
    let ri = routes.index("TRANS_SOURCE_ID")?;
    let ti = contests.index("POSTNAME")?;
    let pi = parties.index("CODE_PARTY")?;
    let gi = geography.index("GEOCODE")?;
    seats.index("DISTRICTCODE")?;
    candidates.index("CAND_CODE")?;
    let erows = workbook.rows("elections");
    workbook_index(erows, "alias")?;
    let mut ei: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, r) in erows.iter().enumerate() {
        ei.entry(text(r, "election_post")?).or_default().push(i);
    }
    summary.election_contexts = erows
        .iter()
        .map(|r| {
            let post = text(r, "election_post")?;
            let centers = (0..centers.table.rows.len())
                .filter_map(|i| match centers.text(i, "POLLING_PLACE") {
                    Ok(p) if p == post => Some(i + 1),
                    _ => None,
                })
                .collect();
            Ok(ElectionContext {
                workbook_row: r.number,
                center_rows: centers,
            })
        })
        .collect::<Result<_, Problem>>()?;
    if summary
        .election_contexts
        .iter()
        .any(|context| context.center_rows.len() > 1)
    {
        return Err(Problem::error(Code::NotApproved,"sheet Elections election_post","one workbook context matches multiple SQL centers; provide unambiguous post identities before associating schedules"));
    }
    summary.seats = (0..seats.table.rows.len())
        .map(|i| {
            let district = seats.text(i, "DISTRICTCODE")?;
            let count = (0..candidates.table.rows.len())
                .map(|c| candidates.text(c, "DISTRICTCODE"))
                .collect::<Result<Vec<_>, _>>()?
                .iter()
                .filter(|d| **d == district)
                .count();
            Ok(SeatSummary {
                source_row: i + 1,
                candidate_rows: count,
                eligible_amount: seats.text(i, "ELIGIBLEAMOUNT")?,
            })
        })
        .collect::<Result<_, Problem>>()?;
    summary.excluded_polling_centers = (0..centers.table.rows.len())
        .map(|i| centers.text(i, "POLLING_PLACE"))
        .collect::<Result<Vec<_>, _>>()?
        .iter()
        .filter(|p| !ei.contains_key(*p))
        .count();
    let arows = workbook.rows("areas");
    let ai = workbook_index(arows, "name")?;
    summary.area_workbook_rows = arows.iter().map(|r| r.number).collect();
    summary.precincts = (0..populations.table.rows.len())
        .map(|pop| {
            let center = centers.lookup(
                &ci,
                &populations.text(pop, "POLLCENTER_CODE")?,
                pop,
                "POLLCENTER_CODE",
            )?;
            let bgy = barangays.lookup(
                &bi,
                &populations.text(pop, "CLUSTERPOLLCENTER")?,
                pop,
                "CLUSTERPOLLCENTER",
            )?;
            let city = format!(
                "{}{}",
                populations.text(pop, "PROV_CODE")?,
                populations.text(pop, "MUN_CODE")?
            );
            let municipality =
                municipalities.lookup(&mi, &city, pop, "ID_CITY")?;
            let barangay = barangays.text(bgy, "AREANAME")?;
            let name = if barangay == centers.text(center, "POLLING_PLACE")? {
                municipalities.text(municipality, "AREANAME")?
            } else {
                barangay
            };
            let workbook_area_rows = arows
                .iter()
                .filter(|r| r.text("name") == Some(&name))
                .map(|r| r.number)
                .collect();
            Ok(PrecinctSummary {
                source_row: pop + 1,
                center_row: center + 1,
                workbook_area_rows,
            })
        })
        .collect::<Result<_, Problem>>()?;
    let mut associations: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    if !options.election_mappings.is_empty()
        || !options.area_mappings.is_empty()
        || options.post_coverage == PostCoverage::WorkbookPostsOnly
        || options.precinct_coverage == PostCoverage::WorkbookPostsOnly
        || options.affiliation == AffiliationPolicy::PreserveUnresolvedCode
    {
        let digests=options.source_digests.as_ref().ok_or_else(||Problem::error(Code::NotApproved,"historical decisions sourceDigests","explicit associations or limited post coverage must be bound to both source-file hashes"))?;
        if digests.sql_sha256 != summary.sql_sha256
            || digests.workbook_sha256 != summary.workbook_sha256
        {
            return Err(Problem::error(
                Code::IntegrityMismatch,
                "historical decisions sourceDigests",
                "decisions describe different source bytes",
            ));
        }
    }
    for mapping in &options.election_mappings {
        if !erows.iter().any(|r| r.number == mapping.workbook_row)
            || mapping.seat_rows.is_empty()
        {
            return Err(Problem::error(
                Code::InvalidValue,
                "historical decisions electionMappings",
                "each mapping must name an existing workbook row and at least one SQL seat row",
            ));
        }
        let mut seen = BTreeSet::new();
        let mut indices = Vec::new();
        for &n in &mapping.seat_rows {
            if n == 0 || n > seats.table.rows.len() || !seen.insert(n) {
                return Err(Problem::error(
                    Code::InvalidValue,
                    "historical decisions electionMappings",
                    "seat row is out of range or repeated",
                ));
            }
            indices.push(n - 1);
        }
        if associations.insert(mapping.workbook_row, indices).is_some() {
            return Err(Problem::error(
                Code::DuplicateId,
                "historical decisions electionMappings",
                "workbook row is mapped twice",
            ));
        }
    }
    if associations.is_empty() && ei.values().any(|group| group.len() > 1) {
        return Err(Problem::error(Code::NotApproved,"sheet Elections election_post + alias","distinct election contexts share a post; specify the SQL seat rows for each workbook row, bound to source hashes"));
    }
    if !associations.is_empty() && associations.len() != erows.len() {
        return Err(Problem::error(
            Code::BallotCoverage,
            "historical decisions electionMappings",
            "every workbook election row must have an explicit association",
        ));
    }
    if summary.excluded_polling_centers > 0
        && options.post_coverage != PostCoverage::WorkbookPostsOnly
    {
        return Err(Problem::error(Code::BallotCoverage,"sheet Elections election_post","SQL polling centers have no workbook context; provide coverage or explicitly choose workbook-posts-only with source hashes"));
    }
    let mut used_workbook_rows = BTreeSet::new();

    let mut area_associations = BTreeMap::new();
    for mapping in &options.area_mappings {
        let workbook_index = arows
            .iter()
            .position(|r| r.number == mapping.workbook_row)
            .ok_or_else(|| {
                Problem::error(
                    Code::InvalidValue,
                    "historical decisions areaMappings",
                    "area workbook row is absent",
                )
            })?;
        if mapping.precinct_rows.is_empty() {
            return Err(Problem::error(
                Code::InvalidValue,
                "historical decisions areaMappings",
                "area mapping has no precinct rows",
            ));
        }
        for &n in &mapping.precinct_rows {
            if n == 0
                || n > populations.table.rows.len()
                || area_associations.insert(n - 1, workbook_index).is_some()
            {
                return Err(Problem::error(
                    Code::InvalidValue,
                    "historical decisions areaMappings",
                    "precinct row is out of range or mapped twice",
                ));
            }
        }
    }
    let servers = workbook.rows("ccsservers");
    let si = workbook_index(servers, "tag")?;
    let mut output_elections = BTreeMap::new();
    let mut output_areas = BTreeMap::new();
    let mut output_contests = BTreeMap::new();
    let mut output_candidates = BTreeMap::new();
    let mut output_links = BTreeMap::new();
    let mut used_candidates = BTreeSet::new();
    let mut unresolved_affiliations = BTreeSet::new();
    for pop in 0..populations.table.rows.len() {
        let center = centers.lookup(
            &ci,
            &populations.text(pop, "POLLCENTER_CODE")?,
            pop,
            "POLLCENTER_CODE",
        )?;
        let bgy = barangays.lookup(
            &bi,
            &populations.text(pop, "CLUSTERPOLLCENTER")?,
            pop,
            "CLUSTERPOLLCENTER",
        )?;
        let city_code = format!(
            "{}{}",
            populations.text(pop, "PROV_CODE")?,
            populations.text(pop, "MUN_CODE")?
        );
        let municipality =
            municipalities.lookup(&mi, &city_code, pop, "ID_CITY")?;
        let route_code =
            format!("{}0{}", city_code, populations.text(pop, "BRGY_CODE")?);
        let route = routes.lookup(&ri, &route_code, pop, "TRANS_SOURCE_ID")?;
        let post = centers.text(center, "POLLING_PLACE")?;
        let barangay = barangays.text(bgy, "AREANAME")?;
        let area_name = if barangay == post {
            municipalities.text(municipality, "AREANAME")?
        } else {
            barangay
        };
        let Some(contexts) = ei.get(&post) else {
            summary.excluded_precinct_rows += 1;
            continue;
        };
        if !area_associations.contains_key(&pop)
            && !ai.contains_key(&area_name)
            && options.precinct_coverage == PostCoverage::WorkbookPostsOnly
        {
            summary.excluded_precinct_rows += 1;
            continue;
        }
        let mut area = if let Some(&index) = area_associations.get(&pop) {
            arows[index].clone()
        } else {
            sheet_row(arows, &ai, &area_name, "Areas", "name")?.clone()
        };
        // Names are workbook lookup keys only. Stable ids come from source codes.
        let area_id =
            format!("barangay:{}", barangays.text(bgy, "ID_BARANGAY")?);
        set(&mut area, "external_id", json!(area_id));
        set(
            &mut area,
            "annotations.miru:area-station-id",
            json!(route_code),
        );
        set(
            &mut area,
            "annotations.legacy_destination_id",
            json!(routes.text(route, "TRANS_DEST_ID")?),
        );
        if let Some(tags) = area.get("annotations.miru_ccs_server_tags") {
            let tags = match tags {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => {
                    return Err(Problem::error(
                        Code::InvalidValue,
                        area.origin(Some("annotations.miru_ccs_server_tags"))
                            .to_string(),
                        "CCS tags must be scalar references",
                    ))
                }
            };
            let mut resolved = Vec::new();
            for tag in tags.split(',').map(str::trim) {
                let server = sheet_row(servers, &si, tag, "CcsServers", "tag")?;
                let send_logs = match server.get("send_logs") {
                    Some(Value::Bool(v)) => *v,
                    Some(Value::String(v))
                        if v.eq_ignore_ascii_case("true") =>
                    {
                        true
                    }
                    Some(Value::String(v))
                        if v.eq_ignore_ascii_case("false") =>
                    {
                        false
                    }
                    _ => {
                        return Err(Problem::error(
                            Code::InvalidValue,
                            server.origin(Some("send_logs")).to_string(),
                            "send_logs must be true or false",
                        ))
                    }
                };
                resolved.push(json!({"name":text(server,"name")?,"tag":tag.parse::<i64>().map_err(|_|Problem::error(Code::InvalidValue,"sheet CcsServers column tag","CCS tag must be an integer"))?.to_string(),"address":text(server,"address")?,"public_key_pem":text(server,"public_key")?,"send_logs":send_logs}));
            }
            set(
                &mut area,
                "annotations.miru:area-ccs-servers",
                json!(super::canonical::compact(&json!(resolved)).map_err(
                    |_| {
                        Problem::error(
                            Code::InvalidValue,
                            "sheet CcsServers",
                            "could not serialize CCS metadata",
                        )
                    }
                )?),
            );
        }
        if let Some(trustees) = area
            .get("annotations.miru_trustee_servers")
            .and_then(Value::as_str)
        {
            let trustees: Vec<_> = trustees.split(',').map(str::trim).collect();
            let encoded = serde_json::to_string(&trustees).map_err(|_| {
                Problem::error(
                    Code::InvalidValue,
                    "sheet Areas",
                    "could not serialize trustee metadata",
                )
            })?;
            set(
                &mut area,
                "annotations.miru:area-trustee-users",
                json!(encoded),
            );
        }
        for (_, value) in area
            .cells
            .iter_mut()
            .filter(|(key, _)| key.starts_with("annotations."))
        {
            if !value.is_string() {
                *value = json!(value.to_string());
            }
        }
        if let Some(previous) =
            output_areas.insert(area_id.clone(), area.clone())
        {
            if previous.cells != area.cells {
                return Err(Problem::error(
                    Code::ConflictingColumns,
                    "sheet Areas",
                    "precinct rows provide conflicting area metadata",
                ));
            }
        }
        for &context_index in contexts {
            let original_election = &erows[context_index];
            used_workbook_rows.insert(original_election.number);
            let election_id = format!(
                "center:{}:context:{}",
                centers.text(center, "ID")?,
                text(original_election, "alias")?
            );
            let mut election = original_election.clone();
            localized_name(&mut election)?;
            election.cells.retain(|(c, _)| c != "election_post");
            set(&mut election, "external_id", json!(election_id));
            historical_annotations(
                &mut election,
                &[
                    (
                        "annotations.miru_election_id",
                        "annotations.miru:election-id",
                    ),
                    (
                        "annotations.miru_election_name",
                        "annotations.miru:election-name",
                    ),
                ],
            )?;
            output_elections.insert(election_id.clone(), election);
            let selected_seats = associations
                .get(&original_election.number)
                .cloned()
                .unwrap_or_else(|| (0..seats.table.rows.len()).collect());
            for seat in selected_seats {
                geography.lookup(
                    &gi,
                    &seats.text(seat, "GEOCODE")?,
                    seat,
                    "GEOCODE",
                )?;
                let contest = contests.lookup(
                    &ti,
                    &seats.text(seat, "CONTEST")?,
                    seat,
                    "CONTEST",
                )?;
                let district = seats.text(seat, "DISTRICTCODE")?;
                let postcode = contests.text(contest, "POSTCODE")?;
                let contest_id = format!("{election_id}:seat:{district}");
                let eligible = seats
                    .text(seat, "ELIGIBLEAMOUNT")?
                    .parse::<u64>()
                    .map_err(|_| {
                        problem(
                            "seat",
                            seat,
                            "ELIGIBLEAMOUNT",
                            Code::InvalidValue,
                            "seat count must be a positive integer",
                        )
                    })?;
                if eligible == 0 {
                    return Err(problem(
                        "seat",
                        seat,
                        "ELIGIBLEAMOUNT",
                        Code::InvalidValue,
                        "seat count must be positive",
                    ));
                }
                let order = contests
                    .text(contest, "SORT_ORDER")?
                    .parse::<u64>()
                    .map_err(|_| {
                        problem(
                            "contest",
                            contest,
                            "SORT_ORDER",
                            Code::InvalidValue,
                            "ballot order must be an integer",
                        )
                    })?;
                output_contests.insert(
                    contest_id.clone(),
                    row(
                        "Contests",
                        seat,
                        vec![
                            ("external_id", json!(contest_id)),
                            ("election.external_id", json!(election_id)),
                            (NAME, json!(seats.text(seat, "CONTEST")?)),
                            ("min_votes", json!(0)),
                            ("max_votes", json!(eligible)),
                            ("winning_candidates_num", json!(eligible)),
                            ("voting_type", json!("non-preferential")),
                            ("counting_algorithm", json!("plurality-at-large")),
                            ("presentation.sort_order", json!(order)),
                            ("annotations.miru:contest-id", json!(district)),
                            (
                                "annotations.miru:contest-name",
                                json!(seats.text(seat, "CONTEST")?),
                            ),
                        ],
                    ),
                );
                output_links.insert(
                    (area_id.clone(), contest_id.clone()),
                    row(
                        "AreaContests",
                        seat,
                        vec![
                            ("area.external_id", json!(area_id)),
                            ("contest.external_id", json!(contest_id)),
                        ],
                    ),
                );
                let mut candidate_order = 0;
                for candidate in 0..candidates.table.rows.len() {
                    if candidates.text(candidate, "POST_CODE")? != postcode
                        || candidates.text(candidate, "DISTRICTCODE")?
                            != district
                    {
                        continue;
                    }
                    let code = candidates.text(candidate, "CAND_CODE")?;
                    used_candidates.insert(code.clone());
                    candidate_order += 1;
                    let mut candidate_row = row(
                        "Candidates",
                        candidate,
                        vec![
                            (
                                "external_id",
                                json!(format!("{contest_id}:candidate:{code}")),
                            ),
                            ("contest.external_id", json!(contest_id)),
                            (
                                NAME,
                                json!(candidates
                                    .text(candidate, "NAMEONBALLOT")?),
                            ),
                            ("presentation.sort_order", json!(candidate_order)),
                            ("annotations.miru:candidate-id", json!(code)),
                            (
                                "annotations.miru:candidate-name",
                                json!(candidates
                                    .text(candidate, "NAMEONBALLOT")?),
                            ),
                            (
                                "annotations.miru:candidate-setting",
                                json!("candidate"),
                            ),
                        ],
                    );
                    if let Some(party) = candidates
                        .optional(candidate, "NOMINATEDBY")?
                        .filter(|s| !s.is_empty())
                    {
                        if let Some(&party_row) = pi.get(&party) {
                            set(
                                &mut candidate_row,
                                "annotations.miru:candidate-affiliation-id",
                                json!(party),
                            );
                            set(
                                &mut candidate_row,
                                "annotations.miru:candidate-affiliation-registered-name",
                                json!(parties.text(party_row, "NAME_PARTY")?),
                            );
                            set(
                                &mut candidate_row,
                                "annotations.miru:candidate-affiliation-party",
                                json!(parties.text(party_row, "SHORT_NAME")?),
                            );
                        } else {
                            unresolved_affiliations.insert(code.clone());
                            summary.unresolved_affiliations =
                                unresolved_affiliations.len();
                            if options.affiliation
                                != AffiliationPolicy::PreserveUnresolvedCode
                            {
                                return Err(problem("candidate",candidate,"NOMINATEDBY",Code::DanglingReference,"affiliation has no party row; explicitly preserve the unresolved code or provide an authoritative reference"));
                            }
                            set(
                                &mut candidate_row,
                                "annotations.miru:candidate-affiliation-id",
                                json!(party),
                            );
                        }
                    }
                    output_candidates.insert(
                        format!("{contest_id}:candidate:{code}"),
                        candidate_row,
                    );
                }
            }
        }
    }
    summary.selected_candidates = used_candidates.len();
    summary.outside_scope_candidates =
        candidates.table.rows.len() - used_candidates.len();
    if summary.outside_scope_candidates > 0
        && options.scope != ScopePolicy::ConfiguredSeatsOnly
    {
        return Err(Problem::error(
            Code::NotApproved,
            "SQL table candidate",
            "candidate rows lie outside configured seats; review source scope explicitly",
        ));
    }
    if used_workbook_rows.len() != erows.len()
        || output_areas.len() != arows.len()
    {
        return Err(Problem::error(
            Code::BallotCoverage,
            "supplemental workbook",
            "election/area workbook rows and source-derived coverage differ",
        ));
    }
    let dates = historical_dates(xlsx, options.schedule_offset_minutes)?;
    let mut schedules = Vec::new();
    for (i, original) in workbook.rows("scheduledevents").iter().enumerate() {
        let alias = text(original, "election_alias")?;
        let election = output_elections
            .values()
            .find(|r| r.text("alias") == Some(&alias))
            .ok_or_else(|| {
                Problem::error(
                    Code::DanglingReference,
                    original.origin(Some("election_alias")).to_string(),
                    "schedule alias has no configured election",
                )
            })?;
        let date = dates.get(&original.number).ok_or_else(|| {
            Problem::error(
                Code::MissingField,
                original.origin(Some("date")).to_string(),
                "schedule date is absent",
            )
        })?;
        schedules.push(row(
            "ScheduledEvents",
            i,
            vec![
                (
                    "election.external_id",
                    json!(text(election, "external_id")?),
                ),
                ("event_type", json!(text(original, "type")?)),
                ("scheduled_datetime", json!(date)),
            ],
        ));
    }
    let provenance = json!({
        "profile": PROFILE,
        "sqlSha256": summary.sql_sha256,
        "workbookSha256": summary.workbook_sha256,
        "decisionsSha256": summary.decisions_sha256,
        "sourceRows": summary.source_rows,
        "selectedCandidates": summary.selected_candidates,
        "excludedVoters": summary.excluded_voters,
        "excludedPollingCenters": summary.excluded_polling_centers,
        "excludedPrecinctRows": summary.excluded_precinct_rows,
        "unresolvedAffiliations": summary.unresolved_affiliations,
        "opaqueBinaryValuesRetainedInSource": summary.opaque_binary_values,
        "decisions": options
    });
    let provenance = super::canonical::compact(&provenance).map_err(|_| {
        Problem::error(
            Code::InvalidValue,
            "historical provenance",
            "could not encode conversion provenance",
        )
    })?;
    set(
        &mut event,
        "annotations.import:provenance",
        json!(provenance),
    );
    let sheets = vec![
        rows_sheet("ElectionEvent", vec![event]),
        rows_sheet("Elections", output_elections.into_values().collect()),
        rows_sheet("Areas", output_areas.into_values().collect()),
        rows_sheet("Contests", output_contests.into_values().collect()),
        rows_sheet("Candidates", output_candidates.into_values().collect()),
        rows_sheet("AreaContests", output_links.into_values().collect()),
        rows_sheet("ScheduledEvents", schedules),
    ];
    summary.generated_rows = sheets
        .iter()
        .map(|s| (s.name.clone(), s.rows.len()))
        .collect();
    Workbook::new(sheets)
}

fn historical_dates(
    bytes: &[u8],
    offset: Option<i32>,
) -> Result<BTreeMap<usize, String>, Problem> {
    use calamine::{Data, Reader, Xlsx};
    use std::io::Cursor;
    let mut book: Xlsx<_> = Xlsx::new(Cursor::new(bytes)).map_err(|_| {
        Problem::error(Code::Unreadable, "workbook", "cannot read date cells")
    })?;
    let name = book
        .sheet_names()
        .iter()
        .find(|n| super::sheet::normalise_sheet_name(n) == "scheduledevents")
        .cloned()
        .ok_or_else(|| {
            Problem::error(
                Code::MissingField,
                "sheet ScheduledEvents",
                "schedule sheet is required",
            )
        })?;
    let range = book.worksheet_range(&name).map_err(|_| {
        Problem::error(
            Code::Unreadable,
            "sheet ScheduledEvents",
            "cannot read schedule dates",
        )
    })?;
    let column = range
        .rows()
        .next()
        .and_then(|r| {
            r.iter()
                .position(|v| matches!(v,Data::String(s) if s.trim()=="date"))
        })
        .ok_or_else(|| {
            Problem::error(
                Code::MissingField,
                "sheet ScheduledEvents column date",
                "date column is required",
            )
        })?;
    let mut dates = BTreeMap::new();
    for (i, row) in range.rows().enumerate().skip(1) {
        let Some(value) = row.get(column) else {
            continue;
        };
        if matches!(value, Data::Empty) {
            continue;
        }
        let at = format!("sheet ScheduledEvents row {} column date", i + 1);
        let naive = match value {
            Data::DateTime(excel) if !excel.is_duration() => {
                excel.as_datetime()
            }
            Data::String(s) | Data::DateTimeIso(s) => {
                if let Ok(date) = DateTime::parse_from_rfc3339(s) {
                    dates.insert(i + 1, date.to_rfc3339());
                    continue;
                }
                ["%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"]
                    .iter()
                    .find_map(|f| NaiveDateTime::parse_from_str(s, f).ok())
            }
            _ => None,
        }
        .ok_or_else(|| {
            Problem::error(
                Code::InvalidValue,
                &at,
                "date must be an instant or an Excel date",
            )
        })?;
        let minutes =
            offset.filter(|n| (-840..=840).contains(n)).ok_or_else(|| {
                Problem::error(
                Code::NotApproved,
                &at,
                "timezone-less dates require an explicit UTC offset in minutes",
            )
            })?;
        let zone = FixedOffset::east_opt(minutes * 60).ok_or_else(|| {
            Problem::error(Code::InvalidValue, &at, "invalid UTC offset")
        })?;
        let instant =
            zone.from_local_datetime(&naive).single().ok_or_else(|| {
                Problem::error(
                    Code::InvalidValue,
                    &at,
                    "date cannot be resolved",
                )
            })?;
        dates.insert(i + 1, instant.to_rfc3339());
    }
    Ok(dates)
}

#[cfg(test)]
#[path = "historical_tests.rs"]
mod tests;

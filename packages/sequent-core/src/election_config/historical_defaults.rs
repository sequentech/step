// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A defaults workbook: what the dump does not hold, for any dump.
//!
//! The supplemental workbook adds the event's name and identity, the voting
//! period, the CCS servers and each area's servers. A full workbook also names
//! one election context per post and one area per cluster, so it fits a single
//! dump. A workbook without an `Elections` sheet is a defaults workbook instead:
//! the contexts and areas come from the source itself and the rest from its
//! sheets. The result is the sheets a full workbook would have, so the
//! conversion that follows is the one every workbook goes through.
//!
//! What is derived, and from where:
//!
//! - **Elections**: one per polling center that a `pop` row places voters in,
//!   named and aliased by its `POLLING_PLACE`. A center no `pop` row uses has no
//!   voters and gets no election; it is counted as excluded. An
//!   `ElectionDefaults` row gives elections its other columns, `name` included:
//!   one without a `polling_place` every election, one with a pattern the
//!   elections whose `POLLING_PLACE` it matches (`*-T`, `* PE`, or a whole
//!   name), the most specific pattern on top. `{place}` in a value is what the
//!   `*` stood for, written as a voter reads it (`ABU DHABI` is `Abu Dhabi`), or
//!   the whole polling place as its embassy label reads (`Bangkok PE`). The alias stays the polling place, since
//!   schedules name elections by it. A pattern no election matches is not an
//!   error: one defaults workbook serves every dump.
//! - **Areas**: one per precinct cluster (`pop.CLUSTERPOLLCENTER`), named by the
//!   historical rule (the barangay's name, or the municipality's when the barangay
//!   is named after its own post) and described by the cluster's own polling
//!   place, which is the embassy a preserved voter is shown under. An `Areas` row
//!   without a `name` gives every area its other columns; a row with a `name`
//!   gives them to that area only, after the defaults.
//! - **Schedules**: a `ScheduledEvents` row without an `election_alias` is every
//!   election's; one with an alias is that election's, as in a full workbook.
//! - **Event, CCS servers**: the `ElectionEvent` and `CcsServers` sheets as they
//!   are. Nothing is invented: an event without a name or identity is refused,
//!   and an election without a schedule is the same warning a full workbook
//!   gives.

use super::{problem, row, rows_sheet, Data};
use crate::election_config::{
    problem::{Code, Problem},
    sheet::{Row, Workbook},
    sql_dump::SqlDump,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// Whether `workbook` is a defaults workbook rather than a full one.
pub(super) fn is_defaults(workbook: &Workbook) -> bool {
    !workbook.has("elections")
}

/// The derived workbook, and the area row each `pop` row belongs to. Areas are
/// joined by row rather than by name, because two clusters may share a name.
pub(super) struct Derived {
    pub workbook: Workbook,
    pub areas_by_precinct: BTreeMap<usize, usize>,
    /// For each derived ScheduledEvents row, in order, the defaults workbook row
    /// it came from, whose date it takes.
    pub schedule_rows: Vec<usize>,
}

pub(super) fn derive(
    dump: &SqlDump,
    defaults: &Workbook,
) -> Result<Derived, Problem> {
    let populations = Data::new(
        dump,
        "pop",
        &[
            "CLUSTERPOLLCENTER",
            "POLLCENTER_CODE",
            "PROV_CODE",
            "MUN_CODE",
        ],
    )?;
    let centers = Data::new(dump, "polling_centers", &["ID", "POLLING_PLACE"])?;
    let barangays = Data::new(dump, "allbgy", &["ID_BARANGAY", "AREANAME"])?;
    let municipalities = Data::new(dump, "allmun", &["ID_CITY", "AREANAME"])?;
    let ci = centers.index("ID")?;
    let bi = barangays.index("ID_BARANGAY")?;
    let mi = municipalities.index("ID_CITY")?;
    let (area_defaults, area_overrides) = area_rows(defaults)?;
    let mut overridden = BTreeSet::new();

    let mut posts: Vec<String> = Vec::new();
    let mut areas: Vec<Row> = Vec::new();
    // Each area's post as derived; an override may change its description.
    let mut area_posts: Vec<String> = Vec::new();
    let mut area_by_cluster: BTreeMap<String, usize> = BTreeMap::new();
    let mut areas_by_precinct = BTreeMap::new();
    for pop in 0..populations.table.rows.len() {
        let center = centers.lookup(
            &ci,
            &populations.text(pop, "POLLCENTER_CODE")?,
            pop,
            "POLLCENTER_CODE",
        )?;
        let post = centers.text(center, "POLLING_PLACE")?;
        if !posts.contains(&post) {
            posts.push(post.clone());
        }
        let cluster = populations.text(pop, "CLUSTERPOLLCENTER")?;
        if let Some(&area) = area_by_cluster.get(&cluster) {
            if !ci.contains_key(&cluster) && area_posts[area] != post {
                return Err(problem(
                    "pop",
                    pop,
                    "POLLCENTER_CODE",
                    Code::ConflictingColumns,
                    "cluster has no own polling center and its precinct rows reference different posts",
                ));
            }
            areas_by_precinct.insert(pop, area);
            continue;
        }
        let barangay =
            barangays.lookup(&bi, &cluster, pop, "CLUSTERPOLLCENTER")?;
        // The cluster's own center, when it has one, rather than whichever center
        // (a test one, say) happens to come first.
        let own_post = match ci.get(&cluster) {
            Some(&own) => centers.text(own, "POLLING_PLACE")?,
            None => post,
        };
        let barangay_name = barangays.text(barangay, "AREANAME")?;
        let name = if barangay_name == own_post {
            let city = format!(
                "{}{}",
                populations.text(pop, "PROV_CODE")?,
                populations.text(pop, "MUN_CODE")?
            );
            let municipality =
                municipalities.lookup(&mi, &city, pop, "ID_CITY")?;
            municipalities.text(municipality, "AREANAME")?
        } else {
            barangay_name
        };
        let index = areas.len();
        let mut area = row(
            "Areas",
            index,
            vec![("name", json!(name)), ("description", json!(own_post))],
        );
        // Two clusters may share a name, and a row naming it cannot tell
        // them apart.
        let named = area_overrides.get(&name);
        if let Some(given) = named {
            if !overridden.insert(name.clone()) {
                return Err(Problem::error(
                    Code::ConflictingColumns,
                    given.origin(Some("name")).to_string(),
                    "more than one area derived from the SQL has this name",
                )
                .at(&given.origin(Some("name"))));
            }
        }
        for given in area_defaults.iter().chain(named) {
            for (column, value) in given.without(&["name"]) {
                super::set(&mut area, &column, value);
            }
        }
        areas.push(area);
        area_posts.push(own_post);
        area_by_cluster.insert(cluster, index);
        areas_by_precinct.insert(pop, index);
    }
    if let Some(unused) = area_overrides
        .iter()
        .find(|(name, _)| !overridden.contains(*name))
        .map(|(_, row)| row)
    {
        return Err(Problem::error(
            Code::DanglingReference,
            unused.origin(Some("name")).to_string(),
            "no area derived from the SQL has this name",
        )
        .at(&unused.origin(Some("name"))));
    }

    let (election_defaults, election_patterns) = election_rows(defaults)?;
    let mut elections = Vec::new();
    for (i, post) in posts.iter().enumerate() {
        let mut election = row(
            "Elections",
            i,
            vec![
                ("election_post", json!(post)),
                ("alias", json!(post)),
                ("name", json!(post)),
            ],
        );
        let matched = most_specific(&election_patterns, post)?;
        let every = election_defaults
            .as_ref()
            .map(|given| (given, post.as_str()));
        for (given, place) in every.into_iter().chain(matched) {
            // A whole post reads as its embassy label does; a part of one is a
            // place name.
            let place = if place == post {
                super::format_embassy(place)
            } else {
                super::format_place(place)
            };
            for (column, value) in given.without(&[POLLING_PLACE]) {
                super::set(&mut election, &column, with_place(value, &place));
            }
        }
        elections.push(election);
    }
    let mut schedules = Vec::new();
    let mut schedule_rows = Vec::new();
    for given in defaults.rows("scheduledevents") {
        let aliases = match given.text("election_alias") {
            Some(alias) if !alias.trim().is_empty() => vec![alias.to_string()],
            _ => posts.clone(),
        };
        for alias in aliases {
            let mut schedule = Row {
                sheet: "ScheduledEvents".into(),
                number: schedules.len() + 2,
                cells: given.cells.clone(),
            };
            super::set(&mut schedule, "election_alias", Value::String(alias));
            schedules.push(schedule);
            schedule_rows.push(given.number);
        }
    }
    let mut sheets = vec![
        rows_sheet("Elections", elections),
        rows_sheet("Areas", areas),
        rows_sheet("ScheduledEvents", schedules),
    ];
    sheets.extend(
        defaults
            .sheets()
            .iter()
            .filter(|sheet| {
                !matches!(
                    sheet.key.as_str(),
                    "areas" | "scheduledevents" | SHEET_ELECTION_DEFAULTS
                )
            })
            .cloned(),
    );
    Ok(Derived {
        workbook: Workbook::new(sheets)?,
        areas_by_precinct,
        schedule_rows,
    })
}

/// The sheet a defaults workbook names its elections in. Not `Elections`, whose
/// presence is what makes a workbook a full one.
const SHEET_ELECTION_DEFAULTS: &str = "electiondefaults";
const POLLING_PLACE: &str = "polling_place";

/// A polling-place pattern: a whole name, or one `*` standing for any text.
struct Pattern {
    prefix: String,
    suffix: Option<String>,
    row: Row,
}

impl Pattern {
    /// What the `*` stands for, or the whole name; `None` when it does not match.
    fn capture<'a>(&self, place: &'a str) -> Option<&'a str> {
        match &self.suffix {
            None => (place == self.prefix).then_some(place),
            Some(suffix) => place
                .strip_prefix(self.prefix.as_str())
                .and_then(|rest| rest.strip_suffix(suffix.as_str()))
                .filter(|middle| !middle.is_empty()),
        }
    }

    /// A whole name over any pattern, then the one that spells out more.
    fn specificity(&self) -> (bool, usize) {
        (
            self.suffix.is_none(),
            self.prefix.len() + self.suffix.as_ref().map_or(0, String::len),
        )
    }
}

/// The `ElectionDefaults` row without a pattern, and the rows with one.
fn election_rows(
    defaults: &Workbook,
) -> Result<(Option<Row>, Vec<Pattern>), Problem> {
    let mut every = None;
    let mut patterns: Vec<Pattern> = Vec::new();
    for given in defaults.rows(SHEET_ELECTION_DEFAULTS) {
        // The identity a schedule names an election by, and the post it is.
        if let Some(column) = ["alias", "election_post"]
            .into_iter()
            .find(|column| given.get(column).is_some())
        {
            return Err(Problem::error(
                Code::InvalidValue,
                given.origin(Some(column)).to_string(),
                "an election's alias and post are its polling place; set its name instead",
            )
            .at(&given.origin(Some(column))));
        }
        let origin = given.origin(Some(POLLING_PLACE));
        let duplicate = match given.text(POLLING_PLACE).map(str::trim) {
            None | Some("") => every.replace(given.clone()).is_some(),
            Some(text) => {
                let pattern = match text.split_once('*') {
                    None => Pattern {
                        prefix: text.to_string(),
                        suffix: None,
                        row: given.clone(),
                    },
                    Some((prefix, suffix)) if !suffix.contains('*') => {
                        Pattern {
                            prefix: prefix.to_string(),
                            suffix: Some(suffix.to_string()),
                            row: given.clone(),
                        }
                    }
                    Some(_) => {
                        return Err(Problem::error(
                            Code::InvalidValue,
                            origin.to_string(),
                            "a polling place pattern has one * at most",
                        )
                        .at(&origin));
                    }
                };
                let seen = patterns.iter().any(|p| {
                    p.prefix == pattern.prefix && p.suffix == pattern.suffix
                });
                patterns.push(pattern);
                seen
            }
        };
        if duplicate {
            return Err(Problem::error(
                Code::DuplicateId,
                origin.to_string(),
                "a defaults workbook gives each polling place pattern, and every election, one row at most",
            )
            .at(&origin));
        }
    }
    Ok((every, patterns))
}

/// The most specific pattern matching `place`, with what its `*` stood for.
fn most_specific<'a>(
    patterns: &'a [Pattern],
    place: &'a str,
) -> Result<Option<(&'a Row, &'a str)>, Problem> {
    let mut best: Option<(&Pattern, &str)> = None;
    for pattern in patterns {
        let Some(captured) = pattern.capture(place) else {
            continue;
        };
        match best {
            Some((other, _)) if other.specificity() > pattern.specificity() => {
            }
            Some((other, _))
                if other.specificity() == pattern.specificity() =>
            {
                let origin = pattern.row.origin(Some(POLLING_PLACE));
                return Err(Problem::error(
                    Code::ConflictingColumns,
                    origin.to_string(),
                    format!(
                        "polling place {place} matches two patterns equally; make one more specific"
                    ),
                )
                .at(&origin));
            }
            _ => best = Some((pattern, captured)),
        }
    }
    Ok(best.map(|(pattern, captured)| (&pattern.row, captured)))
}

/// `value` with `{place}` in its text replaced.
fn with_place(value: Value, place: &str) -> Value {
    match value {
        Value::String(text) => Value::String(text.replace("{place}", place)),
        other => other,
    }
}

/// The `Areas` row without a name, and the rows with one by name.
fn area_rows(
    defaults: &Workbook,
) -> Result<(Option<Row>, BTreeMap<String, Row>), Problem> {
    let mut every = None;
    let mut named = BTreeMap::new();
    for given in defaults.rows("areas") {
        let duplicate = match given.text("name").map(str::trim) {
            None | Some("") => every.replace(given.clone()).is_some(),
            Some(name) => {
                named.insert(name.to_string(), given.clone()).is_some()
            }
        };
        if duplicate {
            return Err(Problem::error(
                Code::DuplicateId,
                given.origin(Some("name")).to_string(),
                "a defaults workbook gives each area, and every area, one row at most",
            )
            .at(&given.origin(Some("name"))));
        }
    }
    Ok((every, named))
}

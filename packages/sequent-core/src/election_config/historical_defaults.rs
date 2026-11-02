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
//!   voters and gets no election; it is counted as excluded.
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
            if !ci.contains_key(&cluster)
                && areas[area].text("description") != Some(post.as_str())
            {
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
        // Two clusters may share a name; both take its row.
        let named = area_overrides.get(&name);
        if named.is_some() {
            overridden.insert(name.clone());
        }
        for given in area_defaults.iter().chain(named) {
            for (column, value) in given.without(&["name"]) {
                super::set(&mut area, &column, value);
            }
        }
        areas.push(area);
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

    let elections = posts
        .iter()
        .enumerate()
        .map(|(i, post)| {
            row(
                "Elections",
                i,
                vec![
                    ("election_post", json!(post)),
                    ("alias", json!(post)),
                    ("name", json!(post)),
                ],
            )
        })
        .collect();
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
                !matches!(sheet.key.as_str(), "areas" | "scheduledevents")
            })
            .cloned(),
    );
    Ok(Derived {
        workbook: Workbook::new(sheets)?,
        areas_by_precinct,
        schedule_rows,
    })
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

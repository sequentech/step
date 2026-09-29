// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Evaluating a resolved query against a snapshot payload.
//!
//! This is where the correctness rules of the monitoring figures live, in one
//! pure function both Harvest and the preset tests call:
//!
//! - a scope's figures are its own counts, never the sum of its groups or
//!   Posts, so a voter in two Posts is still one voter of the region;
//! - a ratio is its numerator over its denominator, both shown, and a zero
//!   denominator is "—", not 0%;
//! - a missing dimension value is the Unknown group, shown last and never
//!   dropped by a limit;
//! - a day is the sum of its hours, and both are `[start, end)` in the
//!   event's time zone, so buckets add up to the totals;
//! - a measure the producer did not count is refused, never shown as zero.
//!
//! The result's columns are fixed per template, which is what lets a chart
//! name them: `group`, `pct`, `bucket_start` and so on.

use super::config::{DimensionMapping, Ratio, Settings, SortKey, SortOrder};
use super::payload::{
    Counts, Cube, GroupRow, Notice, PostRow, ScopePayload, UNKNOWN_KEY,
    UNKNOWN_LABEL,
};
use super::problem::{Code, Problem, Report};
use super::resolve::ResolvedQuery;
use super::sources::{
    BuiltinDimension, DataSourceId, Measure, QueryTemplate, TimeGrain,
};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;

/// What a result column holds, so the renderer and the export need not guess
/// from the values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ColumnKind {
    Text,
    Integer,
    /// A fraction, or null where it is undefined.
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub kind: ColumnKind,
}

/// One query's rows, in the shape dbt Charts reads as inline values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<Column>,
    pub rows: Vec<Vec<Value>>,
    pub notices: Vec<Notice>,
}

/// Shown for a ratio whose denominator is zero.
pub const UNDEFINED_RATIO: &str = "—";

pub fn evaluate(
    source: DataSourceId,
    query: &ResolvedQuery,
    payload: &ScopePayload,
    settings: Option<&Settings>,
) -> Result<QueryResult, Report> {
    let mut report = Report::default();
    let spec = source.spec();
    let asked = query
        .measures
        .iter()
        .copied()
        .chain(query.ratio.iter().flat_map(|ratio| [ratio.0, ratio.1]));
    for measure in asked {
        if !spec.has_measure(measure) {
            report.push(Problem::error(
                Code::UnsupportedBySource,
                "measures",
                format!("{source} has no measure '{measure}'."),
            ));
        }
    }
    if !report.is_accepted() {
        return Err(report);
    }
    let evaluation = Evaluation {
        query,
        payload,
        settings,
    };
    let result = match query.template {
        QueryTemplate::Summary => evaluation.summary(&mut report),
        QueryTemplate::ByGroup => evaluation.by_group(&mut report),
        QueryTemplate::ByPost => evaluation.by_post(&mut report),
        QueryTemplate::Timeseries => evaluation.timeseries(&mut report),
        QueryTemplate::ByMeasure => evaluation.by_measure(&mut report),
    };
    match result {
        Some((columns, rows)) if report.is_accepted() => Ok(QueryResult {
            columns,
            rows,
            notices: payload.notices.clone(),
        }),
        _ => Err(report),
    }
}

/// The days with any activity, oldest first: the options of a day picker.
pub fn event_days(payload: &ScopePayload) -> Vec<String> {
    let mut days: Vec<String> = Vec::new();
    for bucket in &payload.series {
        let active = bucket.counts.values().any(|count| *count > 0);
        if active && days.last() != Some(&bucket.day) {
            days.push(bucket.day.clone());
        }
    }
    days
}

/// `53.2%` to one decimal, rounded half away from zero; `—` when the
/// denominator is zero.
pub fn percent_label(numerator: u64, denominator: u64) -> String {
    if denominator == 0 {
        return UNDEFINED_RATIO.to_string();
    }
    let (numerator, denominator) =
        (u128::from(numerator), u128::from(denominator));
    let tenths = (numerator * 2000 + denominator) / (denominator * 2);
    format!("{}.{}%", tenths / 10, tenths % 10)
}

struct Evaluation<'a> {
    query: &'a ResolvedQuery,
    payload: &'a ScopePayload,
    settings: Option<&'a Settings>,
}

type Table = (Vec<Column>, Vec<Vec<Value>>);

/// A labelled row before it becomes cells: a group, a Post.
struct Keyed {
    key: String,
    label: String,
    counts: Counts,
    unknown: bool,
}

fn column(name: impl Into<String>, kind: ColumnKind) -> Column {
    Column {
        name: name.into(),
        kind,
    }
}

fn not_counted(what: String) -> Problem {
    Problem::error(
        Code::NotCounted,
        "",
        format!(
            "The snapshot has no count of {what}; it is not shown as zero."
        ),
    )
}

fn count(
    counts: &Counts,
    measure: Measure,
    report: &mut Report,
) -> Option<u64> {
    let found = counts.get(&measure).copied();
    if found.is_none() {
        report.push(not_counted(format!("'{measure}'")));
    }
    found
}

fn ratio_value(numerator: u64, denominator: u64) -> Value {
    if denominator == 0 {
        Value::Null
    } else {
        json!(numerator as f64 / denominator as f64)
    }
}

impl Evaluation<'_> {
    /// The measure columns, then the ratio's four.
    fn value_columns(&self) -> Vec<Column> {
        let mut columns: Vec<Column> = self
            .query
            .measures
            .iter()
            .map(|measure| column(measure.to_string(), ColumnKind::Integer))
            .collect();
        if self.query.ratio.is_some() {
            columns.extend([
                column("numerator", ColumnKind::Integer),
                column("denominator", ColumnKind::Integer),
                column("pct", ColumnKind::Number),
                column("pct_label", ColumnKind::Text),
            ]);
        }
        columns
    }

    fn value_cells(&self, counts: &Counts, report: &mut Report) -> Vec<Value> {
        let mut cells: Vec<Value> = self
            .query
            .measures
            .iter()
            .map(|measure| json!(count(counts, *measure, report).unwrap_or(0)))
            .collect();
        if let Some(ratio) = self.query.ratio {
            let numerator =
                count(counts, ratio.numerator(), report).unwrap_or(0);
            let denominator =
                count(counts, ratio.denominator(), report).unwrap_or(0);
            cells.extend([
                json!(numerator),
                json!(denominator),
                ratio_value(numerator, denominator),
                json!(percent_label(numerator, denominator)),
            ]);
        }
        cells
    }

    fn summary(&self, report: &mut Report) -> Option<Table> {
        let totals = if self.query.filters.is_empty() {
            self.payload.totals.clone()
        } else {
            let cube = self.cube(report)?;
            let cells = self.filtered_cells(cube, report)?;
            sum(cells.iter().map(|cell| &cell.counts))
        };
        let row = self.value_cells(&totals, report);
        Some((self.value_columns(), vec![row]))
    }

    fn cube(&self, report: &mut Report) -> Option<&Cube> {
        let cube = self.payload.cube.as_ref();
        if cube.is_none() {
            report.push(not_counted("voters by their dimensions".to_string()));
        }
        cube
    }

    fn filtered_cells<'c>(
        &self,
        cube: &'c Cube,
        report: &mut Report,
    ) -> Option<Vec<&'c super::payload::CubeCell>> {
        let mut filters: Vec<(usize, &Vec<String>)> = Vec::new();
        for (dimension, values) in &self.query.filters {
            match cube.dimensions.iter().position(|name| name == dimension) {
                Some(at) => filters.push((at, values)),
                None => {
                    report
                        .push(not_counted(format!("voters by '{dimension}'")));
                    return None;
                }
            }
        }
        Some(
            cube.cells
                .iter()
                .filter(|cell| {
                    filters
                        .iter()
                        .all(|(at, values)| values.contains(&cell.values[*at]))
                })
                .collect(),
        )
    }

    fn by_group(&self, report: &mut Report) -> Option<Table> {
        let Some(dimension) = self.query.group_by.as_deref() else {
            report.push(Problem::error(
                Code::TemplateParameter,
                "group_by",
                "The by_group template needs a dimension to group by.",
            ));
            return None;
        };
        let rows = match dimension.parse::<BuiltinDimension>() {
            Ok(_) => self.builtin_groups(dimension, report)?,
            Err(_) => self.voter_groups(dimension, report)?,
        };
        let rows = self.sorted(rows, report);
        let mut columns = vec![
            column("group", ColumnKind::Text),
            column("group_key", ColumnKind::Text),
        ];
        columns.extend(self.value_columns());
        let rows = rows
            .into_iter()
            .map(|row| {
                let mut cells = vec![json!(row.label), json!(row.key)];
                cells.extend(self.value_cells(&row.counts, report));
                cells
            })
            .collect();
        Some((columns, rows))
    }

    fn builtin_groups(
        &self,
        dimension: &str,
        report: &mut Report,
    ) -> Option<Vec<Keyed>> {
        if !self.query.filters.is_empty() {
            report.push(Problem::error(
                Code::TemplateParameter,
                "filters",
                format!("Groups by '{dimension}' are counted on their own and cannot be filtered by voter dimensions."),
            ));
            return None;
        }
        let Some(rows) = self.payload.groups.get(dimension) else {
            report.push(not_counted(format!("groups by '{dimension}'")));
            return None;
        };
        Some(rows.iter().map(keyed_group).collect())
    }

    fn voter_groups(
        &self,
        dimension: &str,
        report: &mut Report,
    ) -> Option<Vec<Keyed>> {
        let cube = self.cube(report)?;
        let Some(at) =
            cube.dimensions.iter().position(|name| name == dimension)
        else {
            report.push(not_counted(format!("voters by '{dimension}'")));
            return None;
        };
        let cells = self.filtered_cells(cube, report)?;
        let mut groups: IndexMap<&str, Counts> = IndexMap::new();
        for cell in cells {
            let totals = groups.entry(cell.values[at].as_str()).or_default();
            add(totals, &cell.counts);
        }
        let mapping = self
            .settings
            .and_then(|settings| settings.dimensions.get(dimension));
        let mut rows: Vec<Keyed> = groups
            .into_iter()
            .map(|(key, counts)| {
                let unknown = key == UNKNOWN_KEY;
                let label = if unknown {
                    UNKNOWN_LABEL.to_string()
                } else {
                    mapping
                        .and_then(|mapping| mapping.labels.get(key))
                        .cloned()
                        .unwrap_or_else(|| key.to_string())
                };
                Keyed {
                    key: key.to_string(),
                    label,
                    counts,
                    unknown,
                }
            })
            .collect();
        rows.sort_by(|left, right| configured_order(mapping, left, right));
        Some(rows)
    }

    /// The query's sort and limit over known rows; Unknown stays last and is
    /// kept whatever the limit, so no voter silently drops out of a chart.
    fn sorted(&self, rows: Vec<Keyed>, report: &mut Report) -> Vec<Keyed> {
        let (mut known, unknown): (Vec<Keyed>, Vec<Keyed>) =
            rows.into_iter().partition(|row| !row.unknown);
        if let Some(sort) = self.query.sort {
            let mut keyed: Vec<(Keyed, SortValue)> = known
                .into_iter()
                .map(|row| {
                    let value = self
                        .sort_value(sort.by, &row, report)
                        .unwrap_or(SortValue::Undefined);
                    (row, value)
                })
                .collect();
            keyed.sort_by(|(_, left), (_, right)| {
                left.compare(right, sort.order)
            });
            known = keyed.into_iter().map(|(row, _)| row).collect();
        }
        if let Some(limit) = self.query.limit {
            known.truncate(limit as usize);
        }
        known.extend(unknown);
        known
    }

    fn sort_value(
        &self,
        by: SortKey,
        row: &Keyed,
        report: &mut Report,
    ) -> Option<SortValue> {
        match by {
            SortKey::Label => Some(SortValue::Text(row.label.clone())),
            SortKey::Value => {
                let measure = self
                    .query
                    .measures
                    .first()
                    .copied()
                    .or(self.query.ratio.map(|ratio| ratio.numerator()))?;
                count(&row.counts, measure, report).map(SortValue::Count)
            }
            SortKey::Ratio => {
                let Ratio(numerator, denominator) = self.query.ratio?;
                let numerator = count(&row.counts, numerator, report)?;
                let denominator = count(&row.counts, denominator, report)?;
                Some(if denominator == 0 {
                    SortValue::Undefined
                } else {
                    SortValue::Fraction(numerator as f64 / denominator as f64)
                })
            }
        }
    }

    fn by_post(&self, report: &mut Report) -> Option<Table> {
        let stated = self.payload.posts.iter().any(|post| post.state.is_some());
        let mut columns = vec![
            column("post", ColumnKind::Text),
            column("post_id", ColumnKind::Text),
            column("region", ColumnKind::Text),
        ];
        if stated {
            columns.extend([
                column("state", ColumnKind::Text),
                column("state_label", ColumnKind::Text),
            ]);
        }
        columns.extend(self.value_columns());

        let by_id: IndexMap<&str, &PostRow> = self
            .payload
            .posts
            .iter()
            .map(|post| (post.post_id.as_str(), post))
            .collect();
        let keyed: Vec<Keyed> = self
            .payload
            .posts
            .iter()
            .map(|post| Keyed {
                key: post.post_id.clone(),
                label: post.post.clone(),
                counts: post.counts.clone(),
                unknown: false,
            })
            .collect();
        let rows = self
            .sorted(keyed, report)
            .into_iter()
            .filter_map(|row| by_id.get(row.key.as_str()).copied())
            .map(|post| {
                let mut cells = vec![
                    json!(post.post),
                    json!(post.post_id),
                    post.region
                        .as_ref()
                        .map_or(Value::Null, |region| json!(region)),
                ];
                if stated {
                    match post.state {
                        Some(state) => cells.extend([
                            json!(state.to_string()),
                            json!(self.label(
                                &state.to_string(),
                                state.default_label()
                            )),
                        ]),
                        None => cells.extend([Value::Null, Value::Null]),
                    }
                }
                cells.extend(self.value_cells(&post.counts, report));
                cells
            })
            .collect();
        Some((columns, rows))
    }

    fn timeseries(&self, report: &mut Report) -> Option<Table> {
        let Some(grain) = self.query.grain else {
            report.push(Problem::error(
                Code::TemplateParameter,
                "grain",
                "The timeseries template needs a grain.",
            ));
            return None;
        };
        // (start, label, day, counts), oldest first.
        let mut buckets: Vec<(String, String, String, Counts)> = Vec::new();
        for bucket in &self.payload.series {
            match grain {
                TimeGrain::Hour => buckets.push((
                    bucket.start.clone(),
                    bucket.start.get(11..16).unwrap_or_default().to_string(),
                    bucket.day.clone(),
                    bucket.counts.clone(),
                )),
                TimeGrain::Day => match buckets.last_mut() {
                    Some((_, _, day, counts)) if *day == bucket.day => {
                        add(counts, &bucket.counts)
                    }
                    _ => buckets.push((
                        format!("{}T00:00:00", bucket.day),
                        bucket.day.clone(),
                        bucket.day.clone(),
                        bucket.counts.clone(),
                    )),
                },
            }
        }

        let mut columns = vec![
            column("bucket_start", ColumnKind::Text),
            column("bucket_label", ColumnKind::Text),
        ];
        for measure in &self.query.measures {
            columns.push(column(measure.to_string(), ColumnKind::Integer));
            columns.push(column(
                format!("{measure}_cumulative"),
                ColumnKind::Integer,
            ));
        }

        let mut running: Counts = Counts::new();
        let mut rows = Vec::new();
        for (start, label, day, counts) in buckets {
            let mut cells = vec![json!(start), json!(label)];
            for measure in &self.query.measures {
                let value = count(&counts, *measure, report)?;
                let total = running.entry(*measure).or_default();
                *total += value;
                cells.push(json!(value));
                cells.push(json!(*total));
            }
            let shown = match &self.query.day {
                Some(wanted) => grain == TimeGrain::Day || *wanted == day,
                None => true,
            };
            if shown {
                rows.push(cells);
            }
        }
        Some((columns, rows))
    }

    fn by_measure(&self, report: &mut Report) -> Option<Table> {
        let columns = vec![
            column("measure", ColumnKind::Text),
            column("label", ColumnKind::Text),
            column("value", ColumnKind::Integer),
        ];
        let totals = if self.query.filters.is_empty() {
            self.payload.totals.clone()
        } else {
            let cube = self.cube(report)?;
            sum(self
                .filtered_cells(cube, report)?
                .iter()
                .map(|cell| &cell.counts))
        };
        let rows = self
            .query
            .measures
            .iter()
            .map(|measure| {
                let key = measure.to_string();
                vec![
                    json!(key),
                    json!(self.label(&key, measure.default_label())),
                    json!(count(&totals, *measure, report).unwrap_or(0)),
                ]
            })
            .collect();
        Some((columns, rows))
    }

    fn label(&self, key: &str, default: &str) -> String {
        self.query
            .labels
            .get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }
}

fn keyed_group(row: &GroupRow) -> Keyed {
    let unknown = row.key == UNKNOWN_KEY;
    Keyed {
        key: row.key.clone(),
        label: if unknown {
            UNKNOWN_LABEL.to_string()
        } else {
            row.label.clone().unwrap_or_else(|| row.key.clone())
        },
        counts: row.counts.clone(),
        unknown,
    }
}

/// Age bands in band order; otherwise labelled values in the order the
/// settings list them, then the rest by value; Unknown last.
fn configured_order(
    mapping: Option<&DimensionMapping>,
    left: &Keyed,
    right: &Keyed,
) -> Ordering {
    let rank = |row: &Keyed| -> (u8, usize, String) {
        if row.unknown {
            return (2, 0, String::new());
        }
        let position = mapping.and_then(|mapping| {
            mapping
                .age_bands
                .iter()
                .position(|band| band.label == row.key)
                .or_else(|| mapping.labels.get_index_of(row.key.as_str()))
        });
        match position {
            Some(position) => (0, position, String::new()),
            None => (1, 0, row.key.clone()),
        }
    };
    rank(left).cmp(&rank(right))
}

fn add(into: &mut Counts, counts: &Counts) {
    for (measure, value) in counts {
        *into.entry(*measure).or_default() += value;
    }
}

/// The sum of partition cells. Only ever called on a partition — never on
/// groups or Posts, which overlap.
fn sum<'c>(cells: impl Iterator<Item = &'c Counts>) -> Counts {
    let mut totals = Counts::new();
    for counts in cells {
        add(&mut totals, counts);
    }
    totals
}

#[derive(Debug, Clone, PartialEq)]
enum SortValue {
    Text(String),
    Count(u64),
    Fraction(f64),
    /// A ratio over zero: last in either order.
    Undefined,
}

impl SortValue {
    fn compare(&self, other: &SortValue, order: SortOrder) -> Ordering {
        let ordered = match (self, other) {
            (SortValue::Undefined, SortValue::Undefined) => {
                return Ordering::Equal
            }
            (SortValue::Undefined, _) => return Ordering::Greater,
            (_, SortValue::Undefined) => return Ordering::Less,
            (SortValue::Text(left), SortValue::Text(right)) => left.cmp(right),
            (SortValue::Count(left), SortValue::Count(right)) => {
                left.cmp(right)
            }
            (SortValue::Fraction(left), SortValue::Fraction(right)) => {
                left.total_cmp(right)
            }
            _ => Ordering::Equal,
        };
        match order {
            SortOrder::Asc => ordered,
            SortOrder::Desc => ordered.reverse(),
        }
    }
}

#[cfg(test)]
#[path = "compute_tests.rs"]
mod compute_tests;

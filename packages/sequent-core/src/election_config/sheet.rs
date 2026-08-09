// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! The shape of an authoring document: sheets of rows of coerced cells.
//!
//! One sheet per entity, header row first, one row per entity. This module does
//! no interpretation beyond coercing cells and remembering where each row came
//! from; resolving references and applying templates is a later step's job.
//!
//! Pure, and deliberately unaware of any spreadsheet library. Everything here
//! works on [`Cell`]s, so a `.xlsx` reader, a CSV reader and a browser form all
//! feed the same code — the `xlsx` module is the only one that needs a file
//! format. Which means the whole of table shaping, the part with the awkward
//! cases in it, is testable without a fixture file.
//!
//! Known gap: a problem found here carries its [`Origin`] flattened into
//! `Problem::path`. A spreadsheet front end that wants to highlight the offending
//! cell needs the sheet, row and column separately, which will mean a structured
//! origin on `Problem` — worth doing when there is a UI to consume it, not
//! before.

use crate::election_config::paths::{coerce_cell, expand, Cell};
use crate::election_config::problem::{Code, Problem};
use serde_json::{Map, Value};
use std::fmt;

/// Sheet keys this module understands.
///
/// Normalised: case and internal whitespace are removed, so `Admin Users`,
/// `admin users` and `AdminUsers` are one sheet. Authors rename tabs.
pub const SHEET_ELECTION_EVENT: &str = "electionevent";
pub const SHEET_ELECTIONS: &str = "elections";
pub const SHEET_CONTESTS: &str = "contests";
pub const SHEET_CANDIDATES: &str = "candidates";
pub const SHEET_AREAS: &str = "areas";
pub const SHEET_AREA_CONTESTS: &str = "areacontests";
pub const SHEET_VOTERS: &str = "voters";
pub const SHEET_SCHEDULED_EVENTS: &str = "scheduledevents";
pub const SHEET_PARAMETERS: &str = "parameters";
pub const SHEET_ADMIN_USERS: &str = "adminusers";
pub const SHEET_PERMISSIONS: &str = "permissions";
pub const SHEET_TEMPLATES: &str = "templates";
pub const SHEET_REPORTS: &str = "reports";

/// Voter-facing help documents — rules, candidate statements, a guide to voting.
///
/// The sheet names each file; the bytes are supplied beside the workbook, the same
/// way a candidate's photograph is, because a spreadsheet cell cannot hold one. See
/// `engineering/how-a-support-material-travels-in-a-bundle` in beyond.
pub const SHEET_MATERIALS: &str = "materials";

/// Every sheet that carries meaning. Anything else is reported as unread, which
/// is how a renamed or misspelled tab gets noticed instead of silently ignored.
pub const KNOWN_SHEETS: &[&str] = &[
    SHEET_ELECTION_EVENT,
    SHEET_ELECTIONS,
    SHEET_CONTESTS,
    SHEET_CANDIDATES,
    SHEET_AREAS,
    SHEET_AREA_CONTESTS,
    SHEET_VOTERS,
    SHEET_SCHEDULED_EVENTS,
    SHEET_PARAMETERS,
    SHEET_ADMIN_USERS,
    SHEET_PERMISSIONS,
    SHEET_TEMPLATES,
    SHEET_REPORTS,
    SHEET_MATERIALS,
];

/// Columns whose cells hold `||`-separated lists, for one sheet.
///
/// Per sheet rather than global, because the same column name has different
/// arity in different places: `Election::permission_label` is `Option<String>`
/// while `Report::permission_label` is `Option<Vec<String>>`. Treating the name
/// as multi-valued everywhere turns the first into a list and fails
/// deserialization — which is exactly the bug this shape prevents.
pub fn multi_value_columns(sheet_key: &str) -> &'static [&'static str] {
    match sheet_key {
        SHEET_VOTERS => &["authorized-election-ids"],
        SHEET_ADMIN_USERS => &["permission_labels", "authorized-election-ids"],
        SHEET_REPORTS => &["permission_label"],
        _ => &[],
    }
}

/// `"Admin Users"` -> `"adminusers"`.
pub fn normalise_sheet_name(name: &str) -> String {
    name.chars()
        .filter(|character| !character.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Where in the source document something is, for a message someone can act on.
///
/// A bundle path like `elections[3].title` is no help to whoever has to fix the
/// spreadsheet; the sheet name and the row number as the spreadsheet shows it
/// are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origin {
    pub sheet: String,

    /// The row as the spreadsheet numbers them, or `0` for the sheet as a whole.
    ///
    /// Spreadsheets count from one, so zero cannot name a real row and is free to
    /// mean "this is about the sheet, not a row in it" — which is what a missing
    /// column or an empty sheet is about.
    pub row: usize,

    pub column: Option<String>,
}

impl Origin {
    /// A problem with a sheet rather than with any row of it.
    pub fn sheet(name: impl Into<String>) -> Self {
        Origin {
            sheet: name.into(),
            row: 0,
            column: None,
        }
    }

    /// A problem with a whole column.
    pub fn column(name: impl Into<String>, column: impl Into<String>) -> Self {
        Origin {
            sheet: name.into(),
            row: 0,
            column: Some(column.into()),
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "sheet '{}'", self.sheet)?;
        if self.row > 0 {
            write!(formatter, " row {}", self.row)?;
        }
        if let Some(column) = &self.column {
            write!(formatter, " column '{column}'")?;
        }
        Ok(())
    }
}

/// One entity's worth of cells, plus where it came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub sheet: String,

    /// Row number as the spreadsheet shows it, so a message is actionable.
    pub number: usize,

    /// Non-blank cells only, keyed by raw dotted header, in column order.
    ///
    /// Ordered rather than a map because two columns writing the same path have
    /// to resolve left to right, the way someone reading the sheet would expect.
    /// Blank cells are absent rather than null: the author said nothing about
    /// them, so a template default survives.
    pub cells: Vec<(String, Value)>,
}

impl Row {
    pub fn origin(&self, column: Option<&str>) -> Origin {
        Origin {
            sheet: self.sheet.clone(),
            row: self.number,
            column: column.map(str::to_string),
        }
    }

    pub fn get(&self, column: &str) -> Option<&Value> {
        self.cells
            .iter()
            .find(|(header, _)| header == column)
            .map(|(_, value)| value)
    }

    /// The cell as text, for the reference columns that are always strings.
    pub fn text(&self, column: &str) -> Option<&str> {
        self.get(column).and_then(Value::as_str)
    }

    /// The cell, or a problem naming the empty one.
    pub fn require(&self, column: &str) -> Result<&Value, Problem> {
        self.get(column).ok_or_else(|| {
            Problem::error(
                Code::MissingField,
                self.origin(Some(column)).to_string(),
                format!("'{column}' is required and this row leaves it empty"),
            )
        })
    }

    /// Cells minus `exclude`, in column order.
    ///
    /// Used to strip the reference and control columns before the rest of the
    /// row is merged onto a rendered template as dotted-path overrides.
    pub fn without(&self, exclude: &[&str]) -> Vec<(String, Value)> {
        self.cells
            .iter()
            .filter(|(header, _)| !exclude.contains(&header.as_str()))
            .cloned()
            .collect()
    }

    /// The row as a nested object, ready to deep-merge onto a template.
    pub fn overrides(
        &self,
        exclude: &[&str],
    ) -> Result<Map<String, Value>, Problem> {
        expand(&self.without(exclude)).map_err(|problem| Problem {
            // Re-point at the cell: `expand` only knows the header.
            path: self.origin(Some(problem.path.as_str())).to_string(),
            ..problem
        })
    }
}

/// One worksheet, read and coerced.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sheet {
    /// Name as it appears in the document, for messages.
    pub name: String,

    /// Normalised name, for lookup.
    pub key: String,

    /// Headers in column order. A blank header is kept as an empty string so
    /// column positions still line up with the cells beneath them.
    pub headers: Vec<String>,

    pub rows: Vec<Row>,
}

impl Sheet {
    /// Shape a grid of cells into a sheet: first row is headers, rest are rows.
    ///
    /// Blank rows are dropped rather than trusted. A spreadsheet's stored
    /// dimensions count rows that were merely visited — the SEIU1000 sample
    /// reports a thousand rows for a sheet holding two — so emptiness has to be
    /// decided from the cells.
    pub fn from_grid(
        name: impl Into<String>,
        grid: &[Vec<Cell>],
    ) -> Result<Self, Problem> {
        let name: String = name.into();
        let key = normalise_sheet_name(&name);

        let Some(header_cells) = grid.first() else {
            return Ok(Sheet {
                name,
                key,
                ..Sheet::default()
            });
        };

        let headers = read_headers(&name, header_cells)?;
        let multi_value = multi_value_columns(&key);

        let mut rows = Vec::new();
        for (offset, cells) in grid.iter().skip(1).enumerate() {
            // +1 for the header row, +1 because spreadsheets count from one.
            let number = offset + 2;
            if let Some(row) =
                read_row(&name, number, &headers, cells, multi_value)
            {
                rows.push(row);
            }
        }

        Ok(Sheet {
            name,
            key,
            headers,
            rows,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }
}

fn read_headers(
    sheet_name: &str,
    cells: &[Cell],
) -> Result<Vec<String>, Problem> {
    let mut headers: Vec<String> = Vec::with_capacity(cells.len());

    for (index, cell) in cells.iter().enumerate() {
        let header = match cell {
            // A blank header means no column. Trailing blank columns are as
            // common as trailing blank rows and mean nothing; keeping the
            // placeholder preserves the positions of the ones that follow.
            Cell::Blank => String::new(),
            Cell::Text(text) => text.trim().to_string(),
            other => {
                match crate::election_config::paths::coerce_scalar_cell(other) {
                    Some(Value::String(text)) => text,
                    Some(value) => value.to_string(),
                    None => String::new(),
                }
            }
        };

        if !header.is_empty() {
            if let Some(first) = headers.iter().position(|seen| seen == &header)
            {
                return Err(Problem::error(
                    Code::ConflictingColumns,
                    format!("sheet '{sheet_name}'"),
                    format!(
                        "column '{header}' appears twice, at positions {} and {}. \
                         Which one wins would be arbitrary.",
                        first + 1,
                        index + 1
                    ),
                ));
            }
        }
        headers.push(header);
    }

    Ok(headers)
}

/// One data row, or `None` if it holds nothing.
fn read_row(
    sheet_name: &str,
    number: usize,
    headers: &[String],
    cells: &[Cell],
    multi_value: &[&str],
) -> Option<Row> {
    let mut values: Vec<(String, Value)> = Vec::new();

    for (header, cell) in headers.iter().zip(cells) {
        if header.is_empty() || cell.is_blank() {
            continue;
        }
        if let Some(value) =
            coerce_cell(cell, multi_value.contains(&header.as_str()))
        {
            values.push((header.clone(), value));
        }
    }

    if values.is_empty() {
        return None;
    }
    Some(Row {
        sheet: sheet_name.to_string(),
        number,
        cells: values,
    })
}

/// A whole authoring document: normalised sheets of coerced rows.
#[derive(Debug, Clone, Default)]
pub struct Workbook {
    sheets: Vec<Sheet>,
}

impl Workbook {
    /// Refuses two sheets that normalise to the same key.
    ///
    /// `Admin Users` beside `AdminUsers` is not a document with a duplicate tab;
    /// it is a document where nobody knows which tab is live, and picking one
    /// silently would eventually import the wrong voters.
    pub fn new(sheets: Vec<Sheet>) -> Result<Self, Problem> {
        for (index, sheet) in sheets.iter().enumerate() {
            if let Some(earlier) =
                sheets[..index].iter().find(|seen| seen.key == sheet.key)
            {
                return Err(Problem::error(
                    Code::ConflictingColumns,
                    format!("sheet '{}'", sheet.name),
                    format!(
                        "'{}' and '{}' are the same sheet once names are \
                         normalised. Which one is meant cannot be guessed.",
                        earlier.name, sheet.name
                    ),
                ));
            }
        }
        Ok(Workbook { sheets })
    }

    pub fn sheet(&self, key: &str) -> Option<&Sheet> {
        self.sheets.iter().find(|sheet| sheet.key == key)
    }

    /// The sheet's rows, or none.
    ///
    /// Absent and empty are the same thing to every caller: a document with no
    /// `Reports` sheet and one with an empty `Reports` sheet both mean no
    /// reports.
    pub fn rows(&self, key: &str) -> &[Row] {
        self.sheet(key).map_or(&[], |sheet| sheet.rows.as_slice())
    }

    pub fn has(&self, key: &str) -> bool {
        self.sheet(key).is_some()
    }

    /// Sheets that carry no meaning here, so a caller can warn about a typo.
    pub fn unread_sheets(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .sheets
            .iter()
            .filter(|sheet| !KNOWN_SHEETS.contains(&sheet.key.as_str()))
            .map(|sheet| sheet.name.as_str())
            .collect();
        names.sort_unstable();
        names
    }

    pub fn sheet_names(&self) -> Vec<&str> {
        self.sheets
            .iter()
            .map(|sheet| sheet.name.as_str())
            .collect()
    }

    pub fn sheets(&self) -> &[Sheet] {
        &self.sheets
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn text(value: &str) -> Cell {
        Cell::text(value)
    }

    fn grid(rows: Vec<Vec<Cell>>) -> Vec<Vec<Cell>> {
        rows
    }

    #[test]
    fn a_tab_name_normalises_to_a_key() {
        // Authors rename tabs, and "Admin Users" is the same sheet as
        // "adminusers".
        assert_eq!(normalise_sheet_name("Admin Users"), "adminusers");
        assert_eq!(normalise_sheet_name("  ELECTIONS "), "elections");
        assert_eq!(normalise_sheet_name("Area\tContests"), "areacontests");
    }

    #[test]
    fn every_named_sheet_is_in_the_known_list() {
        // A constant nobody added to the list would be read but reported unread.
        for key in [
            SHEET_ELECTION_EVENT,
            SHEET_ELECTIONS,
            SHEET_CONTESTS,
            SHEET_CANDIDATES,
            SHEET_AREAS,
            SHEET_AREA_CONTESTS,
            SHEET_VOTERS,
            SHEET_SCHEDULED_EVENTS,
            SHEET_PARAMETERS,
            SHEET_ADMIN_USERS,
            SHEET_MATERIALS,
            SHEET_PERMISSIONS,
            SHEET_TEMPLATES,
            SHEET_REPORTS,
        ] {
            assert!(
                KNOWN_SHEETS.contains(&key),
                "{key} missing from KNOWN_SHEETS"
            );
        }
    }

    #[test]
    fn multi_value_is_decided_per_sheet_not_globally() {
        // Election::permission_label is Option<String> and
        // Report::permission_label is Option<Vec<String>>. One name, two arities.
        assert!(
            multi_value_columns(SHEET_REPORTS).contains(&"permission_label")
        );
        assert!(
            !multi_value_columns(SHEET_ELECTIONS).contains(&"permission_label")
        );
    }

    #[test]
    fn a_permission_label_stays_a_string_on_the_elections_sheet() {
        // The regression this shape exists for: as a list it fails to
        // deserialize into Option<String>.
        let sheet = Sheet::from_grid(
            "Elections",
            &grid(vec![
                vec![text("external_id"), text("permission_label")],
                vec![text("board"), text("statewide-officers")],
            ]),
        )
        .unwrap();
        assert_eq!(
            sheet.rows[0].get("permission_label"),
            Some(&json!("statewide-officers"))
        );
    }

    #[test]
    fn the_same_column_on_the_reports_sheet_is_a_list() {
        let sheet = Sheet::from_grid(
            "Reports",
            &grid(vec![
                vec![text("external_id"), text("permission_label")],
                vec![text("tally"), text("a || b")],
            ]),
        )
        .unwrap();
        assert_eq!(
            sheet.rows[0].get("permission_label"),
            Some(&json!(["a", "b"]))
        );
    }

    #[test]
    fn a_single_value_in_a_multi_value_column_is_still_a_list() {
        let sheet = Sheet::from_grid(
            "Reports",
            &grid(vec![vec![text("permission_label")], vec![text("only")]]),
        )
        .unwrap();
        assert_eq!(
            sheet.rows[0].get("permission_label"),
            Some(&json!(["only"]))
        );
    }

    #[test]
    fn blank_rows_are_dropped_rather_than_trusted() {
        // A spreadsheet's stored dimensions count rows that were merely visited:
        // the SEIU1000 sample claims a thousand rows for a sheet holding two.
        let sheet = Sheet::from_grid(
            "Elections",
            &grid(vec![
                vec![text("external_id")],
                vec![text("board")],
                vec![Cell::Blank],
                vec![text("   ")],
                vec![text("council")],
                vec![Cell::Blank],
            ]),
        )
        .unwrap();
        assert_eq!(sheet.len(), 2);
        assert_eq!(sheet.rows[0].text("external_id"), Some("board"));
        assert_eq!(sheet.rows[1].text("external_id"), Some("council"));
    }

    #[test]
    fn a_row_number_is_the_one_the_spreadsheet_shows() {
        // Off by one here means an author looking at the wrong line.
        let sheet = Sheet::from_grid(
            "Elections",
            &grid(vec![
                vec![text("external_id")],
                vec![text("first")],
                vec![text("second")],
            ]),
        )
        .unwrap();
        assert_eq!(sheet.rows[0].number, 2);
        assert_eq!(sheet.rows[1].number, 3);
    }

    #[test]
    fn a_dropped_blank_row_does_not_shift_the_numbers_after_it() {
        let sheet = Sheet::from_grid(
            "Elections",
            &grid(vec![
                vec![text("external_id")],
                vec![Cell::Blank],
                vec![text("after the gap")],
            ]),
        )
        .unwrap();
        assert_eq!(sheet.rows[0].number, 3);
    }

    #[test]
    fn a_blank_header_ends_the_table_without_shifting_the_rest() {
        // Trailing blank columns are as common as trailing blank rows.
        let sheet = Sheet::from_grid(
            "Elections",
            &grid(vec![
                vec![text("a"), Cell::Blank, text("c")],
                vec![text("1"), text("ignored"), text("3")],
            ]),
        )
        .unwrap();
        assert_eq!(sheet.headers, vec!["a", "", "c"]);
        assert_eq!(sheet.rows[0].cells.len(), 2);
        assert_eq!(sheet.rows[0].get("c"), Some(&json!("3")));
    }

    #[test]
    fn a_duplicated_column_is_refused() {
        // Which one wins would be arbitrary, and the author cannot see the
        // difference.
        let problem = Sheet::from_grid(
            "Elections",
            &grid(vec![vec![text("title"), text("title")]]),
        )
        .unwrap_err();
        assert_eq!(problem.code, Code::ConflictingColumns);
        assert!(problem.message.contains("positions 1 and 2"));
    }

    #[test]
    fn a_header_that_differs_only_by_padding_is_still_a_duplicate() {
        assert!(Sheet::from_grid(
            "Elections",
            &grid(vec![vec![text("title"), text("  title  ")]]),
        )
        .is_err());
    }

    #[test]
    fn a_text_cell_that_looks_numeric_stays_text() {
        // Deliberate, and the same as the Python: a spreadsheet hands over a
        // number for a numeric cell, so text here means the author formatted the
        // column as text — which is how member id 007 stays 007. A number typed
        // as a number arrives as `Cell::Float` and does become an integer.
        let sheet = Sheet::from_grid(
            "Voters",
            &grid(vec![
                vec![text("member_id"), text("max_votes")],
                vec![text("007"), Cell::Float(3.0)],
            ]),
        )
        .unwrap();
        assert_eq!(sheet.rows[0].get("member_id"), Some(&json!("007")));
        assert_eq!(sheet.rows[0].get("max_votes"), Some(&json!(3)));
    }

    #[test]
    fn a_numeric_header_is_read_as_a_name() {
        // A year used as a column name is a name, not a number.
        let sheet = Sheet::from_grid(
            "Parameters",
            &grid(vec![vec![Cell::Int(2027)], vec![text("x")]]),
        )
        .unwrap();
        assert_eq!(sheet.headers, vec!["2027"]);
    }

    #[test]
    fn an_empty_grid_is_an_empty_sheet_rather_than_an_error() {
        let sheet = Sheet::from_grid("Reports", &grid(vec![])).unwrap();
        assert!(sheet.is_empty());
        assert_eq!(sheet.key, "reports");
    }

    #[test]
    fn a_header_row_with_no_rows_under_it_is_empty_too() {
        let sheet =
            Sheet::from_grid("Reports", &grid(vec![vec![text("external_id")]]))
                .unwrap();
        assert!(sheet.is_empty());
        assert_eq!(sheet.headers, vec!["external_id"]);
    }

    #[test]
    fn short_rows_do_not_invent_cells() {
        // Spreadsheets truncate trailing empties; zip stops at the shorter side.
        let sheet = Sheet::from_grid(
            "Elections",
            &grid(vec![vec![text("a"), text("b"), text("c")], vec![text("1")]]),
        )
        .unwrap();
        assert_eq!(sheet.rows[0].cells.len(), 1);
        assert_eq!(sheet.rows[0].get("b"), None);
    }

    #[test]
    fn a_row_hands_over_its_overrides_as_nested_json() {
        let sheet = Sheet::from_grid(
            "Contests",
            &grid(vec![
                vec![
                    text("external_id"),
                    text("election_id"),
                    text("presentation.i18n.en.name"),
                    text("max_votes"),
                ],
                vec![
                    text("president"),
                    text("board"),
                    text("President"),
                    Cell::Float(3.0),
                ],
            ]),
        )
        .unwrap();

        let overrides = sheet.rows[0]
            .overrides(&["external_id", "election_id"])
            .unwrap();
        assert_eq!(
            Value::Object(overrides),
            json!({
                "presentation": {"i18n": {"en": {"name": "President"}}},
                "max_votes": 3,
            })
        );
    }

    #[test]
    fn a_shape_conflict_in_a_row_names_the_cell_not_just_the_column() {
        // The author has to find it in the spreadsheet.
        let problem = Sheet::from_grid(
            "Contests",
            &grid(vec![
                vec![text("presentation"), text("presentation.i18n")],
                vec![text("plain"), text("{}")],
            ]),
        )
        .unwrap()
        .rows[0]
            .overrides(&[])
            .unwrap_err();
        assert_eq!(problem.code, Code::ConflictingColumns);
        assert!(problem.path.contains("sheet 'Contests' row 2"));
        assert!(problem.path.contains("presentation.i18n"));
    }

    #[test]
    fn a_missing_required_cell_names_where_to_look() {
        let sheet = Sheet::from_grid(
            "Elections",
            &grid(vec![
                vec![text("external_id"), text("title")],
                vec![text("board"), Cell::Blank],
            ]),
        )
        .unwrap();
        let problem = sheet.rows[0].require("title").unwrap_err();
        assert_eq!(problem.code, Code::MissingField);
        assert_eq!(problem.path, "sheet 'Elections' row 2 column 'title'");
    }

    #[test]
    fn an_origin_about_a_whole_sheet_does_not_claim_a_row() {
        // "row 0" names no row a spreadsheet has, and reads as a bug.
        assert_eq!(
            Origin::sheet("Parameters").to_string(),
            "sheet 'Parameters'"
        );
        assert_eq!(
            Origin::column("Voters", "home address").to_string(),
            "sheet 'Voters' column 'home address'"
        );
    }

    #[test]
    fn an_origin_reads_as_a_place() {
        let row = Row {
            sheet: "Voters".to_string(),
            number: 47,
            cells: vec![],
        };
        assert_eq!(row.origin(None).to_string(), "sheet 'Voters' row 47");
        assert_eq!(
            row.origin(Some("email")).to_string(),
            "sheet 'Voters' row 47 column 'email'"
        );
    }

    #[test]
    fn an_absent_sheet_and_an_empty_one_read_the_same() {
        // Callers treat both as "no reports", so neither may be a special case.
        let workbook = Workbook::new(vec![Sheet::from_grid(
            "Reports",
            &grid(vec![vec![text("external_id")]]),
        )
        .unwrap()])
        .unwrap();
        assert!(workbook.rows(SHEET_REPORTS).is_empty());
        assert!(workbook.rows(SHEET_VOTERS).is_empty());
        assert!(workbook.has(SHEET_REPORTS));
        assert!(!workbook.has(SHEET_VOTERS));
    }

    #[test]
    fn two_tabs_that_normalise_alike_are_refused() {
        // Not a duplicate tab: a document where nobody knows which tab is live.
        let problem = Workbook::new(vec![
            Sheet::from_grid("Admin Users", &grid(vec![])).unwrap(),
            Sheet::from_grid("AdminUsers", &grid(vec![])).unwrap(),
        ])
        .unwrap_err();
        assert!(problem.message.contains("the same sheet"));
    }

    #[test]
    fn sheets_nobody_reads_are_listed_so_a_typo_shows_up() {
        let workbook = Workbook::new(vec![
            Sheet::from_grid("Elections", &grid(vec![])).unwrap(),
            Sheet::from_grid("Electons", &grid(vec![])).unwrap(),
            Sheet::from_grid("Read Me", &grid(vec![])).unwrap(),
        ])
        .unwrap();
        assert_eq!(workbook.unread_sheets(), vec!["Electons", "Read Me"]);
        assert_eq!(
            workbook.sheet_names(),
            vec!["Elections", "Electons", "Read Me"]
        );
    }
}

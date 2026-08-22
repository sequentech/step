// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What travels beside a plan, rather than inside it.
//!
//! A [`Blueprint`](super::architect::Blueprint) is a *description* of an election.
//! It used to be a description that also carried the bulk it described — the whole
//! census, the logo's bytes, every candidate's photograph — which made a plan for a
//! union of six figures a JSON document of six figures, and a plan for ten million
//! members impossible. The census was written **three times into one delivery**:
//! into `blueprint.json`, into `election_workbook.xlsx`, and into
//! `export_voters-<id>.csv`, which is the only one the platform actually reads.
//!
//! So the plan describes and this carries. Nothing here is a second copy of
//! anything: the census lives in its CSV, the files live under their own names, and
//! the plan names neither a row count nor a column list, because a recorded
//! expectation is the duplication again in miniature — it can disagree with the
//! thing it describes, and then somebody has to decide which is right.
//!
//! **The shape is chosen for the streaming that comes next.** Phase one may hold a
//! census in memory; what it may not do is settle on an API that has to change when
//! it stops. Hence a trait: a file-backed source, a database-backed source and a
//! source the browser owns are all later `impl` blocks, and no signature above them
//! moves. The batch shape is not invented here either — it is
//! [`CensusCsv::next_batch`](super::census_csv::CensusCsv::next_batch)'s, which
//! exists so that "the high-water mark becomes the text plus one batch".

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use super::architect::PlannedVoter;
use super::census_csv::CensusCsv;

/// How many rows a caller asks for at a time when it has no reason to prefer
/// another number. The reader's own unit of work, and the batch size
/// `plan_from_event` has always used.
pub const BATCH: usize = 5_000;

/// Where a build's voters come from.
///
/// **Re-openable, and that is not an optional nicety.** One compile reads the
/// census three times — to check it, to name the realm's user-profile attributes,
/// and to write the importable CSV — and those three happen in different modules at
/// different depths. A source that could only be read once would force them into a
/// single pass years before there is a reason to.
///
/// **`&self`, with the cursor behind interior mutability.** [`Sources`] threads
/// through `validate_plan`, `to_workbook` and `build` as a shared borrow, the way
/// `&BuildOptions` already does; a `&mut` source would make every one of those
/// signatures exclusive for the sake of a cursor.
///
/// **No `Send + Sync`.** The browser's implementation is backed by a
/// `js_sys::Function`, which is neither, and a bound here would rule it out — which
/// would mean the wizard could not own its own census, which is the whole point.
pub trait CensusSource {
    /// The columns every row carries, in the plan's own vocabulary.
    ///
    /// **Answerable before a single row is read**, and the design turns on it.
    /// `build_realm::census_attributes` declares one Keycloak user-profile
    /// attribute per census column, so this list *is* what a sign-in flow can read
    /// off a voter. Deriving it from the rows instead — the way the Voters sheet
    /// used to, as the union of every voter's `extra` keys — would mean a full pass
    /// before the realm patch could be written, and an empty census would silently
    /// declare nothing.
    fn columns(&self) -> &[String];

    /// Begin a pass at the first row.
    ///
    /// Called before each of the three reads. An implementation that cannot rewind
    /// — a socket, a one-shot stream — is not a census source; a census is a file,
    /// and files can be reopened.
    fn rewind(&self) -> Result<(), String>;

    /// The next rows of the open pass, at most `size` of them.
    ///
    /// Empty means the pass is done. Fewer than `size` does **not**: a short batch
    /// at the end of a chunk is ordinary, and a caller that stopped early on one
    /// would truncate a census without saying so.
    fn next_batch(&self, size: usize) -> Result<Vec<PlannedVoter>, String>;
}

/// Everything that travels beside a plan.
///
/// The compile-side twin of [`Beside`](super::plan_from_event::Beside), which is
/// the same idea on the way in: a plan is a description, and the bulk it describes
/// arrives *with* it rather than inside it.
///
/// `Arc` rather than owned values so this is `Clone` without copying a census or a
/// photograph — several call sites clone their options today, and one of them
/// cloning ten million rows would undo the whole exercise.
#[derive(Clone, Default)]
pub struct Sources {
    /// The voters. `None` is a plan with no census, which is most plans.
    pub census: Option<Arc<dyn CensusSource>>,
    /// The logo, the support materials and the candidates' photographs, keyed by
    /// the file's own name.
    ///
    /// By name because that is the join the workbook boundary already uses —
    /// `presentation.logo_file`, the Materials sheet's `file` column — and adding a
    /// second key would mean two answers to "which file is this". It does make the
    /// names load-bearing: two candidates whose photographs are both `photo.jpg`
    /// are indistinguishable here, which is why `check_sources` refuses it rather
    /// than letting the last one win.
    pub files: BTreeMap<String, Arc<[u8]>>,
}

impl std::fmt::Debug for Sources {
    /// Hand-written because `dyn CensusSource` is not `Debug` and should not be:
    /// deriving it would print ten million rows into a panic message.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Sources")
            .field(
                "census",
                &self.census.as_ref().map(|source| source.columns().len()),
            )
            .field("files", &self.files.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl Sources {
    /// Everything a plan still carries inline, lifted out of it.
    ///
    /// **The scaffolding, and it is meant to be temporary.** While `Blueprint` still
    /// has `voters` and the three `bytes` fields, this is what lets every caller
    /// take a `&Sources` without any of them changing what they mean — so the
    /// commit that finally deletes those fields is a small diff over a codebase
    /// that already threads sources everywhere, rather than one enormous one that
    /// does both at once.
    ///
    /// It goes away in the same change as the fields. Nothing new should call it.
    pub fn from_plan(plan: &super::architect::Blueprint) -> Self {
        let mut files: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
        let mut carry = |name: &str, bytes: &[u8]| {
            if !name.is_empty() && !bytes.is_empty() {
                files.insert(name.to_owned(), Arc::from(bytes));
            }
        };
        if let Some(logo) = &plan.logo {
            carry(&logo.file_name, &logo.bytes);
        }
        for material in &plan.materials {
            carry(&material.file_name, &material.bytes);
        }
        for election in &plan.elections {
            for contest in &election.contests {
                for candidate in &contest.candidates {
                    if let Some(image) = &candidate.image {
                        carry(&image.file_name, &image.bytes);
                    }
                }
            }
        }

        Sources {
            // `None` rather than an empty source, so "this plan has no census" and
            // "this census has no rows yet" stay distinguishable — `voters_sheet`
            // already returns `None` for the first and a header for the second.
            census: (!plan.voters.is_empty()).then(|| {
                Arc::new(VecCensus::new(plan.voters.clone()))
                    as Arc<dyn CensusSource>
            }),
            files,
        }
    }

    /// Read a census in full, for the callers that genuinely need it all at once.
    ///
    /// Provided so those callers say so out loud rather than each writing their own
    /// batch loop and quietly disagreeing about the batch size. Nothing that could
    /// stream should use it — and when a caller stops needing it, the loop it
    /// replaces is three lines, not a redesign.
    pub fn all_voters(&self) -> Result<Vec<PlannedVoter>, String> {
        let Some(census) = &self.census else {
            return Ok(Vec::new());
        };
        census.rewind()?;
        let mut all = Vec::new();
        loop {
            let batch = census.next_batch(BATCH)?;
            if batch.is_empty() {
                return Ok(all);
            }
            all.extend(batch);
        }
    }
}

/// Which column of a census row means what.
///
/// Worked out once per source rather than once per row: the previous readers each
/// called `columns.iter().position(…)` inside the row loop, which is a scan of the
/// header per field per voter. At six figures that is the measurable part.
///
/// Shared so the three doors a census can arrive through — a CSV a person dropped,
/// the CSV inside an export, a workbook's Voters sheet — agree about what a row
/// means. They were three separate mappings, and they had already drifted: only one
/// of them resolved `area_name` against the areas.
pub struct RowShape {
    columns: Vec<String>,
    username: Option<usize>,
    email: Option<usize>,
    first_name: Option<usize>,
    last_name: Option<usize>,
    /// `area.external_id`, which is what a plan and a workbook use.
    area_id: Option<usize>,
    /// `area_name`, which is what the platform's own export writes.
    area_name: Option<usize>,
    /// Everything the wizard has no field for, by position and name.
    extra: Vec<(usize, String)>,
}

/// Columns a [`PlannedVoter`] has a field of its own for, plus the ones the
/// platform regenerates and a plan must never carry.
const OWNED: &[&str] = &[
    "username",
    "email",
    "first_name",
    "last_name",
    "area_name",
    "area.external_id",
    "id",
    "enabled",
    "email_verified",
    "authorized-election-ids",
];

impl RowShape {
    pub fn of(columns: &[String]) -> Self {
        let at = |name: &str| columns.iter().position(|each| each == name);
        RowShape {
            username: at("username"),
            email: at("email"),
            first_name: at("first_name"),
            last_name: at("last_name"),
            area_id: at("area.external_id"),
            area_name: at("area_name"),
            extra: columns
                .iter()
                .enumerate()
                .filter(|(_, name)| {
                    !name.is_empty() && !OWNED.contains(&name.as_str())
                })
                .map(|(at, name)| (at, name.clone()))
                .collect(),
            columns: columns.to_vec(),
        }
    }

    /// The columns a voter built from this shape will carry.
    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    /// One row as a voter.
    ///
    /// `by_area_name` resolves the platform's `area_name` to the `external_id` a
    /// plan keys by. **A name nothing matches is kept rather than dropped**, so
    /// `check_census` can report it against the row — a voter who silently belongs
    /// nowhere is worse than one whose area is visibly wrong.
    pub fn voter(
        &self,
        row: &[String],
        by_area_name: &BTreeMap<String, String>,
    ) -> PlannedVoter {
        let pick = |index: Option<usize>| -> String {
            index
                .and_then(|at| row.get(at))
                .map(|value| value.trim().to_owned())
                .unwrap_or_default()
        };

        // The identifier wins where a row carries both. A workbook writes
        // `area.external_id`; only the platform's export writes `area_name`, and
        // `plan_from_workbook::area_of` has always preferred the identifier for the
        // same reason — a name is a label somebody edits.
        let area = match self.area_id {
            Some(_) if !pick(self.area_id).is_empty() => pick(self.area_id),
            _ => {
                let named = pick(self.area_name);
                by_area_name.get(&named).cloned().unwrap_or(named)
            }
        };

        PlannedVoter {
            username: pick(self.username),
            email: pick(self.email),
            first_name: pick(self.first_name),
            last_name: pick(self.last_name),
            area_external_id: area,
            extra: self
                .extra
                .iter()
                .filter_map(|(at, name)| {
                    let value = row.get(*at)?.trim();
                    (!value.is_empty())
                        .then(|| (name.clone(), value.to_owned()))
                })
                .collect(),
        }
    }
}

/// A census already in memory.
///
/// The compatibility shim, and the one every test wants. It is also the honest
/// shape for the small censuses that are most of them: a committee of nine does not
/// need a file handle.
pub struct VecCensus {
    columns: Vec<String>,
    voters: Vec<PlannedVoter>,
    at: std::cell::Cell<usize>,
}

impl VecCensus {
    /// The columns are the union of what the voters carry, which is what the
    /// Voters sheet's header has always been.
    pub fn new(voters: Vec<PlannedVoter>) -> Self {
        let mut columns: Vec<String> = vec![
            "username".to_owned(),
            "email".to_owned(),
            "first_name".to_owned(),
            "last_name".to_owned(),
            "area.external_id".to_owned(),
        ];
        let mut extra: Vec<String> = voters
            .iter()
            .flat_map(|voter| voter.extra.keys().cloned())
            .collect();
        extra.sort();
        extra.dedup();
        columns.extend(extra);
        VecCensus {
            columns,
            voters,
            at: std::cell::Cell::new(0),
        }
    }
}

impl CensusSource for VecCensus {
    fn columns(&self) -> &[String] {
        &self.columns
    }

    fn rewind(&self) -> Result<(), String> {
        self.at.set(0);
        Ok(())
    }

    fn next_batch(&self, size: usize) -> Result<Vec<PlannedVoter>, String> {
        let from = self.at.get();
        let to = (from + size).min(self.voters.len());
        self.at.set(to);
        Ok(self.voters[from..to].to_vec())
    }
}

/// A census read from CSV text, a batch at a time.
///
/// Wraps [`CensusCsv`], which already refuses a file with no `username` and already
/// drops the columns the platform regenerates — so the header this reports is
/// exactly the set a realm should declare.
///
/// **`rewind` re-reads from the text**, which costs a copy of it per pass. That is
/// the honest limit of phase one and the reason the trait exists: a source backed by
/// a file handle rewinds by seeking, and replacing this one changes nothing above it.
pub struct CsvCensus {
    text: String,
    shape: RowShape,
    by_area_name: BTreeMap<String, String>,
    notes: Vec<String>,
    reader: RefCell<Option<CensusCsv>>,
}

impl CsvCensus {
    /// `by_area_name` maps an area's display name to its `external_id`, for the
    /// export's `area_name` column. Pass an empty map where there is nothing to
    /// resolve against; the name is then kept as written.
    pub fn new(
        text: &str,
        by_area_name: BTreeMap<String, String>,
    ) -> Result<Self, String> {
        let reader = CensusCsv::new(text)?;
        let header = reader.header();
        Ok(CsvCensus {
            shape: RowShape::of(&header.columns),
            notes: header.notes.clone(),
            by_area_name,
            text: text.to_owned(),
            reader: RefCell::new(Some(reader)),
        })
    }

    /// What the file said about itself — the derived columns it is ignoring.
    /// Surfaced so a caller can report them; they are not errors.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }
}

impl CensusSource for CsvCensus {
    fn columns(&self) -> &[String] {
        self.shape.columns()
    }

    fn rewind(&self) -> Result<(), String> {
        *self.reader.try_borrow_mut().map_err(busy)? =
            Some(CensusCsv::new(&self.text)?);
        Ok(())
    }

    fn next_batch(&self, size: usize) -> Result<Vec<PlannedVoter>, String> {
        let mut held = self.reader.try_borrow_mut().map_err(busy)?;
        let reader = held
            .as_mut()
            .ok_or_else(|| "this census has not been opened".to_owned())?;
        Ok(reader
            .next_batch(size)?
            .iter()
            .map(|row| self.shape.voter(row, &self.by_area_name))
            .collect())
    }
}

/// A census that arrived as a workbook's Voters sheet.
///
/// Keeps the sheet rather than copying it into voters: the sheet is already in
/// memory, and a second materialisation is the thing this module exists to stop.
pub struct SheetCensus {
    sheet: Arc<super::sheet::Sheet>,
    shape: RowShape,
    by_area_name: BTreeMap<String, String>,
    at: std::cell::Cell<usize>,
}

impl SheetCensus {
    pub fn new(
        sheet: Arc<super::sheet::Sheet>,
        by_area_name: BTreeMap<String, String>,
    ) -> Self {
        let columns: Vec<String> = sheet
            .headers
            .iter()
            .filter(|header| !header.is_empty())
            .cloned()
            .collect();
        SheetCensus {
            shape: RowShape::of(&columns),
            sheet,
            by_area_name,
            at: std::cell::Cell::new(0),
        }
    }
}

impl CensusSource for SheetCensus {
    fn columns(&self) -> &[String] {
        self.shape.columns()
    }

    fn rewind(&self) -> Result<(), String> {
        self.at.set(0);
        Ok(())
    }

    fn next_batch(&self, size: usize) -> Result<Vec<PlannedVoter>, String> {
        let from = self.at.get();
        let to = (from + size).min(self.sheet.rows.len());
        self.at.set(to);
        // A sheet row is keyed by column name rather than by position, so the
        // values are lined up against the header here and the shared mapping does
        // the rest.
        Ok(self.sheet.rows[from..to]
            .iter()
            .map(|row| {
                let values: Vec<String> = self
                    .shape
                    .columns()
                    .iter()
                    .map(|column| {
                        row.get(column).map(cell_text).unwrap_or_default()
                    })
                    .collect();
                self.shape.voter(&values, &self.by_area_name)
            })
            .collect())
    }
}

/// A cell as the text a census column means.
///
/// The same rule `build.rs::value_as_text` applies to reference columns, written
/// again rather than shared because that one lives behind
/// `election_config_templates` and this module is ungated — `validate_plan` has to
/// be able to read a census in a build that cannot render a template. A number
/// typed into a username column is a username: `1001` has to match `1001` however
/// the cell was formatted.
fn cell_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(text) => text.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A source read while another pass is open.
///
/// A refusal rather than a panic, because this runs in a browser: a `RefCell`
/// panic there is an unwind through the wasm boundary and a blank page, where a
/// message is a problem somebody can act on.
fn busy(_: std::cell::BorrowMutError) -> String {
    "this census is already being read; finish one pass before starting another"
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voter(username: &str, area: &str) -> PlannedVoter {
        PlannedVoter {
            username: username.to_owned(),
            area_external_id: area.to_owned(),
            ..Default::default()
        }
    }

    /// Every impl answers the trait the same way, so a caller cannot depend on
    /// which one it was handed. Written as one body over the three because the
    /// point is that they agree.
    fn behaves(source: &dyn CensusSource, expected: &[&str]) {
        source.rewind().expect("opens");
        let first = source.next_batch(2).expect("reads");
        let rest = source.next_batch(100).expect("reads");
        let names: Vec<String> = first
            .iter()
            .chain(rest.iter())
            .map(|each| each.username.clone())
            .collect();
        assert_eq!(names, expected);

        // Empty means done, and it keeps meaning done.
        assert!(source.next_batch(10).expect("reads").is_empty());
        assert!(source.next_batch(10).expect("reads").is_empty());

        // And a second pass gives the same answer, which is what the three reads
        // of one compile depend on.
        source.rewind().expect("reopens");
        let again: Vec<String> = source
            .next_batch(100)
            .expect("reads")
            .iter()
            .map(|each| each.username.clone())
            .collect();
        assert_eq!(again, expected);
    }

    #[test]
    fn a_census_in_memory_reads_in_batches_and_rewinds() {
        let source = VecCensus::new(vec![
            voter("ada", "north"),
            voter("grace", "north"),
            voter("alan", "south"),
        ]);
        behaves(&source, &["ada", "grace", "alan"]);
        assert_eq!(
            source.columns(),
            [
                "username",
                "email",
                "first_name",
                "last_name",
                "area.external_id"
            ]
        );
    }

    #[test]
    fn a_census_from_csv_reads_in_batches_and_rewinds() {
        let source = CsvCensus::new(
            "username,area_name\nada,North\ngrace,North\nalan,South\n",
            BTreeMap::new(),
        )
        .expect("reads");
        behaves(&source, &["ada", "grace", "alan"]);
    }

    #[test]
    fn the_columns_are_known_before_any_row_is_read() {
        // The property `census_attributes` depends on: a realm's user-profile
        // attributes are named from this list, and naming them must not require a
        // pass over the census — nor produce nothing for a file with no rows yet.
        let source =
            CsvCensus::new("username,email,branch_code\n", BTreeMap::new())
                .expect("reads");

        assert_eq!(source.columns(), ["username", "email", "branch_code"]);
        assert!(source.next_batch(10).expect("reads").is_empty());
    }

    #[test]
    fn the_columns_never_include_the_ones_the_platform_regenerates() {
        // `CensusCsv` drops them and says so; this asserts the trait inherits that
        // rather than re-deriving the header from raw text. A realm that declared
        // `id` or `enabled` as a user attribute would be a realm fighting the
        // platform's own fields.
        let source = CsvCensus::new(
            "username,id,enabled,email_verified,authorized-election-ids,branch\nada,1,true,true,x,west\n",
            BTreeMap::new(),
        )
        .expect("reads");

        assert_eq!(source.columns(), ["username", "branch"]);
        assert!(!source.notes().is_empty(), "and it says why");

        source.rewind().expect("opens");
        let read = source.next_batch(10).expect("reads");
        assert_eq!(
            read[0].extra.get("branch").map(String::as_str),
            Some("west")
        );
        assert!(read[0].extra.get("id").is_none());
    }

    #[test]
    fn an_area_named_rather_than_identified_is_resolved() {
        // The platform's own export writes `area_name`; a plan keys by
        // `external_id`. One mapping, so the three doors cannot disagree about it.
        let areas: BTreeMap<String, String> =
            [("North Local".to_owned(), "north".to_owned())]
                .into_iter()
                .collect();
        let source =
            CsvCensus::new("username,area_name\nada,North Local\n", areas)
                .expect("reads");

        source.rewind().expect("opens");
        assert_eq!(
            source.next_batch(1).expect("reads")[0].area_external_id,
            "north"
        );
    }

    #[test]
    fn an_area_nothing_matches_is_kept_rather_than_dropped() {
        // So `check_census` can report it against the row. A voter who silently
        // belongs nowhere is worse than one whose area is visibly wrong.
        let source = CsvCensus::new(
            "username,area_name\nada,Nowhere\n",
            BTreeMap::new(),
        )
        .expect("reads");

        source.rewind().expect("opens");
        assert_eq!(
            source.next_batch(1).expect("reads")[0].area_external_id,
            "Nowhere"
        );
    }

    #[test]
    fn the_identifier_wins_where_a_row_carries_both() {
        // A workbook writes `area.external_id`; only the export writes
        // `area_name`. A file carrying both is one that has been through both
        // doors, and the identifier is the one that cannot have been edited into
        // disagreement.
        let areas: BTreeMap<String, String> =
            [("North".to_owned(), "wrong".to_owned())]
                .into_iter()
                .collect();
        let source = CsvCensus::new(
            "username,area_name,area.external_id\nada,North,right\n",
            areas,
        )
        .expect("reads");

        source.rewind().expect("opens");
        assert_eq!(
            source.next_batch(1).expect("reads")[0].area_external_id,
            "right"
        );
    }

    #[test]
    fn reading_a_census_twice_at_once_is_refused_rather_than_a_panic() {
        // A `RefCell` panic inside wasm unwinds through the boundary and leaves a
        // blank page. This runs in a browser, so it has to be a message.
        let source =
            CsvCensus::new("username\nada\n", BTreeMap::new()).expect("reads");
        source.rewind().expect("opens");

        let held = source.reader.borrow_mut();
        let refused = source.next_batch(1);
        drop(held);

        assert!(refused.is_err(), "a second pass was allowed to start");
    }

    #[test]
    fn a_plan_with_no_census_reads_as_no_voters_rather_than_an_error() {
        assert_eq!(Sources::default().all_voters().expect("reads"), vec![]);
    }

    #[test]
    fn reading_it_all_gives_back_every_row_in_order() {
        let sources = Sources {
            census: Some(Arc::new(VecCensus::new(vec![
                voter("ada", "north"),
                voter("grace", "north"),
            ]))),
            ..Default::default()
        };
        let all = sources.all_voters().expect("reads");
        assert_eq!(
            all.iter().map(|each| &each.username).collect::<Vec<_>>(),
            ["ada", "grace"]
        );
    }
}

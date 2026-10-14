// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Build an importable election event from an authoring workbook.
//!
//! This is janitor, folded into `step-cli`. Every decision it makes lives in
//! `sequent_core::election_config`, which is also what windmill validates with and
//! what the browser-side tools will run — so a bundle that builds here is a bundle
//! the platform accepts, and there is no second implementation to drift.
//!
//! All this file does is talk to the filesystem and to whoever ran it. Reading a
//! workbook, building, validating and laying out the files are all pure functions
//! in the core, which is why nothing is written until every one of them has
//! succeeded: a workbook with a dangling reference leaves no half-written output
//! directory behind.

use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use sequent_core::election_config::render::ENTITY_TEMPLATES;
use sequent_core::election_config::sources::Sources;
use sequent_core::election_config::xlsx::read_xlsx;
use sequent_core::election_config::{
    archive, build, presets, validate, BuildOptions, Bundle, ImportElectionEventSchema, Problem,
    Severity, TemplateSet, ValidationReport,
};
use sequent_core::types::ceremonies::CeremoniesPolicy;
use std::fs;
use std::path::{Component, Path, PathBuf};

/// Build an importable election event from an `.xlsx` workbook
#[derive(Args)]
#[command(about)]
pub struct BuildElectionEvent {
    /// Path to the authoring workbook
    #[arg(short = 'w', long, value_name = "WORKBOOK")]
    workbook: PathBuf,

    /// Directory to write the bundle into, under a subdirectory named after the
    /// event
    #[arg(short = 'o', long, value_name = "OUT", default_value = "out")]
    out: PathBuf,

    /// Tenant id to write into the file
    ///
    /// Import remaps it onto the tenant of the importing request, so getting it
    /// wrong cannot send an event to the wrong tenant. It does name the
    /// `export_permissions-<tenant>.csv` file, which nothing rewrites.
    #[arg(short = 't', long, value_name = "TENANT_ID")]
    tenant_id: Option<String>,

    /// An existing export (`.json` or `.zip`) to inherit platform defaults and a
    /// Keycloak realm from
    ///
    /// Without one no realm is emitted, and the platform loads its own provisioned
    /// default instead.
    #[arg(short = 'b', long, value_name = "BASE_EXPORT")]
    base_export: Option<PathBuf>,

    /// Directory of `.hbs` files overriding the built-in entity templates
    #[arg(long, value_name = "TEMPLATES_DIR")]
    templates_dir: Option<PathBuf>,

    /// Authentication preset, overriding the workbook's `auth_type`
    ///
    /// `none` leaves the realm alone whatever the workbook declares.
    #[arg(long, value_name = "PRESET")]
    auth_preset: Option<String>,

    /// Name for the output directory and archive
    #[arg(long, value_name = "SLUG")]
    slug: Option<String>,

    /// Refuse to write anything if there are warnings
    ///
    /// Worth using in CI: a warning here means the bundle imports and the
    /// configuration probably is not what its author meant.
    #[arg(long)]
    strict: bool,

    /// Validate and report, writing nothing
    ///
    /// Named after the importer's own `check_only`, which does the same thing on
    /// the server.
    #[arg(long)]
    check_only: bool,

    /// Timestamp for every generated entity
    ///
    /// Fixed by default so regenerating an unchanged workbook produces
    /// byte-identical output. The importer overwrites these, so this only exists
    /// to make a rebuild diffable.
    #[arg(long, value_name = "CREATED_AT")]
    created_at: Option<String>,
}

impl BuildElectionEvent {
    pub fn run(&self) {
        match self.build() {
            Ok(()) => (),
            Err(error) => {
                eprintln!("{} {error:#}", "error:".red().bold());
                std::process::exit(1);
            }
        }
    }

    fn build(&self) -> Result<()> {
        if let Some(preset) = &self.auth_preset {
            let known =
                preset.eq_ignore_ascii_case(presets::NONE) || presets::get(preset).is_some();
            if !known {
                return Err(anyhow!(
                    "'{preset}' is not an authentication preset. Expected {} or {}.",
                    presets::NONE,
                    presets::names().join(", ")
                ));
            }
        }

        let bytes = fs::read(&self.workbook)
            .with_context(|| format!("could not read {}", self.workbook.display()))?;
        let workbook = read_xlsx(&bytes).map_err(problem_error)?;

        for unread in workbook.unread_sheets() {
            // A misspelled tab is silently ignored otherwise, and the missing
            // entities look like an authoring mistake somewhere else entirely.
            println!(
                "{} sheet '{unread}' is not one this reads, and was ignored",
                "note:".cyan()
            );
        }

        let templates = self.templates()?;
        for overridden in templates.overridden() {
            println!("{} using your own '{overridden}' template", "note:".cyan());
        }

        let options = BuildOptions {
            tenant_id: self.tenant_id.clone(),
            base_export: self.base_export()?,
            slug: self.slug.clone(),
            created_at: self.created_at.clone(),
            auth_preset: self.auth_preset.clone(),
            // Both empty, and both deliberately spelled out rather than
            // `..Default::default()`. A workbook cell cannot hold bytes, so the
            // workbook path has no photographs to offer; and nothing here builds a
            // key ceremony, which is `compile-plan`'s job because only a plan
            // carries the trustee list. Naming them keeps the compiler as the thing
            // that notices the next field — which is exactly what caught these two.
            images: Vec::new(),
            // A workbook path has no bytes to offer for either. The Materials
            // sheet names files that a caller supplies; until it does, empty.
            materials: Vec::new(),
            keys_ceremony: None,
            // No ceremony from a workbook, so this labels nothing; the platform's
            // own default, named rather than defaulted for the reason above.
            ceremony_policy: CeremoniesPolicy::MANUAL_CEREMONIES,
        };

        let bundle = match build(&workbook, &templates, &options, &Sources::default()) {
            Ok(bundle) => bundle,
            Err(report) => {
                report_problems(&report);
                return Err(anyhow!(
                    "{} problem(s) in {}",
                    report.errors().count(),
                    self.workbook.display()
                ));
            }
        };

        // The same validation windmill runs before importing, on the bundle as
        // written. A workbook can be internally consistent and still describe an
        // event the platform would reject.
        let schema: ImportElectionEventSchema = serde_json::from_value(bundle.export.clone())
            .context(
                "the built bundle does not match the import schema, which is a \
                 bug in this tool rather than in the workbook",
            )?;
        let checked = validate(&schema);

        report_problems(&bundle.warnings);
        report_problems(&checked);

        if checked.has_errors() {
            return Err(anyhow!(
                "{} problem(s) in the bundle this workbook describes",
                checked.errors().count()
            ));
        }

        let warnings = bundle.warnings.warnings().count() + checked.warnings().count();
        if self.strict && warnings > 0 {
            return Err(anyhow!("{warnings} warning(s), and --strict was given"));
        }

        if self.check_only {
            println!(
                "{} {} would build, with {warnings} warning(s)",
                "ok:".green().bold(),
                bundle.event_external_id
            );
            return Ok(());
        }

        self.write(&bundle)?;
        Ok(())
    }

    /// The built-in templates, with any `.hbs` file in `--templates-dir` on top.
    fn templates(&self) -> Result<TemplateSet> {
        let Some(directory) = &self.templates_dir else {
            return TemplateSet::builtin().map_err(problem_error);
        };
        if !directory.is_dir() {
            return Err(anyhow!(
                "templates directory not found: {}",
                directory.display()
            ));
        }

        let mut sources: Vec<(String, String)> = Vec::new();
        for name in ENTITY_TEMPLATES {
            let candidate = directory.join(format!("{name}.hbs"));
            if candidate.is_file() {
                let source = fs::read_to_string(&candidate)
                    .with_context(|| format!("could not read {}", candidate.display()))?;
                sources.push(((*name).to_string(), source));
            }
        }

        // Anything else in the directory is a name nothing renders, which is
        // almost always a typo. Saying so beats rendering the built-in and
        // leaving its author staring at output that ignores their edit.
        for entry in fs::read_dir(directory)
            .with_context(|| format!("could not list {}", directory.display()))?
        {
            let path = entry?.path();
            let is_template = path.extension().is_some_and(|extension| extension == "hbs");
            let is_known = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .is_some_and(|stem| ENTITY_TEMPLATES.contains(&stem));
            if is_template && !is_known {
                println!(
                    "{} {} is not an entity template and was ignored. Expected \
                     one of: {}.",
                    "warning:".yellow(),
                    path.display(),
                    ENTITY_TEMPLATES.join(", ")
                );
            }
        }

        let borrowed: Vec<(&str, &str)> = sources
            .iter()
            .map(|(name, source)| (name.as_str(), source.as_str()))
            .collect();
        TemplateSet::with_overrides(&borrowed).map_err(problem_error)
    }

    /// Read a base export from a `.json` file or an export `.zip`.
    fn base_export(&self) -> Result<Option<serde_json::Value>> {
        let Some(path) = &self.base_export else {
            return Ok(None);
        };
        let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;

        let is_zip = path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"));
        if !is_zip {
            return serde_json::from_slice(&bytes)
                .with_context(|| format!("{} is not valid JSON", path.display()))
                .map(Some);
        }

        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes))
            .with_context(|| format!("{} is not a zip", path.display()))?;
        let member = (0..zip.len())
            .map(|index| zip.by_index(index).map(|file| file.name().to_string()))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .find(|name| {
                let base = name.rsplit('/').next().unwrap_or(name);
                base.starts_with("export_election_event") && base.ends_with(".json")
            })
            .ok_or_else(|| {
                anyhow!(
                    "{} has no export_election_event*.json member; is it an \
                     election event export?",
                    path.display()
                )
            })?;

        let file = zip.by_name(&member)?;
        serde_json::from_reader(file)
            .with_context(|| format!("{member} is not valid JSON"))
            .map(Some)
    }

    /// Write the bundle, the archive, and everything that travels beside it.
    fn write(&self, bundle: &Bundle) -> Result<()> {
        let layout = archive::layout(bundle);
        let directory = self.out.join(&bundle.slug);

        // Replaced rather than merged into: a stale member left over from a
        // previous run is a file someone would upload.
        if directory.exists() {
            fs::remove_dir_all(&directory)
                .with_context(|| format!("could not clear {}", directory.display()))?;
        }
        fs::create_dir_all(&directory)
            .with_context(|| format!("could not create {}", directory.display()))?;

        for artifact in layout.importable.iter().chain(layout.auxiliary.iter()) {
            write_artifact(&directory, &artifact.name, &artifact.bytes)?;
        }

        let archive_bytes = archive::zip(&layout.importable).map_err(problem_error)?;
        let archive_path = directory.join(&layout.archive_name);
        fs::write(&archive_path, &archive_bytes)
            .with_context(|| format!("could not write {}", archive_path.display()))?;

        println!(
            "{} {}",
            "built".green().bold(),
            bundle.event_external_id.bold()
        );
        println!(
            "  import this: {}",
            archive_path.display().to_string().bold()
        );
        for artifact in &layout.importable {
            println!("    {}", artifact.name);
        }
        if !layout.auxiliary.is_empty() {
            println!("  beside it, not part of the import:");
            for artifact in &layout.auxiliary {
                println!("    {}", directory.join(&artifact.name).display());
            }
        }
        if bundle.admin_users.is_some() {
            println!(
                "  {} admin_users.csv may carry clear-text passwords. It is a \
                 secret, not a deliverable.",
                "warning:".yellow()
            );
        }
        Ok(())
    }
}

/// Write one artifact of a layout under `directory`, creating its parents.
///
/// The name is checked first. Most of a layout's names are fixed, but an image's
/// and a support material's end in the file name the plan or the workbook gave
/// it, so `../../somewhere` there would otherwise be written outside the output
/// directory — and an absolute name would replace `directory` altogether. Only
/// plain relative components are accepted.
///
/// Shared with `compile-plan`, whose layouts have the same nested members.
pub(crate) fn write_artifact(directory: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    let relative = Path::new(name);
    if name.is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(anyhow!(
            "refusing to write the artifact {name:?}: it is not a plain relative path \
             inside the output directory"
        ));
    }
    let path = directory.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    fs::write(&path, bytes).with_context(|| format!("could not write {}", path.display()))
}

/// Print every problem, errors first, in the order they were found.
fn report_problems(report: &ValidationReport) {
    for problem in report.errors() {
        eprintln!(
            "{} {}: {}",
            "error:".red().bold(),
            problem.path,
            problem.message
        );
    }
    for problem in report.warnings() {
        eprintln!(
            "{} {}: {}",
            "warning:".yellow().bold(),
            problem.path,
            problem.message
        );
    }
}

/// A single problem as an error, for the paths that fail before there is a report.
fn problem_error(problem: Problem) -> anyhow::Error {
    let label = match problem.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
    };
    anyhow!("{label}: {}: {}", problem.path, problem.message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sequent_core::election_config::paths::Cell;
    use sequent_core::election_config::render::BUILTIN_TEMPLATES;
    use sequent_core::election_config::xlsx_write::write_xlsx;
    use sequent_core::election_config::{Sheet, Workbook};
    use std::io::Write;

    const TENANT: &str = "90505c8a-23a9-4cdf-a26b-4e19f6a097d5";

    fn text(value: &str) -> Cell {
        Cell::text(value)
    }

    /// One election, one contest with two candidates, two areas: the smallest
    /// workbook that builds cleanly, plus any extra sheets a test adds.
    fn workbook(extra: Vec<(&str, Vec<Vec<Cell>>)>) -> Vec<u8> {
        let mut sheets = vec![
            (
                "ElectionEvent",
                vec![
                    vec![text("external_id"), text("presentation.i18n.en.name")],
                    vec![text("union-2027"), text("Union Election 2027")],
                ],
            ),
            (
                "Elections",
                vec![
                    vec![text("external_id"), text("presentation.i18n.en.name")],
                    vec![text("statewide"), text("Statewide Officers")],
                ],
            ),
            (
                "Contests",
                vec![
                    vec![
                        text("external_id"),
                        text("election.external_id"),
                        text("presentation.i18n.en.name"),
                        text("max_votes"),
                    ],
                    vec![
                        text("president"),
                        text("statewide"),
                        text("President"),
                        Cell::Int(1),
                    ],
                ],
            ),
            (
                "Candidates",
                vec![
                    vec![
                        text("external_id"),
                        text("contest.external_id"),
                        text("presentation.i18n.en.name"),
                    ],
                    vec![text("alice"), text("president"), text("Alice")],
                    vec![text("bob"), text("president"), text("Bob")],
                ],
            ),
            (
                "Areas",
                vec![
                    vec![text("external_id"), text("name")],
                    vec![text("area-north"), text("North")],
                    vec![text("area-south"), text("South")],
                ],
            ),
            (
                "AreaContests",
                vec![
                    vec![text("area.external_id"), text("contest.external_id")],
                    vec![text("area-north"), text("president")],
                    vec![text("area-south"), text("president")],
                ],
            ),
        ];
        for (name, grid) in extra {
            sheets.retain(|(existing, _)| *existing != name);
            sheets.push((name, grid));
        }
        let workbook = Workbook::new(
            sheets
                .into_iter()
                .map(|(name, grid)| Sheet::from_grid(name, &grid).unwrap())
                .collect(),
        )
        .unwrap();
        write_xlsx(&workbook).unwrap()
    }

    /// A blank parameter builds, with a warning.
    fn with_warning() -> Vec<u8> {
        workbook(vec![(
            "Parameters",
            vec![
                vec![text("type"), text("key"), text("value")],
                vec![text("settings"), text("saml_idp_metadata_url"), Cell::Blank],
            ],
        )])
    }

    struct Fixture {
        root: tempfile::TempDir,
        command: BuildElectionEvent,
    }

    impl Fixture {
        fn new(bytes: &[u8]) -> Self {
            let root = tempfile::tempdir().unwrap();
            let path = root.path().join("event.xlsx");
            fs::write(&path, bytes).unwrap();
            let command = BuildElectionEvent {
                workbook: path,
                out: root.path().join("out"),
                tenant_id: Some(TENANT.to_string()),
                base_export: None,
                templates_dir: None,
                auth_preset: None,
                slug: Some("event".to_string()),
                strict: false,
                check_only: false,
                created_at: None,
            };
            Fixture { root, command }
        }

        fn directory(&self) -> PathBuf {
            self.root.path().join("out").join("event")
        }

        fn error(&self) -> String {
            format!("{:#}", self.command.build().expect_err("the build fails"))
        }

        fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.root.path().join(name);
            fs::write(&path, bytes).unwrap();
            path
        }
    }

    fn zip_of(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut cursor);
            for (name, bytes) in members {
                zip.start_file(*name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                zip.write_all(bytes).unwrap();
            }
            zip.finish().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn a_sound_workbook_writes_the_archive_and_replaces_a_stale_directory() {
        let fixture = Fixture::new(&workbook(vec![]));
        let stale = fixture.directory().join("stale.json");
        fs::create_dir_all(fixture.directory()).unwrap();
        fs::write(&stale, b"left over").unwrap();

        fixture.command.build().unwrap();

        assert!(!stale.exists(), "a stale member would be uploaded");
        let archive = fs::read(fixture.directory().join("event.zip")).unwrap();
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(archive)).unwrap();
        let names: Vec<String> = zip.file_names().map(str::to_string).collect();
        let export = names
            .iter()
            .find(|name| name.starts_with("export_election_event"))
            .unwrap_or_else(|| panic!("{names:?}"));
        let document: serde_json::Value =
            serde_json::from_reader(zip.by_name(export).unwrap()).unwrap();
        assert_eq!(document["tenant_id"], TENANT);
        // Every importable member is also written loose beside the archive.
        for name in &names {
            assert!(fixture.directory().join(name).is_file(), "{name}");
        }
    }

    #[test]
    fn check_only_validates_and_writes_nothing() {
        let mut fixture = Fixture::new(&workbook(vec![]));
        fixture.command.check_only = true;

        fixture.command.build().unwrap();

        assert!(!fixture.root.path().join("out").exists());
    }

    #[test]
    fn strict_refuses_a_workbook_with_warnings_before_writing() {
        let mut fixture = Fixture::new(&with_warning());
        fixture.command.build().unwrap();
        fs::remove_dir_all(fixture.root.path().join("out")).unwrap();

        fixture.command.strict = true;
        let error = fixture.error();

        assert!(error.contains("--strict"), "{error}");
        assert!(!fixture.root.path().join("out").exists());
    }

    #[test]
    fn a_dangling_reference_is_reported_and_leaves_no_output() {
        let fixture = Fixture::new(&workbook(vec![(
            "Candidates",
            vec![
                vec![
                    text("external_id"),
                    text("contest.external_id"),
                    text("presentation.i18n.en.name"),
                ],
                vec![text("alice"), text("no-such-contest"), text("Alice")],
            ],
        )]));

        let error = fixture.error();

        assert!(error.contains("problem(s) in"), "{error}");
        assert!(error.contains("event.xlsx"), "{error}");
        assert!(!fixture.root.path().join("out").exists());
    }

    #[test]
    fn an_unknown_auth_preset_is_refused_before_the_workbook_is_read() {
        let mut fixture = Fixture::new(b"never read");
        fixture.command.workbook = fixture.root.path().join("missing.xlsx");
        fixture.command.auth_preset = Some("carrier-pigeon".to_string());

        let error = fixture.error();

        assert!(error.contains("'carrier-pigeon' is not an authentication preset"));
        assert!(error.contains(presets::NONE), "{error}");
    }

    #[test]
    fn the_none_preset_is_accepted_in_any_case() {
        let mut fixture = Fixture::new(&workbook(vec![]));
        fixture.command.auth_preset = Some(presets::NONE.to_uppercase());
        fixture.command.check_only = true;

        fixture.command.build().unwrap();
    }

    #[test]
    fn an_unreadable_or_invalid_workbook_is_an_error() {
        let mut fixture = Fixture::new(b"not a spreadsheet");
        let error = fixture.error();
        assert!(error.starts_with("error: "), "{error}");

        fixture.command.workbook = fixture.root.path().join("missing.xlsx");
        let error = fixture.error();
        assert!(error.contains("could not read"), "{error}");
    }

    #[test]
    fn templates_come_from_the_directory_and_a_missing_one_is_refused() {
        let mut fixture = Fixture::new(&workbook(vec![]));
        fixture.command.check_only = true;
        fixture.command.templates_dir = Some(fixture.root.path().join("absent"));
        let error = fixture.error();
        assert!(error.contains("templates directory not found"), "{error}");

        // A known name overrides the builtin; any other .hbs or file is ignored.
        let directory = fixture.root.path().join("templates");
        fs::create_dir(&directory).unwrap();
        let (_, area) = BUILTIN_TEMPLATES
            .iter()
            .find(|(name, _)| *name == "area")
            .unwrap();
        fs::write(directory.join("area.hbs"), area).unwrap();
        fs::write(directory.join("aera.hbs"), "{{typo}}").unwrap();
        fs::write(directory.join("README.md"), "notes").unwrap();
        fixture.command.templates_dir = Some(directory.clone());
        let templates = fixture.command.templates().unwrap();
        assert_eq!(templates.overridden(), ["area"]);
        fixture.command.build().unwrap();

        // A template that does not compile is reported rather than skipped.
        fs::write(directory.join("area.hbs"), "{{#if}}").unwrap();
        let error = fixture.error();
        assert!(error.contains("area"), "{error}");
    }

    #[test]
    fn a_base_export_is_read_from_json_or_from_an_export_zip() {
        let fixture = Fixture::new(&workbook(vec![]));
        fixture.command.build().unwrap();
        let export_name = fs::read_dir(fixture.directory())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .find(|name| name.starts_with("export_election_event"))
            .unwrap();
        let export = fs::read(fixture.directory().join(&export_name)).unwrap();
        let expected: serde_json::Value = serde_json::from_slice(&export).unwrap();

        let mut command = Fixture::new(&workbook(vec![]));
        command.command.base_export = Some(command.file("base.json", &export));
        assert_eq!(
            command.command.base_export().unwrap(),
            Some(expected.clone())
        );

        // Inside a zip it is found by name, even under a directory, and an
        // upper-case extension is still a zip.
        let nested = format!("export/{export_name}");
        let zip = zip_of(&[("README.txt", b"hello"), (&nested, &export)]);
        command.command.base_export = Some(command.file("base.ZIP", &zip));
        assert_eq!(command.command.base_export().unwrap(), Some(expected));

        // And a build on top of it succeeds.
        command.command.check_only = true;
        command.command.build().unwrap();
    }

    #[test]
    fn a_base_export_that_is_not_one_is_refused() {
        let mut fixture = Fixture::new(&workbook(vec![]));
        let cases: Vec<(&str, Vec<u8>, &str)> = vec![
            ("base.json", b"{not json".to_vec(), "is not valid JSON"),
            ("base.zip", b"not a zip".to_vec(), "is not a zip"),
            (
                "other.zip",
                zip_of(&[("voters.csv", b"a,b")]),
                "has no export_election_event",
            ),
            (
                "broken.zip",
                zip_of(&[("export_election_event-1.json", b"{")]),
                "export_election_event-1.json is not valid JSON",
            ),
        ];
        for (name, bytes, expected) in cases {
            fixture.command.base_export = Some(fixture.file(name, &bytes));
            let error = format!("{:#}", fixture.command.base_export().unwrap_err());
            assert!(error.contains(expected), "{name}: {error}");
        }

        fixture.command.base_export = Some(fixture.root.path().join("missing.json"));
        let error = fixture.error();
        assert!(error.contains("could not read"), "{error}");
    }

    #[test]
    fn a_problem_error_carries_its_severity_path_and_message() {
        use sequent_core::election_config::Code;
        let error = problem_error(Problem::error(Code::InvalidValue, "a.b", "broken"));
        assert_eq!(error.to_string(), "error: a.b: broken");
        let warning = problem_error(Problem::warning(Code::InvalidValue, "c", "odd"));
        assert_eq!(warning.to_string(), "warning: c: odd");
    }

    /// A nested member gets its directory.
    #[test]
    fn a_nested_artifact_is_written_with_its_directory() {
        let out = tempfile::tempdir().unwrap();

        write_artifact(out.path(), "images/document_1_photo.png", b"png").unwrap();

        assert_eq!(
            fs::read(out.path().join("images/document_1_photo.png")).unwrap(),
            b"png"
        );
    }

    /// A name that leaves the output directory is refused, and nothing is written.
    ///
    /// An image's or a material's entry ends in a file name the plan chose, so
    /// `export_S3_files/document_<id>_../../../x` climbed out of the directory.
    #[test]
    fn a_name_that_leaves_the_output_directory_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let out = root.path().join("out");
        fs::create_dir(&out).unwrap();
        let outside = root.path().join("escaped");

        for name in [
            "export_S3_files/document_1_../../../escaped",
            "../escaped",
            outside.to_str().unwrap(),
            "",
        ] {
            let error = write_artifact(&out, name, b"x").expect_err(name);
            assert!(error.to_string().contains("refusing"), "{error}");
        }
        assert!(!outside.exists());
    }
}

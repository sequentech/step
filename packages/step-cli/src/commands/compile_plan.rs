// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Build an importable election event from an Election Architect plan.
//!
//! The sibling of `build-election-event`, which does the same thing from a
//! spreadsheet. Both call `sequent_core::election_config`, so a plan and a
//! workbook describing the same event produce the same bundle — and this command
//! is how that gets checked without a browser.
//!
//! It also writes the ballot preview, which is what makes a plan reviewable
//! outside the wizard: `ballot-preview.json` is the document the Voting Portal's
//! own preview route opens, so a delivery engineer can see the ballot in the
//! client's own portal at the client's own version.
//!
//! Everything here is filesystem and terminal. The decisions are all in the core.

use super::build_election_event::write_artifact;
use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use sequent_core::election_config::architect::{compile_plan, Compile};
use sequent_core::election_config::open::open_named;
use sequent_core::election_config::preview::{preview_publication, PreviewOptions};
use sequent_core::election_config::profile::{ClientProfile, Profile};
use sequent_core::election_config::{
    archive, BuildOptions, Problem, Severity, TemplateSet, ValidationReport,
};
use sequent_core::types::ceremonies::CeremoniesPolicy;
use std::fs;
use std::path::PathBuf;

/// Build an importable election event from an Election Architect plan
#[derive(Args)]
#[command(about)]
pub struct CompilePlan {
    /// Path to `blueprint.json`, as the wizard saves it
    #[arg(short = 'p', long, value_name = "PLAN")]
    plan: PathBuf,

    /// Directory to write the bundle into, under a subdirectory named after the
    /// event
    #[arg(short = 'o', long, value_name = "OUT", default_value = "out")]
    out: PathBuf,

    /// A client profile to apply first
    ///
    /// The same document the wizard loads from `?profile=<id>`. Applied before
    /// validation, so a locked value is the one that gets checked and the one
    /// that gets built.
    #[arg(long, value_name = "PROFILE")]
    profile: Option<PathBuf>,

    /// Tenant id to write into the file
    #[arg(short = 't', long, value_name = "TENANT_ID")]
    tenant_id: Option<String>,

    /// An existing export (`.json`) to inherit platform defaults and a Keycloak
    /// realm from
    #[arg(short = 'b', long, value_name = "BASE_EXPORT")]
    base_export: Option<PathBuf>,

    /// Name for the output directory and archive
    #[arg(long, value_name = "SLUG")]
    slug: Option<String>,

    /// Timestamp for every generated entity
    ///
    /// Fixed by default so recompiling an unchanged plan produces byte-identical
    /// output.
    #[arg(long, value_name = "CREATED_AT")]
    created_at: Option<String>,

    /// Refuse to write anything if there are warnings
    #[arg(long)]
    strict: bool,

    /// Validate and report, writing nothing
    #[arg(long)]
    check_only: bool,

    /// Also write `ballot-preview.json`
    ///
    /// The document the Voting Portal's preview route opens, holding one ballot
    /// per area and election exactly as a publication would generate them. The
    /// key it carries is a stand-in flagged `is_demo`; nothing can be cast
    /// against it.
    #[arg(long)]
    preview: bool,
}

impl CompilePlan {
    pub fn run(&self) {
        if let Err(error) = self.compile() {
            eprintln!("{} {error:#}", "error:".red().bold());
            std::process::exit(1);
        }
    }

    fn compile(&self) -> Result<()> {
        let bytes = fs::read(&self.plan)
            .with_context(|| format!("could not read {}", self.plan.display()))?;
        // Through the core's own door, the one the wizard opens files with. A bare
        // plan goes through `read_plan`, so an older one is migrated exactly as the
        // wizard migrates it — `serde_json::from_str` here read a version 2 plan as
        // though it were current. And what travelled beside the plan comes back
        // with it: a version 3 plan's members, which the migration lifts out of the
        // document, and the census and files of a save-file zip or a delivery.
        // Keeping only the plan compiled all of those with nobody in the census.
        let opened = open_named(&bytes, self.plan.file_name().and_then(|name| name.to_str()))
            .map_err(|report| {
                report_problems(&report);
                anyhow!("{} is not an election plan", self.plan.display())
            })?;
        report_problems(&opened.report);
        let plan = opened.plan;

        let profile = self.profile()?;
        let templates = TemplateSet::builtin().map_err(problem_error)?;
        let options = BuildOptions {
            tenant_id: self.tenant_id.clone(),
            base_export: self.base_export()?,
            slug: self.slug.clone(),
            created_at: self.created_at.clone(),
            auth_preset: None,
            // Set from the plan by `compile_plan` itself — the trustees become the
            // ceremony and the candidates' photographs travel as files — so whatever
            // is passed here is replaced. Spelled out anyway, so the next field added
            // to `BuildOptions` still stops the build rather than defaulting quietly.
            images: Vec::new(),
            // Set from the plan by `compile_plan` itself, like `images` — the
            // support materials' bytes live in the plan, not in these options.
            materials: Vec::new(),
            keys_ceremony: None,
            // Replaced by `compile_plan` from the plan's own `ceremony_policy`,
            // like the two above.
            ceremony_policy: CeremoniesPolicy::MANUAL_CEREMONIES,
        };

        let compiled = match compile_plan(Compile {
            plan: &plan,
            templates: &templates,
            options: &options,
            profile: profile.as_ref(),
            sources: Some(&opened.sources),
        }) {
            Ok(compiled) => compiled,
            Err(report) => {
                report_problems(&report);
                return Err(anyhow!(
                    "{} problem(s) in {}",
                    report.errors().count(),
                    self.plan.display()
                ));
            }
        };

        report_problems(&compiled.report);
        let warnings = opened.report.warnings().count() + compiled.report.warnings().count();
        if self.strict && warnings > 0 {
            return Err(anyhow!("{warnings} warning(s), and --strict was given"));
        }

        // Built before anything is written, so a plan whose ballots cannot be
        // generated fails before leaving files behind — the same reason
        // `build-election-event` validates before it writes.
        let preview = if self.preview {
            Some(
                preview_publication(&compiled.bundle, &PreviewOptions::default()).map_err(
                    |report| {
                        report_problems(&report);
                        anyhow!("the plan compiles but its ballots do not")
                    },
                )?,
            )
        } else {
            None
        };

        if self.check_only {
            println!(
                "{} {} would build, with {warnings} warning(s)",
                "ok:".green().bold(),
                compiled.bundle.event_external_id
            );
            return Ok(());
        }

        let directory = self.out.join(&compiled.bundle.slug);
        if directory.exists() {
            fs::remove_dir_all(&directory)
                .with_context(|| format!("could not clear {}", directory.display()))?;
        }
        fs::create_dir_all(&directory)
            .with_context(|| format!("could not create {}", directory.display()))?;

        for artifact in compiled
            .layout
            .importable
            .iter()
            .chain(compiled.layout.auxiliary.iter())
        {
            // Through the shared writer, which creates `images/`,
            // `export_S3_files/` and `templates/` and refuses a name that would
            // leave the directory.
            write_artifact(&directory, &artifact.name, &artifact.bytes)?;
        }

        let archive_bytes = archive::zip(&compiled.layout.importable).map_err(problem_error)?;
        let archive_path = directory.join(&compiled.layout.archive_name);
        fs::write(&archive_path, &archive_bytes)
            .with_context(|| format!("could not write {}", archive_path.display()))?;

        println!(
            "{} {}",
            "built".green().bold(),
            compiled.bundle.event_external_id.bold()
        );
        println!(
            "  import this: {}",
            archive_path.display().to_string().bold()
        );

        if let Some(preview) = preview {
            let path = directory.join("ballot-preview.json");
            // Through `to_document`, which sorts every key: a preview is a file
            // people diff against last week's.
            let text = serde_json::to_string_pretty(&preview.to_document())?;
            fs::write(&path, format!("{text}\n"))
                .with_context(|| format!("could not write {}", path.display()))?;
            println!(
                "  {} ballot(s) to preview: {}",
                preview.ballot_styles.len(),
                path.display().to_string().bold()
            );
            println!(
                "    open it in a Voting Portal at /preview/file. The key it \
                 carries is a stand-in — nothing can be cast against it."
            );
        }

        Ok(())
    }

    fn profile(&self) -> Result<Option<Profile>> {
        let Some(path) = &self.profile else {
            return Ok(None);
        };
        let source = fs::read_to_string(path)
            .with_context(|| format!("could not read {}", path.display()))?;
        let document: ClientProfile = serde_json::from_str(&source)
            .with_context(|| format!("{} is not a client profile", path.display()))?;
        Profile::read(&document)
            .map(Some)
            .map_err(|report| anyhow!("{report}"))
    }

    fn base_export(&self) -> Result<Option<serde_json::Value>> {
        let Some(path) = &self.base_export else {
            return Ok(None);
        };
        let bytes = fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
        serde_json::from_slice(&bytes)
            .with_context(|| format!("{} is not valid JSON", path.display()))
            .map(Some)
    }
}

fn report_problems(report: &ValidationReport) {
    for problem in &report.problems {
        let label = match problem.severity {
            Severity::Error => "error:".red().bold(),
            Severity::Warning => "warning:".yellow().bold(),
        };
        println!("{label} {} — {}", problem.path.bold(), problem.message);
    }
}

fn problem_error(problem: Problem) -> anyhow::Error {
    anyhow!("{} — {}", problem.path, problem.message)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A plan somebody could have saved: one election, one contest, two areas.
    fn plan(version: u32) -> serde_json::Value {
        serde_json::json!({
            "version": version,
            "external_id": "union-2027",
            "name": {"en": "Union Election 2027"},
            "languages": ["en"],
            "trustees": [
                {"name": "A", "email": "a@example.org"},
                {"name": "B", "email": "b@example.org"},
                {"name": "C", "email": "c@example.org"}
            ],
            "trustee_threshold": 2,
            "areas": [{"external_id": "north", "name": "North Local 1"}],
            "elections": [{
                "external_id": "officers",
                "name": {"en": "Officers"},
                "contests": [{
                    "external_id": "president",
                    "name": {"en": "President"},
                    "max_votes": 1,
                    "winners": 1,
                    "candidates": [
                        {"external_id": "alice", "name": {"en": "Alice"}},
                        {"external_id": "bob", "name": {"en": "Bob"}}
                    ]
                }]
            }]
        })
    }

    fn compile(document: &serde_json::Value) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("blueprint.json");
        fs::write(&path, serde_json::to_vec(document).unwrap()).unwrap();
        let command = CompilePlan {
            plan: path,
            out: root.path().join("out"),
            profile: None,
            tenant_id: None,
            base_export: None,
            slug: Some("event".to_string()),
            created_at: None,
            strict: false,
            check_only: false,
            preview: false,
        };
        command.compile().expect("the plan compiles");
        let directory = root.path().join("out").join("event");
        (root, directory)
    }

    fn written(directory: &std::path::Path) -> Vec<String> {
        let mut names = Vec::new();
        let mut stack = vec![directory.to_path_buf()];
        while let Some(next) = stack.pop() {
            for entry in fs::read_dir(next).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    names.push(
                        path.strip_prefix(directory)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                    );
                }
            }
        }
        names
    }

    /// A version 3 plan's members reach the census it compiles.
    ///
    /// The migration lifts them out of the document into the sources `read_plan`
    /// returns, and this command kept only the plan, so the event compiled with
    /// nobody in it and said nothing.
    #[test]
    fn a_version_three_plan_compiles_with_its_members() {
        let mut document = plan(3);
        document["voters"] = serde_json::json!([
            {"username": "ada", "email": "ada@example.org",
             "area_external_id": "north"}
        ]);

        let (_root, directory) = compile(&document);

        let census = written(&directory)
            .into_iter()
            .find(|name| name.contains("export_voters") && name.ends_with(".csv"))
            .expect("a census member");
        let text = fs::read_to_string(directory.join(census)).unwrap();
        assert!(text.contains("ada@example.org"), "{text}");
    }

    /// A support material is written under `export_S3_files/`, whose directory
    /// nothing else creates.
    #[test]
    fn a_plan_with_a_support_material_writes_its_file() {
        let mut document = plan(sequent_core::election_config::architect::BLUEPRINT_VERSION);
        document["materials"] = serde_json::json!([{
            "external_id": "guide",
            "title": {"en": "Voter guide"},
            "kind": "application/pdf",
            "file_name": "guide.pdf",
            "bytes": "JVBERi0xLjQK"
        }]);

        let (_root, directory) = compile(&document);

        assert!(
            written(&directory)
                .iter()
                .any(|name| name.starts_with("export_S3_files/") && name.ends_with("guide.pdf")),
            "{:?}",
            written(&directory)
        );
    }
}

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

use anyhow::{anyhow, Context, Result};
use clap::Args;
use colored::Colorize;
use sequent_core::election_config::architect::{compile_plan, Blueprint};
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
        let source = fs::read_to_string(&self.plan)
            .with_context(|| format!("could not read {}", self.plan.display()))?;
        let plan: Blueprint = serde_json::from_str(&source)
            .with_context(|| format!("{} is not an election plan", self.plan.display()))?;

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

        let compiled = match compile_plan(&plan, &templates, &options, profile.as_ref()) {
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
        let warnings = compiled.report.warnings().count();
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
            let path = directory.join(&artifact.name);
            fs::write(&path, &artifact.bytes)
                .with_context(|| format!("could not write {}", path.display()))?;
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

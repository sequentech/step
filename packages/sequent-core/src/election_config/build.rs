// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Turning a source document's rows into an import bundle.
//!
//! Each entity is a rendered template with the row's dotted-path columns
//! deep-merged over it, identified by a [`super::ids::IdFactory`] uuid5, and
//! joined to the others by `external_id`. Nothing here reaches for a clock or a
//! filesystem, so the same build runs in `step-cli` and in a browser and produces
//! the same bytes.
//!
//! Problems accumulate rather than stopping at the first one: an author fixing a
//! spreadsheet wants the whole list, not one round trip per mistake. Every
//! problem carries the sheet and row it came from, because a bundle path is no
//! use to whoever has to edit the file.
//!
//! Split across three files so each stays readable, all one unit of code: the CSV
//! members and the files that travel beside a bundle are in `build_tables.rs`, and
//! everything to do with the Keycloak realm is in `build_realm.rs`. Both are child
//! modules, so both can reach the resolved ids without any of it being public.

use crate::election_config::ids::IdFactory;
use crate::election_config::paths::{deep_merge, set_path, split_path};
use crate::election_config::presets::{self, AuthPreset, RealmPatch};
use crate::election_config::problem::{Code, Problem, Report};
use crate::election_config::render::TemplateSet;
use crate::election_config::sheet::{
    normalise_sheet_name, Origin, Row, Workbook, SHEET_AREAS,
    SHEET_AREA_CONTESTS, SHEET_CANDIDATES, SHEET_CONTESTS, SHEET_ELECTIONS,
    SHEET_ELECTION_EVENT, SHEET_MATERIALS, SHEET_PARAMETERS, SHEET_REPORTS,
    SHEET_SCHEDULED_EVENTS,
};
use crate::types::ceremonies::CeremoniesPolicy;
use serde_json::{json, Map, Value};

#[path = "build_realm.rs"]
mod realm;

#[path = "build_tables.rs"]
mod tables;

pub use realm::PARAM_LOGIN_CUSTOM_CSS;
pub use tables::{
    CommunicationTemplate, JsonTable, PlainTable, EVENT_PROCESSORS,
    VOTER_LEADING_COLUMNS,
};

/// Timestamp on every generated entity.
///
/// Fixed rather than "now" so that regenerating an unchanged source produces
/// byte-identical output. A real timestamp would make every regeneration a diff
/// with no information in it, and the importer overwrites these anyway.
pub const DEFAULT_CREATED_AT: &str = "2026-01-01T00:00:00.000000Z";

/// Bundle format version written when neither a base export nor an option says
/// otherwise.
pub const DEFAULT_VERSION: &str = "v10.0.0";

/// Columns the builder consumes itself.
///
/// These are not dotted paths into the entity and must not be merged into it —
/// `election.external_id` is how a contest names its election, not a field called
/// `external_id` on an object called `election`.
pub fn control_columns(sheet_key: &str) -> &'static [&'static str] {
    match sheet_key {
        SHEET_CONTESTS => &["election.external_id"],
        SHEET_CANDIDATES => &["contest.external_id"],
        SHEET_AREAS => &["parent.external_id"],
        SHEET_AREA_CONTESTS => &["area.external_id", "contest.external_id"],
        SHEET_SCHEDULED_EVENTS => &[
            "event_name",
            "event_type",
            "election.external_id",
            "scheduled_datetime",
        ],
        SHEET_REPORTS => &[
            "election.external_id",
            "template.alias",
            "report_type",
            "cron_config",
            "encryption_policy",
            "password",
            "permission_label",
        ],
        _ => &[],
    }
}

/// Parameters whose key opens with one of these is a dotted patch into that
/// target rather than free-form metadata.
pub const PARAMETER_PREFIXES: &[&str] = &[
    "keycloak_event_realm.",
    "keycloak_admin_realm.",
    "election_event.",
];

/// Parameters the builder acts on directly rather than carrying as metadata.
pub const HANDLED_PARAMETERS: &[&str] =
    &["tenant_id", realm::PARAM_LOGIN_CUSTOM_CSS];

/// Fields of a base entity that describe *that* event rather than the platform's
/// defaults, and so must not leak into a new one.
///
/// `bulletin_board_reference` and `public_key` point at the base event's own
/// board and keys; `statistics` and `status` describe a run that already
/// happened. Carrying any of them over produces an event that looks configured
/// and is not.
const SCRUB_EVENT: &[&str] = &[
    "id",
    "tenant_id",
    "external_id",
    "bulletin_board_reference",
    "public_key",
    "statistics",
    "status",
    "created_at",
    "updated_at",
];

const SCRUB_ELECTION: &[&str] = &[
    "id",
    "tenant_id",
    "election_event_id",
    "external_id",
    "keys_ceremony_id",
    "statistics",
    "status",
    "created_at",
    "last_updated_at",
];

const SCRUB_CONTEST: &[&str] = &[
    "id",
    "tenant_id",
    "election_event_id",
    "election_id",
    "external_id",
    "created_at",
    "last_updated_at",
];

const SCRUB_CANDIDATE: &[&str] = &[
    "id",
    "tenant_id",
    "election_event_id",
    "contest_id",
    "external_id",
    "created_at",
    "last_updated_at",
];

const SCRUB_AREA: &[&str] = &[
    "id",
    "tenant_id",
    "election_event_id",
    "parent_id",
    "name",
    "description",
    "created_at",
    "last_updated_at",
];

/// What a caller may vary about a build.
#[derive(Debug, Clone, Default)]
pub struct BuildOptions {
    /// The tenant id to write into the file.
    ///
    /// Import does not read this as a destination: `replace_ids` maps the file's
    /// value onto the tenant of the importing request unconditionally. It still
    /// matters for `export_permissions-<tenant>.csv`, which is a tenant-config
    /// artifact whose name nothing rewrites.
    pub tenant_id: Option<String>,

    /// An existing export to inherit platform defaults from.
    ///
    /// Merged *under* the templates, so it contributes fields the templates do
    /// not know about — a newer platform version's additions, for instance —
    /// without overriding anything the author wrote.
    pub base_export: Option<Value>,

    /// Name for the generated archive. Derived from the event's `external_id`
    /// when absent.
    pub slug: Option<String>,

    /// Timestamp for every entity. [`DEFAULT_CREATED_AT`] when absent.
    pub created_at: Option<String>,

    /// Which authentication preset to apply, overriding the document's
    /// `auth_type`.
    ///
    /// [`presets::NONE`] leaves the realm alone whatever the document declares —
    /// worth having while a client has not yet supplied what a preset needs.
    pub auth_preset: Option<String>,

    /// Images the archive should carry, and which document each one is.
    ///
    /// Not in the workbook, because a cell cannot hold bytes. The sheet carries
    /// `image_document_id` and these carry the file, which is what lets the builder
    /// compose the url a voter's ballot reads — it needs the tenant, and
    /// [`Builder::resolve_tenant_id`] is the only thing that knows which tenant the
    /// bundle will claim.
    ///
    /// Empty for the workbook path, which has no bytes to offer.
    pub images: Vec<ImageFile>,

    /// Voter-facing help documents, for the same reason as [`Self::images`]: a
    /// spreadsheet cell cannot hold bytes, so the sheet names a file and these
    /// carry it.
    ///
    /// Each one's `document_id` must also appear on a `support_materials` row in the
    /// JSON, or the import fails on the *zip entry* with a message about a
    /// replacement map. `validate` refuses that pairing rather than letting the
    /// importer discover it.
    pub materials: Vec<MaterialFile>,

    /// Who holds the election key, and how many of them the tally needs.
    ///
    /// `None` for the workbook path, which has no trustee sheet to draw from —
    /// that bundle keeps the empty `keys_ceremonies` the template writes.
    ///
    /// The names are **names**, not identifiers, and that is the platform's own
    /// convention rather than a shortcut:
    /// `windmill/src/services/import/import_election_event.rs` builds a
    /// `HashMap<name, id>` from `get_all_trustees(tenant_id)` and maps the
    /// bundle's `trustee_ids` through it, the same way a voter's area name is
    /// resolved. And the same trap: an unmatched name goes through
    /// `.unwrap_or_default()` and becomes an empty string, so the ceremony
    /// imports with a member who does not exist and nothing reports it.
    pub keys_ceremony: Option<KeysCeremonyPlan>,

    /// Whether the key ceremony is run by people or by the platform.
    ///
    /// Written into the ceremony's `settings`, which is where
    /// `KeysCeremony::policy()` looks and which the importer carries through
    /// untouched. Defaults to `manual-ceremonies` — the platform's own default,
    /// and what every bundle built before this field existed silently was.
    pub ceremony_policy: CeremoniesPolicy,
}

/// Who holds the election key. See [`BuildOptions::keys_ceremony`].
#[derive(Debug, Clone, Default)]
pub struct KeysCeremonyPlan {
    /// Trustee **names**, resolved against the target tenant on import.
    pub trustee_names: Vec<String>,
    pub threshold: i64,
}

/// One image on its way into the archive.
///
/// The three parts of a photograph have to agree, and this holds two of them: the
/// identifier the JSON names and the file the archive carries. See
/// `engineering/how-an-image-travels-in-a-bundle` for what the importer does with
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageFile {
    /// The document's identifier, as `image_document_id` and the url both name it.
    pub document_id: String,
    /// The file's own name. The last segment of the url, and of the archive entry.
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// One voter-facing help document, on its way into the archive.
///
/// The same shape as [`ImageFile`] and a different destination, which is the whole
/// distinction: a candidate's photograph is **public** and goes to `images/`, where
/// the importer uploads it with `is_public = true` and the Voting Portal renders it
/// straight from the public bucket. A support material is **private** — the portal
/// fetches it through the authenticated document route — so it goes to
/// `export_S3_files/`, which the importer uploads against the election event with
/// `is_public = false`.
///
/// Putting one in the other's folder is not a cosmetic error: a material in
/// `images/` is published to anybody holding the URL, and a photograph in
/// `export_S3_files/` 404s on every ballot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaterialFile {
    /// The document's identifier, which the material row's `document_id` names.
    pub document_id: String,
    pub file_name: String,
    pub bytes: Vec<u8>,
}

impl MaterialFile {
    /// The archive entry the importer's `export_S3_files/` branch expects.
    ///
    /// `document_<id>_<name>`, matched by the same unanchored `extract_document_uuid`
    /// that reads an image's name — so the tempfile prefix the platform's own
    /// exporter adds is optional here too.
    pub fn entry_name(&self) -> String {
        format!(
            "export_S3_files/document_{}_{}",
            self.document_id, self.file_name
        )
    }
}

impl ImageFile {
    /// The archive entry name the importer's `images/` branch expects.
    ///
    /// No tempfile prefix. The platform's own exports carry a 12-character one —
    /// `enGgihs9azd5document_…` — which is an artefact of how the exporter names
    /// temporary files; `extract_document_uuid` matches unanchored, so leaving it
    /// out is accepted and keeps the same plan producing the same bytes.
    pub fn entry_name(&self) -> String {
        format!("images/document_{}_{}", self.document_id, self.file_name)
    }

    /// Where the file will be readable, relative to `PUBLIC_BUCKET_URL`.
    ///
    /// Bucket-relative rather than absolute: the Voting Portal concatenates, so an
    /// `https://…` value would produce `https://bucket/https://…`.
    pub fn public_path(&self, tenant_id: &str) -> String {
        format!(
            "tenant-{}/document-{}/{}",
            tenant_id, self.document_id, self.file_name
        )
    }
}

/// A built bundle: the JSON document and what was worth saying about it.
#[derive(Debug, Clone)]
pub struct Bundle {
    pub slug: String,
    pub tenant_id: String,
    pub event_id: String,
    pub event_external_id: String,

    /// The `export_election_event-<id>.json` document.
    pub export: Value,

    /// `export_voters-<id>.csv`.
    pub voters: PlainTable,

    /// `export_scheduled_events-<id>.csv`, the JSON-in-CSV member.
    ///
    /// Where the voting window actually lives: `scheduled_events` in the JSON
    /// document is not read by the importer.
    pub scheduled_events: JsonTable,

    /// `export_reports-<id>.csv`, or nothing when the source names no reports.
    pub reports: Option<PlainTable>,

    /// Admin users, tenant-scoped rather than part of the event import.
    ///
    /// A secret: it carries clear-text passwords when the source does.
    pub admin_users: Option<PlainTable>,

    /// The role/permission matrix, transposed into the platform's own shape.
    pub role_permissions: Option<PlainTable>,

    /// Communication and report templates, loaded through the Admin Portal
    /// rather than imported — the event zip has no member for them.
    pub templates: Vec<CommunicationTemplate>,

    /// Photographs, which *are* imported: `images/` members the importer uploads
    /// and keeps pointed at by the same replacement map that renames the JSON.
    pub images: Vec<ImageFile>,

    /// Support materials' files, the private counterpart of [`Self::images`].
    pub materials: Vec<MaterialFile>,

    /// Everything the document asked of the event's Keycloak realm.
    ///
    /// Kept whether or not it could be applied here, so that an `auth_type` or a
    /// `keycloak_event_realm.*` parameter is never lost just because no base
    /// export was given.
    pub realm_patch: RealmPatch,

    /// `keycloak_admin_realm.*` parameters. Tenant-scoped, so not part of the
    /// event import.
    pub admin_realm_patch: Map<String, Value>,

    /// The preset that was applied, if any.
    pub auth_preset: Option<&'static str>,

    /// Warnings. Not errors: the bundle imports, but something about it looks
    /// unintended.
    pub warnings: Report,
}

/// Build a bundle from a source document.
///
/// Returns every problem at once on failure, so one run tells an author
/// everything they need to fix.
pub fn build(
    workbook: &Workbook,
    templates: &TemplateSet,
    options: &BuildOptions,
) -> Result<Bundle, Report> {
    Builder::new(workbook, templates, options)?.build()
}

struct Builder<'a> {
    workbook: &'a Workbook,
    templates: &'a TemplateSet,
    base_export: Value,
    created_at: String,

    report: Report,

    /// `(type, key) -> value` from the Parameters sheet, in sheet order.
    parameters: Vec<(String, String, Value)>,

    auth_preset: Option<&'static AuthPreset>,
    realm_patch: RealmPatch,

    /// Who holds the election key. See [`BuildOptions::keys_ceremony`].
    keys_ceremony: Option<KeysCeremonyPlan>,
    ceremony_policy: CeremoniesPolicy,
    images: Vec<ImageFile>,
    materials: Vec<MaterialFile>,

    event_row: Row,
    event_external_id: String,
    event_id: String,
    tenant_id: String,
    slug: String,
    ids: IdFactory,

    /// `external_id` -> generated UUID, per entity kind.
    election_ids: Vec<(String, String)>,
    contest_ids: Vec<(String, String)>,
    area_ids: Vec<(String, String)>,
    area_names: Vec<(String, String)>,
}

impl<'a> Builder<'a> {
    fn new(
        workbook: &'a Workbook,
        templates: &'a TemplateSet,
        options: &BuildOptions,
    ) -> Result<Self, Report> {
        let mut report = Report::default();
        let event_row = event_row(workbook).map_err(|problem| {
            let mut report = Report::default();
            report.push(problem);
            report
        })?;

        let event_external_id = match event_row.text("external_id") {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => {
                report.push(Problem::error(
                    Code::MissingField,
                    event_row.origin(Some("external_id")).to_string(),
                    "the election event needs an external_id: every generated \
                     identifier is derived from it",
                ));
                return Err(report);
            }
        };

        // Unwrap is sound: the id was just checked to be non-empty.
        let ids = IdFactory::new(&event_external_id)
            .expect("a non-empty external_id always yields a factory");
        let event_id = ids.uid("election_event", &[&event_external_id]);

        let base_export = options.base_export.clone().unwrap_or(Value::Null);
        let mut builder = Builder {
            keys_ceremony: options.keys_ceremony.clone(),
            ceremony_policy: options.ceremony_policy.clone(),
            images: options.images.clone(),
            materials: options.materials.clone(),
            workbook,
            templates,
            base_export,
            created_at: options
                .created_at
                .clone()
                .unwrap_or_else(|| DEFAULT_CREATED_AT.to_string()),
            report,
            parameters: Vec::new(),
            auth_preset: None,
            realm_patch: RealmPatch::default(),
            event_row,
            event_external_id: event_external_id.clone(),
            event_id,
            tenant_id: String::new(),
            slug: options
                .slug
                .clone()
                .unwrap_or_else(|| slugify(&event_external_id)),
            ids,
            election_ids: Vec::new(),
            contest_ids: Vec::new(),
            area_ids: Vec::new(),
            area_names: Vec::new(),
        };

        builder.parameters = builder.read_parameters();
        builder.tenant_id =
            builder.resolve_tenant_id(options.tenant_id.as_deref());
        builder.auth_preset =
            builder.resolve_auth_preset(options.auth_preset.as_deref());
        Ok(builder)
    }

    fn build(mut self) -> Result<Bundle, Report> {
        // Built before the realm, because the realm applies it, and before the
        // entities so that a preset's requirements are reported first.
        self.realm_patch = self.build_realm_patch();
        self.warn_permission_labels();

        let elections = self.build_elections();
        let contests = self.build_contests();
        let candidates = self.build_candidates();
        let areas = self.build_areas();
        let area_contests = self.build_area_contests();
        let event = self.build_election_event();
        // After the event, because a stale base event id is swapped out of the
        // realm's URLs and that id comes from the base export.
        let realm = self.build_realm();
        let admin_realm_patch = self.admin_realm_patch();

        // One ceremony, from the trustees the caller supplied. The workbook path
        // supplies none and keeps the template's empty array.
        let keys_ceremonies = match self.keys_ceremony.as_ref() {
            None => Vec::new(),
            Some(plan) if plan.trustee_names.is_empty() => Vec::new(),
            Some(plan) => vec![json!({
                "id": self.ids.uid("keys_ceremony", &[&self.event_external_id]),
                "tenant_id": self.tenant_id,
                "election_event_id": self.event_id,
                // Names. See `BuildOptions::keys_ceremony`.
                "trustee_ids": plan.trustee_names,
                "threshold": plan.threshold,
                "is_default": true,
                "name": "Key ceremony",
                // Where `KeysCeremony::policy()` reads it. Absent, it falls back
                // to manual — so writing it is the difference between a client
                // getting what they chose and getting the default quietly.
                "settings": {"policy": self.ceremony_policy.to_string()},
                "permission_label": [],
            })],
        };

        // One row per material the caller supplied, carrying the `document_id`
        // that puts the archive entry's identifier into the importer's replacement
        // map. Built here rather than from a sheet column so the derivation stays
        // in one place, the way `keys_ceremonies` is.
        // From the **sheet**, so the wizard and a workbook produce the same rows.
        //
        // The row names a file; the bytes arrive separately in `options.materials`,
        // keyed by that name — which is what lets a workbook carry documents at all:
        // a spreadsheet cell cannot hold one, so the sheet names `rules.pdf` and the
        // caller hands over a folder or a zip containing it.
        //
        // The document identifier is derived here from the row's `external_id`
        // rather than stored, exactly as every other entity's is, so the JSON and
        // the archive entry cannot disagree and two runs of one workbook produce the
        // same bytes.
        let mut support_materials: Vec<Value> = Vec::new();
        let mut material_files: Vec<MaterialFile> = Vec::new();
        let by_name: std::collections::BTreeMap<&str, &MaterialFile> = self
            .materials
            .iter()
            .map(|file| (file.file_name.as_str(), file))
            .collect();

        for row in self.workbook.rows(SHEET_MATERIALS) {
            let external_id = row.text("external_id").unwrap_or_default();
            if external_id.is_empty() {
                self.report.push(
                    Problem::error(
                        Code::MissingField,
                        format!("{}.external_id", SHEET_MATERIALS),
                        "a support material needs an identifier: its document's id \
                         is derived from it, so without one the file cannot be \
                         matched to the row",
                    )
                    .id("material.no-identifier"),
                );
                continue;
            }

            let file_name = row.text("file").unwrap_or_default();
            let document_id = (!file_name.is_empty())
                .then(|| self.ids.uid("material-document", &[&external_id]));

            // A row naming a file nobody supplied. Refused rather than emitted:
            // the row would import as a link to a document that was never created,
            // which is a tab of broken links and no error anywhere.
            if !file_name.is_empty() {
                match by_name.get(&*file_name) {
                    Some(file) => material_files.push(MaterialFile {
                        document_id: document_id.clone().unwrap_or_default(),
                        file_name: file.file_name.clone(),
                        bytes: file.bytes.clone(),
                    }),
                    None => {
                        self.report.push(
                            Problem::error(
                                Code::DanglingReference,
                                format!("{}.file", SHEET_MATERIALS),
                                format!(
                                    "'{file_name}' is named here and was not \
                                     supplied. Put the file beside the workbook \
                                     under exactly that name."
                                ),
                            )
                            .id("material.file-missing")
                            .detail("file", &file_name)
                            .about(Some(&external_id)),
                        );
                        continue;
                    }
                }
            }

            let mut data = serde_json::Map::new();
            if !file_name.is_empty() {
                data.insert("file_name".to_string(), json!(file_name));
            }
            // Whatever languages the sheet actually carries, read off the row's
            // own headers rather than from a list of languages this function does
            // not have. `presentation.i18n.<lang>.title` is the workbook's own
            // shape, so a bilingual sheet needs no extra column convention.
            let mut title = serde_json::Map::new();
            for (column, value) in &row.cells {
                if let Some(rest) = column.strip_prefix("presentation.i18n.") {
                    if let Some(language) = rest.strip_suffix(".title") {
                        title.insert(language.to_string(), value.clone());
                    }
                }
            }
            if !title.is_empty() {
                data.insert("title".to_string(), Value::Object(title));
            }

            support_materials.push(json!({
                "id": self.ids.uid("support_material", &[&external_id]),
                "tenant_id": self.tenant_id,
                "election_event_id": self.event_id,
                "document_id": document_id,
                "kind": row.text("kind").filter(|kind| !kind.is_empty())
                    .unwrap_or("document"),
                "data": Value::Object(data),
                "labels": {},
                "annotations": {},
                "is_hidden": row.get("is_hidden").and_then(Value::as_bool).unwrap_or(false),
                "created_at": self.created_at,
                "last_updated_at": self.created_at,
            }));
        }

        // A file nobody named. The mirror of the check above, and the more dangerous
        // of the two: it lands in the archive, the importer creates a document for
        // it, and nothing ever points at it.
        for file in &self.materials {
            if !material_files
                .iter()
                .any(|used| used.file_name == file.file_name)
            {
                self.report.push(
                    Problem::warning(
                        Code::BallotCoverage,
                        SHEET_MATERIALS,
                        format!(
                            "'{}' was supplied and no row names it, so it would be \
                             uploaded and shown to nobody.",
                            file.file_name
                        ),
                    )
                    .id("material.file-unused")
                    .detail("file", &file.file_name),
                );
            }
        }

        let version = self
            .base_export
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or(DEFAULT_VERSION)
            .to_string();

        let export = json!({
            "tenant_id": self.tenant_id,
            "keycloak_event_realm": realm,
            "election_event": event,
            "elections": elections,
            "contests": contests,
            "candidates": candidates,
            "areas": areas,
            "area_contests": area_contests,
            // The voting window travels in export_scheduled_events.csv, which is
            // the member import_election_event.rs actually reads.
            "scheduled_events": Value::Null,
            // Reports likewise come from export_reports.csv: insert_reports is
            // only ever reached from process_reports_file, so a populated array
            // here would be silently dropped.
            "reports": [],
            "keys_ceremonies": keys_ceremonies,
            "applications": [],
            "support_materials": support_materials,
            "version": version,
        });

        // Built after the entities, because every one of these resolves an
        // external_id against them.
        let voters = self.build_voters();
        let scheduled_events = self.build_scheduled_events();
        let reports = self.build_reports();
        let admin_users = self.build_admin_users();
        let role_permissions = self.build_role_permissions();
        let templates = self.build_templates();

        if self.report.has_errors() {
            return Err(self.report);
        }

        // Only warnings are left, and they travel with the bundle.
        Ok(Bundle {
            slug: self.slug,
            tenant_id: self.tenant_id,
            event_id: self.event_id,
            event_external_id: self.event_external_id,
            export,
            voters,
            scheduled_events,
            reports,
            admin_users,
            role_permissions,
            templates,
            images: self.images,
            materials: material_files,
            realm_patch: self.realm_patch,
            admin_realm_patch,
            auth_preset: self.auth_preset.map(|preset| preset.name),
            warnings: self.report,
        })
    }

    // -- problems ---------------------------------------------------------

    /// Record a problem and keep going, so one run reports every issue.
    pub(super) fn problem(
        &mut self,
        origin: Origin,
        code: Code,
        message: impl Into<String>,
    ) {
        self.report
            .push(Problem::error(code, origin.to_string(), message));
    }

    pub(super) fn warn(
        &mut self,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.report
            .push(Problem::warning(Code::InvalidValue, path, message));
    }

    // -- parameters -------------------------------------------------------

    fn read_parameters(&mut self) -> Vec<(String, String, Value)> {
        let mut parameters = Vec::new();
        let mut warnings = Vec::new();

        for row in self.workbook.rows(SHEET_PARAMETERS) {
            let Some(key) = row.get("key") else {
                // A row with a comment but no key is a note to the author.
                continue;
            };
            let key = value_as_text(key).trim().to_string();
            if key.is_empty() {
                continue;
            }

            match row.get("value") {
                Some(value) => {
                    let kind = row
                        .get("type")
                        .map(|kind| value_as_text(kind).trim().to_string())
                        .unwrap_or_default();
                    parameters.push((kind, key, value.clone()));
                }
                None => {
                    // A key with no value is a placeholder the author left
                    // blank, e.g. an IdP metadata URL pending the client.
                    warnings.push((
                        row.origin(Some("value")).to_string(),
                        format!(
                            "parameter '{key}' has no value and is ignored"
                        ),
                    ));
                }
            }
        }

        for (path, message) in warnings {
            self.warn(path, message);
        }
        parameters
    }

    /// Look a parameter up by key, whatever its `type` column says.
    ///
    /// The type column is documentation for the author; nothing downstream
    /// distinguishes an `event` parameter from a `settings` one.
    fn parameter(&self, key: &str) -> Option<&Value> {
        self.parameters
            .iter()
            .find(|(_, name, _)| name == key)
            .map(|(_, _, value)| value)
    }

    /// Parameters whose key is a dotted path under `prefix`.
    fn parameter_patches(&self, prefix: &str) -> Vec<(String, Value)> {
        self.parameters
            .iter()
            .filter_map(|(_, key, value)| {
                key.strip_prefix(prefix)
                    .map(|path| (path.to_string(), value.clone()))
            })
            .collect()
    }

    /// Parameters nothing acts on, to be carried in the event's annotations.
    ///
    /// Recorded rather than dropped: a row someone put in the spreadsheet meant
    /// something to them, and silently ignoring it is how a setting goes missing
    /// on election day.
    fn uninterpreted_parameters(&self) -> Vec<(String, Value)> {
        // A key a preset takes is not uninterpreted, whether or not a preset was
        // selected: reporting it as ignored while a preset would have acted on it
        // contradicts itself.
        let consumed = presets::all_preset_parameters();

        let mut carried: Vec<(String, Value)> = self
            .parameters
            .iter()
            .filter(|(_, key, _)| {
                !HANDLED_PARAMETERS.contains(&key.as_str())
                    && !consumed.contains(&key.as_str())
                    && !PARAMETER_PREFIXES
                        .iter()
                        .any(|prefix| key.starts_with(prefix))
            })
            .map(|(kind, key, value)| {
                let kind = if kind.is_empty() { "event" } else { kind };
                // The prefix is what the SEIU1000 bundle already carries; keeping
                // it means a regenerated event does not move its annotations.
                (
                    format!("janitor.param.{kind}.{key}"),
                    match value {
                        Value::String(_) => value.clone(),
                        other => Value::String(other.to_string()),
                    },
                )
            })
            .collect();
        carried.sort_by(|left, right| left.0.cmp(&right.0));
        carried
    }

    fn resolve_tenant_id(&self, explicit: Option<&str>) -> String {
        if let Some(explicit) = explicit.filter(|id| !id.is_empty()) {
            return explicit.to_string();
        }
        if let Some(from_parameters) = self.parameter("tenant_id") {
            let text = value_as_text(from_parameters);
            if !text.is_empty() {
                return text;
            }
        }
        if let Some(from_base) =
            self.base_export.get("tenant_id").and_then(Value::as_str)
        {
            if !from_base.is_empty() {
                return from_base.to_string();
            }
        }
        self.ids.tenant_id()
    }

    // -- entities ---------------------------------------------------------

    /// Render a template, then merge the row's dotted paths over it.
    pub(super) fn render(
        &mut self,
        template: &str,
        row: Option<&Row>,
        context: Value,
    ) -> Map<String, Value> {
        let rendered = match self.templates.render_json(template, &context) {
            Ok(rendered) => rendered,
            Err(problem) => {
                // A template that does not render is a bug in the template, not
                // in the document, so it is reported as-is.
                self.report.push(problem);
                return Map::new();
            }
        };

        let Some(row) = row else {
            return rendered;
        };

        let excluded = control_columns(&normalise_sheet_name(&row.sheet));
        match row.overrides(excluded) {
            Ok(overrides) => {
                match deep_merge(
                    Value::Object(rendered),
                    Value::Object(overrides),
                ) {
                    Value::Object(merged) => merged,
                    // deep_merge of two objects is an object.
                    _ => unreachable!("merging two objects yields an object"),
                }
            }
            Err(problem) => {
                self.report.push(problem);
                rendered
            }
        }
    }

    /// The first entity of `key` in the base export, scrubbed, if there is one.
    fn base(&self, key: &str, scrub: &[&str]) -> Option<Map<String, Value>> {
        let value = self.base_export.get(key)?;
        let object = match value {
            Value::Object(object) => object.clone(),
            Value::Array(items) => match items.first() {
                Some(Value::Object(object)) => object.clone(),
                _ => return None,
            },
            _ => return None,
        };
        if object.is_empty() {
            return None;
        }

        let mut scrubbed = object;
        for field in scrub {
            scrubbed.remove(*field);
        }
        Some(scrubbed)
    }

    /// Merge a scrubbed base entity under `entity`, then reassert identity.
    ///
    /// Identity always comes from this build, never from the base: a base export
    /// naming its own ids is the whole reason the fields are scrubbed, and
    /// reasserting them makes that impossible to get wrong by adding a field.
    fn under_base(
        &self,
        entity: Map<String, Value>,
        key: &str,
        scrub: &[&str],
        identity: &[(&str, &str)],
    ) -> Map<String, Value> {
        let Some(base) = self.base(key, scrub) else {
            return entity;
        };
        let merged = deep_merge(Value::Object(base), Value::Object(entity));
        let mut merged = match merged {
            Value::Object(object) => object,
            _ => unreachable!("merging two objects yields an object"),
        };
        for (field, value) in identity {
            merged.insert((*field).to_string(), json!(value));
        }
        merged
    }

    fn build_election_event(&mut self) -> Value {
        let context = json!({
            "id": self.event_id,
            "tenant_id": self.tenant_id,
            "created_at": self.created_at,
        });
        let row = self.event_row.clone();
        let event = self.render("election_event", Some(&row), context);

        let event_id = self.event_id.clone();
        let tenant_id = self.tenant_id.clone();
        let base = self.base("election_event", SCRUB_EVENT);
        let mut event = self.under_base(
            event,
            "election_event",
            SCRUB_EVENT,
            &[("id", &event_id), ("tenant_id", &tenant_id)],
        );
        if let Some(base) = base {
            // Say out loud what the base contributed to the voter's screen.
            let merged = event.clone();
            self.warn_inherited_branding(&base, &merged);
        }

        event.insert("external_id".to_string(), json!(self.event_external_id));

        for (path, value) in self.parameter_patches("election_event.") {
            if let Err(problem) =
                set_path(&mut event, &split_path(&path), value)
            {
                self.report.push(problem);
            }
        }

        let carried = self.uninterpreted_parameters();
        if !carried.is_empty() {
            let mut annotations = match event.get("annotations") {
                Some(Value::Object(existing)) => existing.clone(),
                _ => Map::new(),
            };
            let mut names: Vec<String> = Vec::new();
            for (name, value) in carried {
                names
                    .push(name.rsplit('.').next().unwrap_or(&name).to_string());
                annotations.insert(name, value);
            }
            event.insert("annotations".to_string(), Value::Object(annotations));

            names.sort();
            names.dedup();
            self.warn(
                "election_event.annotations",
                format!(
                    "these Parameters rows are recorded in \
                     election_event.annotations but not interpreted: {}",
                    names.join(", ")
                ),
            );
        }

        Value::Object(event)
    }

    fn build_elections(&mut self) -> Value {
        let rows: Vec<Row> = self.workbook.rows(SHEET_ELECTIONS).to_vec();
        let mut elections = Vec::new();
        let mut seen: Vec<(String, usize)> = Vec::new();

        for row in &rows {
            let Some(external_id) =
                self.require_external_id(row, "an election", &mut seen)
            else {
                continue;
            };

            let election_id = self.ids.uid("election", &[&external_id]);
            self.election_ids
                .push((external_id.clone(), election_id.clone()));

            let context = json!({
                "id": election_id,
                "tenant_id": self.tenant_id,
                "election_event_id": self.event_id,
                "created_at": self.created_at,
            });
            let election = self.render("election", Some(row), context);

            let event_id = self.event_id.clone();
            let tenant_id = self.tenant_id.clone();
            let mut election = self.under_base(
                election,
                "elections",
                SCRUB_ELECTION,
                &[
                    ("id", &election_id),
                    ("tenant_id", &tenant_id),
                    ("election_event_id", &event_id),
                ],
            );
            election.insert("external_id".to_string(), json!(external_id));
            elections.push(Value::Object(election));
        }

        if elections.is_empty() {
            self.problem(
                Origin::sheet("Elections"),
                Code::MissingField,
                "an election event needs at least one election",
            );
        }
        Value::Array(elections)
    }

    fn build_contests(&mut self) -> Value {
        let rows: Vec<Row> = self.workbook.rows(SHEET_CONTESTS).to_vec();
        let mut contests = Vec::new();
        let mut seen: Vec<(String, usize)> = Vec::new();

        for row in &rows {
            let Some(external_id) =
                self.require_external_id(row, "a contest", &mut seen)
            else {
                continue;
            };

            let elections = self.election_ids.clone();
            let Some(election_id) = self.resolve(
                row,
                "election.external_id",
                &elections,
                "election",
            ) else {
                continue;
            };

            let contest_id = self.ids.uid("contest", &[&external_id]);
            self.contest_ids
                .push((external_id.clone(), contest_id.clone()));

            let context = json!({
                "id": contest_id,
                "tenant_id": self.tenant_id,
                "election_event_id": self.event_id,
                "election_id": election_id,
                "created_at": self.created_at,
            });
            let contest = self.render("contest", Some(row), context);

            let event_id = self.event_id.clone();
            let tenant_id = self.tenant_id.clone();
            let mut contest = self.under_base(
                contest,
                "contests",
                SCRUB_CONTEST,
                &[
                    ("id", &contest_id),
                    ("tenant_id", &tenant_id),
                    ("election_event_id", &event_id),
                    ("election_id", &election_id),
                ],
            );
            contest.insert("external_id".to_string(), json!(external_id));
            contests.push(Value::Object(contest));
        }

        if contests.is_empty() {
            self.problem(
                Origin::sheet("Contests"),
                Code::MissingField,
                "an election event needs at least one contest",
            );
        }
        Value::Array(contests)
    }

    /// The photograph for one candidate, by the identifier its row names.
    fn image_for(&self, document_id: &str) -> Option<&ImageFile> {
        self.images
            .iter()
            .find(|image| image.document_id == document_id)
    }

    fn build_candidates(&mut self) -> Value {
        let rows: Vec<Row> = self.workbook.rows(SHEET_CANDIDATES).to_vec();
        let mut candidates = Vec::new();
        let mut seen: Vec<(String, usize)> = Vec::new();

        for row in &rows {
            let Some(external_id) =
                self.require_external_id(row, "a candidate", &mut seen)
            else {
                continue;
            };

            let contests = self.contest_ids.clone();
            let Some(contest_id) =
                self.resolve(row, "contest.external_id", &contests, "contest")
            else {
                continue;
            };

            let candidate_id = self.ids.uid("candidate", &[&external_id]);
            let context = json!({
                "id": candidate_id,
                "tenant_id": self.tenant_id,
                "election_event_id": self.event_id,
                "contest_id": contest_id,
                "created_at": self.created_at,
            });
            let candidate = self.render("candidate", Some(row), context);

            let event_id = self.event_id.clone();
            let tenant_id = self.tenant_id.clone();
            let mut candidate = self.under_base(
                candidate,
                "candidates",
                SCRUB_CANDIDATE,
                &[
                    ("id", &candidate_id),
                    ("tenant_id", &tenant_id),
                    ("election_event_id", &event_id),
                    ("contest_id", &contest_id),
                ],
            );
            candidate.insert("external_id".to_string(), json!(external_id));

            // The photograph's other half. `image_document_id` came from the row;
            // the url is composed here because it embeds the tenant, and this is
            // the only place that knows which tenant the bundle claims.
            //
            // Only when the archive actually carries the file: a row naming a
            // document with no member behind it gets no url, and `validate`'s
            // `check_images` says so rather than putting a broken picture on a
            // ballot.
            let named = candidate
                .get("image_document_id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string);
            if let Some(image) =
                named.as_deref().and_then(|id| self.image_for(id))
            {
                let url = image.public_path(&self.tenant_id);
                let presentation = candidate
                    .entry("presentation".to_string())
                    .or_insert_with(|| json!({}));
                if let Some(presentation) = presentation.as_object_mut() {
                    // Replacing any image entry rather than appending, which is
                    // what the Admin Portal's own uploader does: `getImageUrl`
                    // takes the *first* `is_image` url, so a second one would be
                    // dead weight that only shows up as a stale picture.
                    let mut urls: Vec<Value> = presentation
                        .get("urls")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                        .into_iter()
                        .filter(|url| {
                            url.get("is_image").and_then(Value::as_bool)
                                != Some(true)
                        })
                        .collect();
                    urls.push(json!({"url": url, "is_image": true}));
                    presentation.insert("urls".to_string(), json!(urls));
                }
            }

            candidates.push(Value::Object(candidate));
        }

        Value::Array(candidates)
    }

    fn build_areas(&mut self) -> Value {
        let rows: Vec<Row> = self.workbook.rows(SHEET_AREAS).to_vec();

        // Two passes over the sheet: ids first, so a parent may appear below its
        // own child. Authors do not sort their spreadsheets topologically.
        let mut seen: Vec<(String, usize)> = Vec::new();
        for row in &rows {
            let Some(external_id) =
                self.require_external_id(row, "an area", &mut seen)
            else {
                continue;
            };
            let area_id = self.ids.uid("area", &[&external_id]);
            self.area_ids.push((external_id.clone(), area_id));

            match row.get("name").map(value_as_text) {
                Some(name) if !name.is_empty() => {
                    self.area_names.push((external_id, name));
                }
                _ => self.problem(
                    row.origin(Some("name")),
                    Code::MissingField,
                    "an area needs a name: the voters CSV identifies a \
                     voter's area by name, not by id",
                ),
            }
        }

        // Names have to be unique for the same reason.
        let names = self.area_names.clone();
        for (index, (external_id, name)) in names.iter().enumerate() {
            if let Some((earlier, _)) = names[..index]
                .iter()
                .find(|(_, seen_name)| seen_name == name)
            {
                self.problem(
                    Origin::column("Areas", "name"),
                    Code::DuplicateId,
                    format!(
                        "two areas are both named '{name}' ('{earlier}' and \
                         '{external_id}'); the voters CSV resolves an area by \
                         name, so names must be unique"
                    ),
                );
            }
        }

        let mut areas = Vec::new();
        for row in &rows {
            let Some(external_id) = row.text("external_id").map(str::to_string)
            else {
                continue;
            };
            let Some(area_id) = lookup(&self.area_ids, &external_id) else {
                continue;
            };

            let mut parent_id = None;
            if row.get("parent.external_id").is_some() {
                let areas_so_far = self.area_ids.clone();
                let Some(resolved) = self.resolve(
                    row,
                    "parent.external_id",
                    &areas_so_far,
                    "area",
                ) else {
                    continue;
                };
                if resolved == area_id {
                    self.problem(
                        row.origin(Some("parent.external_id")),
                        Code::AreaCycle,
                        "an area cannot be its own parent",
                    );
                    continue;
                }
                parent_id = Some(resolved);
            }

            let context = json!({
                "id": area_id,
                "tenant_id": self.tenant_id,
                "election_event_id": self.event_id,
                "created_at": self.created_at,
            });
            let area = self.render("area", Some(row), context);

            let event_id = self.event_id.clone();
            let tenant_id = self.tenant_id.clone();
            let mut area = self.under_base(
                area,
                "areas",
                SCRUB_AREA,
                &[
                    ("id", &area_id),
                    ("tenant_id", &tenant_id),
                    ("election_event_id", &event_id),
                ],
            );
            if let Some(parent_id) = parent_id {
                area.insert("parent_id".to_string(), json!(parent_id));
            }
            areas.push(Value::Object(area));
        }

        if areas.is_empty() {
            self.problem(
                Origin::sheet("Areas"),
                Code::MissingField,
                "an election event needs at least one area; every voter \
                 belongs to one",
            );
        }
        Value::Array(areas)
    }

    fn build_area_contests(&mut self) -> Value {
        let rows: Vec<Row> = self.workbook.rows(SHEET_AREA_CONTESTS).to_vec();
        let mut links = Vec::new();
        let mut seen: Vec<((String, String), usize)> = Vec::new();

        for row in &rows {
            let areas = self.area_ids.clone();
            let contests = self.contest_ids.clone();
            let area_external = row
                .text("area.external_id")
                .map(str::trim)
                .unwrap_or_default()
                .to_string();
            let contest_external = row
                .text("contest.external_id")
                .map(str::trim)
                .unwrap_or_default()
                .to_string();

            let area_id = self.resolve(row, "area.external_id", &areas, "area");
            let contest_id =
                self.resolve(row, "contest.external_id", &contests, "contest");
            let (Some(area_id), Some(contest_id)) = (area_id, contest_id)
            else {
                continue;
            };

            let key = (area_external, contest_external);
            if let Some((_, earlier)) =
                seen.iter().find(|(seen_key, _)| seen_key == &key)
            {
                let message = format!(
                    "area '{}' is already linked to contest '{}' on row {earlier}",
                    key.0, key.1
                );
                self.problem(row.origin(None), Code::DuplicateId, message);
                continue;
            }
            seen.push((key.clone(), row.number));

            let context = json!({
                "id": self.ids.uid("area_contest", &[&key.0, &key.1]),
                "area_id": area_id,
                "contest_id": contest_id,
            });
            let link = self.render("area_contest", Some(row), context);
            links.push(Value::Object(link));
        }

        if links.is_empty() {
            self.problem(
                Origin::sheet("AreaContests"),
                Code::BallotCoverage,
                "no area is linked to any contest, so no voter would see a \
                 ballot",
            );
        }
        Value::Array(links)
    }

    // -- shared row handling ----------------------------------------------

    /// The row's `external_id`, or a problem naming what is wrong with it.
    fn require_external_id(
        &mut self,
        row: &Row,
        what: &str,
        seen: &mut Vec<(String, usize)>,
    ) -> Option<String> {
        let external_id = match row.get("external_id").map(value_as_text) {
            Some(id) if !id.trim().is_empty() => id.trim().to_string(),
            _ => {
                self.problem(
                    row.origin(Some("external_id")),
                    Code::MissingField,
                    format!("{what} needs an external_id"),
                );
                return None;
            }
        };

        if let Some((_, earlier)) =
            seen.iter().find(|(id, _)| id == &external_id)
        {
            let message = format!(
                "external_id '{external_id}' is already used by row {earlier}"
            );
            self.problem(
                row.origin(Some("external_id")),
                Code::DuplicateId,
                message,
            );
            return None;
        }
        seen.push((external_id.clone(), row.number));
        Some(external_id)
    }

    /// `external_id` -> UUID, recording a problem when it does not resolve.
    pub(super) fn resolve(
        &mut self,
        row: &Row,
        column: &str,
        table: &[(String, String)],
        kind: &str,
    ) -> Option<String> {
        let Some(raw) = row.get(column).map(value_as_text) else {
            self.problem(
                row.origin(Some(column)),
                Code::MissingField,
                // Phrased without an article on purpose: "a election" is what
                // the obvious formatting gives, and this message is read by
                // whoever has to fix the file.
                format!(
                    "'{column}' is required: it names the {kind} this row belongs to"
                ),
            );
            return None;
        };
        let key = raw.trim();
        match lookup(table, key) {
            Some(resolved) => Some(resolved),
            None => {
                self.problem(
                    row.origin(Some(column)),
                    Code::DanglingReference,
                    format!("no {kind} has external_id '{key}'"),
                );
                None
            }
        }
    }
}

/// The one row of the ElectionEvent sheet.
fn event_row(workbook: &Workbook) -> Result<Row, Problem> {
    let rows = workbook.rows(SHEET_ELECTION_EVENT);
    match rows.len() {
        0 => Err(Problem::error(
            Code::MissingField,
            "sheet 'ElectionEvent'",
            "this sheet is empty; it must hold exactly one row, with at least \
             an 'external_id' column",
        )),
        1 => Ok(rows[0].clone()),
        count => Err(Problem::error(
            Code::InvalidValue,
            "sheet 'ElectionEvent'",
            format!(
                "this sheet holds {count} rows; an import describes exactly \
                 one election event"
            ),
        )),
    }
}

fn lookup(table: &[(String, String)], key: &str) -> Option<String> {
    table
        .iter()
        .find(|(external_id, _)| external_id == key)
        .map(|(_, id)| id.clone())
}

/// A JSON value as the text a reference column means.
///
/// A number typed into an id column is an id, not a number: `1001` as an
/// `external_id` has to match `1001` written in a reference column, whichever way
/// each cell happened to be formatted.
fn value_as_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// A filesystem-safe name derived from the event's `external_id`.
fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() {
            slug.extend(character.to_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "election-event".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
#[path = "build_tests.rs"]
mod build_tests;

/// Whether a report holds an error whose message contains `needle`.
#[cfg(test)]
pub(crate) fn has_error_saying(report: &Report, needle: &str) -> bool {
    report
        .errors()
        .any(|problem| problem.message.contains(needle))
}

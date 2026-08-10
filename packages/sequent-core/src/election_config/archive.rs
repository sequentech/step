// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! What a bundle becomes as files, and the zip the Admin Portal accepts.
//!
//! Pure, like the rest of the module: this returns named byte blobs and never
//! touches a filesystem. `step-cli` writes them to a directory; a browser offers
//! them as downloads. Which is the whole reason the split exists — a bundle with a
//! dangling reference leaves no half-written output behind either way, because
//! nothing is written until everything is built.
//!
//! Two groups, and the line between them matters. The **importable** members go
//! inside the zip. The **auxiliary** files go beside it: administrators, roles and
//! communication templates are tenant- or portal-scoped, and putting them in the
//! zip would mean importing an election event could silently create administrator
//! accounts.

use crate::election_config::build::{
    Bundle, CommunicationTemplate, PlainTable,
};
use crate::election_config::emit::{json_csv, member, plain_csv};
use serde_json::{json, Map, Value};

/// One file a bundle becomes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Artifact {
    /// Path relative to the output directory. May contain `/`.
    pub name: String,
    pub bytes: Vec<u8>,
}

impl Artifact {
    fn text(name: impl Into<String>, text: String) -> Self {
        Artifact {
            name: name.into(),
            bytes: text.into_bytes(),
        }
    }

    /// A JSON file, pretty-printed with a trailing newline.
    ///
    /// Two-space indentation throughout, which is `serde_json`'s and differs from
    /// the one-space the Python used for the event document. Cosmetic — the
    /// importer parses it — but it does mean the first regeneration of an existing
    /// event reindents the whole file once.
    fn json(name: impl Into<String>, value: &Value) -> Self {
        let mut text = serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "null".to_string());
        text.push('\n');
        Artifact::text(name, text)
    }

    fn csv(name: impl Into<String>, table: &PlainTable) -> Self {
        let columns: Vec<&str> =
            table.columns.iter().map(String::as_str).collect();
        Artifact::text(name, plain_csv(&columns, &table.rows))
    }
}

/// Everything a bundle is written as.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// The members of the importable zip.
    pub importable: Vec<Artifact>,

    /// What the zip should be called.
    pub archive_name: String,

    /// Files that belong beside the zip, never inside it.
    pub auxiliary: Vec<Artifact>,
}

/// Turn a built bundle into the files it is written as.
pub fn layout(bundle: &Bundle) -> Layout {
    let suffix = &bundle.event_id;

    let mut importable = vec![
        Artifact::json(
            member::file_name(member::ELECTION_EVENT, suffix, "json"),
            &bundle.export,
        ),
        Artifact::csv(
            member::file_name(member::VOTERS, suffix, "csv"),
            &bundle.voters,
        ),
    ];

    // The scheduled-events member is the JSON-in-CSV shape, always written even
    // when empty: the voting window lives here, so an absent file and an empty one
    // should not be told apart by whether the source had a sheet.
    let schedule_columns: Vec<&str> = bundle
        .scheduled_events
        .columns
        .iter()
        .map(String::as_str)
        .collect();
    importable.push(Artifact::text(
        member::file_name(member::SCHEDULED_EVENTS, suffix, "csv"),
        json_csv(&schedule_columns, &bundle.scheduled_events.rows),
    ));

    if let Some(reports) = &bundle.reports {
        importable.push(Artifact::csv(
            member::file_name(member::REPORTS, suffix, "csv"),
            reports,
        ));
    }

    // Photographs, inside the zip. `images/document_<uuid>_<file>` is what the
    // importer's own `images/` branch looks for: it pulls the identifier back out of
    // the entry name and resolves it through the same map that renamed every
    // identifier in the JSON, so the file and the two references to it stay
    // together. Uploaded as public, which is what lets `PUBLIC_BUCKET_URL` serve it.
    //
    // Not sorted or deduplicated here: `plan_images` walks the plan in ballot order
    // and derives each identifier from the candidate's `external_id`, which
    // `check_unique_ids` has already refused duplicates of.
    for image in &bundle.images {
        importable.push(Artifact {
            name: image.entry_name(),
            bytes: image.bytes.clone(),
        });
    }

    // The private counterpart, under `export_S3_files/`. Same mechanism, different
    // folder, and the folder is the whole difference: the importer uploads one with
    // `is_public = true` and the other against the election event with
    // `is_public = false`.
    for material in &bundle.materials {
        importable.push(Artifact {
            name: material.entry_name(),
            bytes: material.bytes.clone(),
        });
    }

    Layout {
        importable,
        archive_name: format!("{}.zip", bundle.slug),
        auxiliary: auxiliary(bundle),
    }
}

/// Files the source describes that are not part of the event import.
fn auxiliary(bundle: &Bundle) -> Vec<Artifact> {
    let mut written = Vec::new();

    if let Some(admin_users) = &bundle.admin_users {
        written.push(Artifact::csv("admin_users.csv", admin_users));
    }

    if let Some(role_permissions) = &bundle.role_permissions {
        // Named after the tenant because nothing rewrites this file's name on the
        // way in — which is the one place the tenant id actually matters.
        written.push(Artifact::csv(
            format!("export_permissions-{}.csv", bundle.tenant_id),
            role_permissions,
        ));
    }

    if !bundle.templates.is_empty() {
        let mut manifest = Vec::new();
        for template in &bundle.templates {
            let file_name = template.file_name();
            written.push(Artifact::text(
                format!("templates/{file_name}"),
                template.document.clone(),
            ));
            manifest.push(template_entry(template, &file_name));
        }
        written.push(Artifact::json(
            "templates/templates.json",
            &Value::Array(manifest),
        ));
    }

    if !bundle.admin_realm_patch.is_empty() {
        written.push(Artifact::json(
            "keycloak_admin_realm_patch.json",
            &Value::Object(bundle.admin_realm_patch.clone()),
        ));
    }

    if !bundle.realm_patch.patch.is_empty() {
        written.push(Artifact::json(
            "keycloak_event_realm_patch.json",
            &realm_patch_document(bundle),
        ));
    }

    written
}

fn template_entry(template: &CommunicationTemplate, file_name: &str) -> Value {
    json!({
        "name": template.name,
        "alias": template.alias,
        "file": file_name,
        "type": template.template_type,
        "communication_method": template.communication_method,
        "selected_methods": template.selected_methods,
    })
}

/// The realm patch as a file someone can read and apply.
///
/// Written even when it was already applied to a base export's realm: it is the
/// readable statement of what the source asked of the realm, and it is what you
/// apply by hand when there was no realm here to apply it to. The comment says
/// which of those happened, because the two need opposite things done next.
fn realm_patch_document(bundle: &Bundle) -> Value {
    let applied = bundle
        .export
        .get("keycloak_event_realm")
        .is_some_and(|realm| !realm.is_null());

    let comment = if applied {
        match bundle.auth_preset {
            Some(preset) => format!(
                "Realm changes this source asks for. Already applied to the realm \
                 in the event zip, via the '{preset}' preset."
            ),
            None => "Realm changes this source asks for. Already applied to the \
                     realm in the event zip."
                .to_string(),
        }
    } else {
        "Realm changes this source asks for. NOT applied: no base export was \
         given, so the event zip carries no realm and the platform will load its \
         own default. Apply this to that realm."
            .to_string()
    };

    let mut document = Map::new();
    document.insert("_comment".to_string(), json!(comment));
    document.insert("auth_preset".to_string(), json!(bundle.auth_preset));
    document.insert(
        "patch".to_string(),
        Value::Object(bundle.realm_patch.patch.clone()),
    );

    // The two directives are separate fields on RealmPatch rather than keys inside
    // the patch, so there is nothing to strip out here — but they are worth stating
    // for whoever has to apply the patch by hand, since neither is a plain merge.
    if let Some((authenticator, config_alias)) =
        &bundle.realm_patch.bind_authenticator_config
    {
        document.insert(
            "bind_authenticator_config".to_string(),
            json!({
                "_comment": "Not a merge: point every execution of this \
                             authenticator at this config alias.",
                "authenticator": authenticator,
                "config_alias": config_alias,
            }),
        );
    }
    if let Some(user_profile) = &bundle.realm_patch.user_profile {
        document.insert(
            "user_profile".to_string(),
            json!({
                "_comment": "Not a merge: the user profile is a stringified JSON \
                             blob inside the \
                             org.keycloak.userprofile.UserProfileProvider \
                             component. Parse it, apply these changes to the \
                             matching entries of its `attributes` list, and write \
                             it back as a string.",
                "attributes": user_profile,
            }),
        );
    }

    Value::Object(document)
}

/// Zip the members at the archive root, reproducibly.
///
/// A fixed timestamp and a fixed mode on every entry: without them the archive's
/// bytes change on every run, and "regenerating produced no diff" stops being
/// something anyone can check.
#[cfg(feature = "election_config_archive")]
pub fn zip(
    members: &[Artifact],
) -> Result<Vec<u8>, crate::election_config::Problem> {
    use crate::election_config::problem::Code;
    use std::io::{Cursor, Write};
    use zip::write::SimpleFileOptions;

    /// 2026-01-01T00:00:00, matching what the Python wrote.
    fn fixed_time() -> zip::DateTime {
        zip::DateTime::from_date_and_time(2026, 1, 1, 0, 0, 0)
            .unwrap_or_default()
    }

    let failed = |error: zip::result::ZipError| {
        crate::election_config::Problem::error(
            Code::InvalidValue,
            "archive",
            format!("could not be written: {error}"),
        )
    };

    let mut buffer = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut buffer));
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .last_modified_time(fixed_time())
            // 0o644, the mode a normal export has.
            .unix_permissions(0o644);

        for artifact in members {
            writer
                .start_file(artifact.name.clone(), options)
                .map_err(failed)?;
            writer.write_all(&artifact.bytes).map_err(|error| {
                crate::election_config::Problem::error(
                    Code::InvalidValue,
                    format!("archive.{}", artifact.name),
                    format!("could not be written: {error}"),
                )
            })?;
        }
        writer.finish().map_err(failed)?;
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::election_config::build::{build, BuildOptions};
    use crate::election_config::paths::Cell;
    use crate::election_config::render::TemplateSet;
    use crate::election_config::sheet::{Sheet, Workbook};

    fn text(value: &str) -> Cell {
        Cell::text(value)
    }

    /// The smallest document that builds, plus whatever a test needs.
    fn bundle(extra: Vec<(&str, Vec<Vec<Cell>>)>) -> Bundle {
        let mut sheets = vec![
            (
                "ElectionEvent",
                vec![
                    vec![
                        text("external_id"),
                        text("presentation.i18n.en.name"),
                    ],
                    vec![text("union-2027"), text("Union Election 2027")],
                ],
            ),
            (
                "Elections",
                vec![vec![text("external_id")], vec![text("statewide")]],
            ),
            (
                "Contests",
                vec![
                    vec![text("external_id"), text("election.external_id")],
                    vec![text("president"), text("statewide")],
                ],
            ),
            (
                "Areas",
                vec![
                    vec![text("external_id"), text("name")],
                    vec![text("area-north"), text("North")],
                ],
            ),
            (
                "AreaContests",
                vec![
                    vec![text("area.external_id"), text("contest.external_id")],
                    vec![text("area-north"), text("president")],
                ],
            ),
        ];
        sheets.extend(extra);

        let workbook = Workbook::new(
            sheets
                .into_iter()
                .map(|(name, grid)| Sheet::from_grid(name, &grid).unwrap())
                .collect(),
        )
        .unwrap();

        let templates = TemplateSet::builtin().unwrap();
        build(&workbook, &templates, &BuildOptions::default())
            .expect("a clean build")
    }

    fn names(artifacts: &[Artifact]) -> Vec<&str> {
        artifacts.iter().map(|a| a.name.as_str()).collect()
    }

    #[test]
    fn the_zip_holds_the_members_the_importer_dispatches_on() {
        // Anything named otherwise is silently ignored rather than rejected.
        let bundle = bundle(vec![]);
        let layout = layout(&bundle);
        assert_eq!(
            names(&layout.importable),
            [
                format!("export_election_event-{}.json", bundle.event_id),
                format!("export_voters-{}.csv", bundle.event_id),
                format!("export_scheduled_events-{}.csv", bundle.event_id),
            ]
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
        );
        assert_eq!(layout.archive_name, "union-2027.zip");
    }

    #[test]
    fn the_reports_member_appears_only_when_there_are_reports() {
        // An empty reports CSV is not a valid one.
        assert!(!names(&layout(&bundle(vec![])).importable)
            .iter()
            .any(|name| name.starts_with("export_reports")));

        let with_reports = bundle(vec![(
            "Reports",
            vec![vec![text("report_type")], vec![text("tally")]],
        )]);
        assert!(names(&layout(&with_reports).importable)
            .iter()
            .any(|name| name.starts_with("export_reports")));
    }

    #[test]
    fn the_scheduled_events_member_is_written_even_when_empty() {
        // The voting window lives in it, so whether the file exists must not
        // depend on whether the source had a sheet.
        let layout = layout(&bundle(vec![]));
        let schedule = layout
            .importable
            .iter()
            .find(|artifact| {
                artifact.name.starts_with("export_scheduled_events")
            })
            .expect("the schedule member");
        let text = String::from_utf8(schedule.bytes.clone()).unwrap();
        assert_eq!(text, "id,tenant_id,election_event_id,created_at,stopped_at,archived_at,labels,annotations,event_processor,cron_config,event_payload,task_id\n");
    }

    #[test]
    fn administrators_are_written_beside_the_zip_and_never_inside_it() {
        // Importing an election event must not be able to create administrator
        // accounts.
        let bundle = bundle(vec![(
            "Admin Users",
            vec![
                vec![text("username"), text("permission_labels")],
                vec![text("admin1"), text("statewide-officers")],
            ],
        )]);
        let layout = layout(&bundle);
        assert!(names(&layout.auxiliary).contains(&"admin_users.csv"));
        assert!(!names(&layout.importable).contains(&"admin_users.csv"));
    }

    #[test]
    fn the_permissions_file_is_named_after_the_tenant() {
        // Nothing rewrites this file's name on the way in, which is the one place
        // the tenant id actually matters.
        let bundle = bundle(vec![(
            "Permissions",
            vec![
                vec![text("permission"), text("admin")],
                vec![text("election:read"), text("x")],
            ],
        )]);
        let layout = layout(&bundle);
        assert!(names(&layout.auxiliary).contains(
            &format!("export_permissions-{}.csv", bundle.tenant_id).as_str()
        ));
    }

    #[test]
    fn each_template_becomes_a_file_and_a_manifest_entry() {
        let bundle = bundle(vec![(
            "Templates",
            vec![
                vec![
                    text("name"),
                    text("alias"),
                    text("type"),
                    text("template.document"),
                ],
                vec![
                    text("Voter Credentials"),
                    text("voter_credentials"),
                    text("VOTER_CREDENTIALS"),
                    text("Hello {{name}}"),
                ],
            ],
        )]);
        let layout = layout(&bundle);

        let document = layout
            .auxiliary
            .iter()
            .find(|a| a.name == "templates/voter_credentials.hbs")
            .expect("the template file");
        assert_eq!(document.bytes, b"Hello {{name}}");

        let manifest = layout
            .auxiliary
            .iter()
            .find(|a| a.name == "templates/templates.json")
            .expect("the manifest");
        let parsed: Value =
            serde_json::from_slice(&manifest.bytes).expect("valid JSON");
        assert_eq!(parsed[0]["alias"], json!("voter_credentials"));
        assert_eq!(parsed[0]["file"], json!("voter_credentials.hbs"));
        assert_eq!(parsed[0]["type"], json!("VOTER_CREDENTIALS"));
    }

    #[test]
    fn the_realm_patch_says_it_was_not_applied_when_there_was_no_realm() {
        // Whoever reads it has to know whether to apply it by hand.
        let bundle = bundle(vec![]);
        let layout = layout(&bundle);
        let patch = layout
            .auxiliary
            .iter()
            .find(|a| a.name == "keycloak_event_realm_patch.json")
            .expect("the realm patch");
        let parsed: Value = serde_json::from_slice(&patch.bytes).unwrap();
        assert!(parsed["_comment"].as_str().unwrap().contains("NOT applied"));
        assert_eq!(
            parsed["patch"]["displayName"],
            json!("Union Election 2027")
        );
    }

    #[test]
    fn the_realm_patch_states_the_directives_that_are_not_merges() {
        // A reader applying it by hand cannot deduce either from the patch itself.
        let bundle = bundle(vec![(
            "Parameters",
            vec![
                vec![text("type"), text("key"), text("value")],
                vec![
                    text("settings"),
                    text("auth_type"),
                    text("voter_link_plus_dob"),
                ],
            ],
        )]);
        let layout = layout(&bundle);
        let patch = layout
            .auxiliary
            .iter()
            .find(|a| a.name == "keycloak_event_realm_patch.json")
            .expect("the realm patch");
        let parsed: Value = serde_json::from_slice(&patch.bytes).unwrap();

        assert_eq!(parsed["auth_preset"], json!("voter_link_plus_dob"));
        assert_eq!(
            parsed["bind_authenticator_config"]["authenticator"],
            json!("message-otp-authenticator")
        );
        assert!(parsed["user_profile"]["attributes"]["dateOfBirth"].is_object());
        // And nothing internal leaked into the mergeable part.
        for key in parsed["patch"].as_object().unwrap().keys() {
            assert!(!key.starts_with('_'), "{key} leaked into the patch");
        }
    }

    #[test]
    fn a_bundle_with_nothing_extra_writes_no_auxiliary_files_but_the_realm_patch(
    ) {
        let layout = layout(&bundle(vec![]));
        assert_eq!(
            names(&layout.auxiliary),
            ["keycloak_event_realm_patch.json"]
        );
    }

    #[test]
    fn every_json_artifact_parses_and_ends_in_a_newline() {
        // A file without one is a nuisance in every diff it appears in.
        let bundle = bundle(vec![(
            "Templates",
            vec![
                vec![text("alias"), text("template.document")],
                vec![text("otp"), text("hello")],
            ],
        )]);
        let layout = layout(&bundle);
        for artifact in layout.importable.iter().chain(layout.auxiliary.iter())
        {
            if artifact.name.ends_with(".json") {
                serde_json::from_slice::<Value>(&artifact.bytes)
                    .unwrap_or_else(|error| {
                        panic!("{}: {error}", artifact.name)
                    });
                assert_eq!(
                    artifact.bytes.last(),
                    Some(&b'\n'),
                    "{}",
                    artifact.name
                );
            }
        }
    }

    #[cfg(feature = "election_config_archive")]
    #[test]
    fn the_archive_holds_exactly_the_importable_members_at_its_root() {
        use std::io::Cursor;

        let bundle = bundle(vec![(
            "Admin Users",
            vec![vec![text("username")], vec![text("admin1")]],
        )]);
        let layout = layout(&bundle);
        let bytes = zip(&layout.importable).expect("a zip");

        let mut archive =
            ::zip::ZipArchive::new(Cursor::new(bytes)).expect("readable");
        let mut inside: Vec<String> = (0..archive.len())
            .map(|index| archive.by_index(index).unwrap().name().to_string())
            .collect();
        inside.sort();
        let mut expected: Vec<String> =
            layout.importable.iter().map(|a| a.name.clone()).collect();
        expected.sort();
        assert_eq!(inside, expected);

        // No directories, and the administrators are not in there.
        assert!(inside.iter().all(|name| !name.contains('/')));
    }

    #[cfg(feature = "election_config_archive")]
    #[test]
    fn zipping_the_same_bundle_twice_gives_the_same_bytes() {
        // The point of the fixed timestamp and mode: without them "regenerating
        // produced no diff" is not something anyone can check.
        let layout = layout(&bundle(vec![]));
        assert_eq!(
            zip(&layout.importable).unwrap(),
            zip(&layout.importable).unwrap()
        );
    }

    #[cfg(feature = "election_config_archive")]
    #[test]
    fn a_member_survives_the_round_trip_byte_for_byte() {
        use std::io::Read;

        let layout = layout(&bundle(vec![]));
        let bytes = zip(&layout.importable).unwrap();
        let mut archive =
            ::zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();

        for expected in &layout.importable {
            let mut member = archive.by_name(&expected.name).unwrap();
            let mut got = Vec::new();
            member.read_to_end(&mut got).unwrap();
            assert_eq!(got, expected.bytes, "{}", expected.name);
        }
    }
}

/// The name of the importable zip *inside* the delivery zip.
///
/// `election_architect`'s own name for it, kept so a client who has been handed one of
/// these before finds what they expect, and so the two tools' output is one format
/// rather than two.
pub const IMPORTABLE_MEMBER: &str = "official_election_setup.zip";

/// The reopenable plan inside a delivery, named once so `delivery` and
/// `plan_in_delivery` cannot disagree about it.
pub const PLAN_MEMBER: &str = "blueprint.json";

/// Everything the wizard hands over: one zip that is **not** importable, holding one
/// that is.
///
/// The shape is `election_architect`'s. A delivery contains material a person needs and
/// the Admin Portal must never see — the reopenable plan, the points of contact, the
/// trustee list and threshold, the ceremony dates — beside a nested zip that is exactly
/// what the importer takes. Handing the importable zip over on its own loses all of
/// that; handing the loose files over as separate downloads, which is what this did
/// before, leaves somebody to work out which single file goes to the importer, and one
/// of the others can carry administrator passwords.
///
/// Nesting is what makes that unambiguous: the only thing that can be imported is the
/// only thing that looks like an import, and the outer zip is refused by the Admin
/// Portal rather than half-accepted.
#[cfg(feature = "election_config_archive")]
pub fn delivery(
    layout: &Layout,
) -> Result<Artifact, crate::election_config::Problem> {
    let importable = zip(&layout.importable)?;

    let mut members = Vec::with_capacity(layout.auxiliary.len() + 1);
    members.push(Artifact {
        name: IMPORTABLE_MEMBER.to_string(),
        bytes: importable,
    });
    members.extend(layout.auxiliary.iter().cloned());

    Ok(Artifact {
        name: layout.archive_name.clone(),
        bytes: zip(&members)?,
    })
}

/// The plan inside a delivery zip, or the reason it is not there.
///
/// The other half of [`delivery`]. `Import Configuration` is handed whatever a client
/// kept, and what a client keeps is the whole delivery — so the wizard has to open it and
/// find `blueprint.json`, rather than asking somebody to unzip it first and pick the right
/// file out of eight.
///
/// Here rather than in TypeScript, and the reason is the same one that put `delivery`
/// here: the layout would then exist in two places and drift. `zip` is already a
/// dependency of this crate, and the round trip — written by `delivery`, read by this —
/// is testable in Rust where both ends are.
///
/// Deliberately narrow. It returns the plan's bytes and nothing else: whether they
/// deserialize into a `Blueprint` is `validate_plan`'s business, and a caller that
/// already has a bare `blueprint.json` should not come through here at all.
#[cfg(feature = "election_config_archive")]
pub fn plan_in_delivery(
    bytes: &[u8],
) -> Result<Vec<u8>, crate::election_config::Problem> {
    use crate::election_config::problem::Code;
    use std::io::Read;

    let refused = |message: String| {
        crate::election_config::Problem::error(Code::InvalidValue, "delivery", message)
    };

    let mut outer = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|error| {
        refused(format!(
            "this is not a configuration: it could not be opened as a zip ({error})"
        ))
    })?;

    // The names first, so the borrow of `outer` for reading does not overlap the borrow
    // for listing. Cheap either way, and it means the failure can say what *was* in the
    // zip — which is the difference between fixing it and guessing.
    let names: Vec<String> = (0..outer.len())
        .filter_map(|at| outer.by_index(at).ok().map(|entry| entry.name().to_string()))
        .collect();

    if !names.iter().any(|name| name == PLAN_MEMBER) {
        return Err(refused(format!(
            "this zip has no {PLAN_MEMBER}, so there is no plan in it to reopen. It \
             contains: {}",
            if names.is_empty() {
                "nothing".to_string()
            } else {
                names.join(", ")
            }
        )));
    }

    let mut plan = Vec::new();
    outer
        .by_name(PLAN_MEMBER)
        .map_err(|error| refused(format!("{PLAN_MEMBER} could not be opened ({error})")))?
        .read_to_end(&mut plan)
        .map_err(|error| refused(format!("{PLAN_MEMBER} could not be read ({error})")))?;

    Ok(plan)
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Every route that commits a database transaction logs what it changed
//! (SW-F-0364): it names its admin to the `audit_change` trigger
//! (`audit::act_for`) or stages its own entry. A route that changes nothing
//! an admin would audit is listed, with the reason, in
//! `tests/fixtures/unaudited-committing-routes.txt`.

use std::collections::BTreeMap;
use std::path::Path;

/// Naming the admin to the trigger, handing them to the Windmill code that
/// opens the transaction, or staging an entry of its own.
const AUDITS: [&str; 4] = [
    "audit::act_for(",
    "audit::actor(",
    "audit::record(",
    "stage(",
];

fn exemptions() -> BTreeMap<String, String> {
    include_str!("../fixtures/unaudited-committing-routes.txt")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (file, reason) = line
                .split_once(':')
                .unwrap_or_else(|| panic!("no reason given: {line}"));
            assert!(!reason.trim().is_empty(), "no reason given: {line}");
            (file.trim().to_string(), reason.trim().to_string())
        })
        .collect()
}

/// The setup login cannot insert outbox rows directly. Its two committing
/// functions enforce the specific local overlay trigger in the same tx.
fn local_setup_audit(source: &str) -> bool {
    let sql = include_str!(
        "../../../../hasura/felix3-local-initialize/1794700002280_setup_browser_audit/up.sql"
    );
    let trigger = "local_setup_browser_audit";
    if !source.contains(trigger)
        || !source.contains("t.tgenabled='A'")
        || !sql.contains("INSERT INTO sequent_backend.audit_log_outbox")
        || !sql.contains("AFTER UPDATE OF portal_session_sha256, delivery_revision")
        || !sql.contains("ENABLE ALWAYS TRIGGER local_setup_browser_audit")
        || !sql.contains("REVOKE ALL ON FUNCTION sequent_backend.audit_local_setup_browser_transition() FROM PUBLIC")
    {
        return false;
    }
    let begins = ["pub(crate) async fn begin(", "pub(crate) async fn release("];
    source.matches(".commit()").count() == begins.len()
        && begins
            .into_iter()
            .zip(["bind_browser", "take_delivery"])
            .all(|(start, transition)| {
                let Some((_, route)) = source.split_once(start) else {
                    return false;
                };
                let Some((route, _)) = route.split_once("\n}\n") else {
                    return false;
                };
                let Some((_, after)) = route.split_once(transition) else {
                    return false;
                };
                let Some((before_commit, _)) = after.split_once(".commit()")
                else {
                    return false;
                };
                before_commit
                    .contains("require_setup_browser_audit(&tx).await?")
            })
}

#[test]
fn local_setup_rejects_an_unchecked_guard_or_an_extra_commit() {
    let source =
        include_str!("../../src/routes/local_initialize_self_setup.rs");
    assert!(local_setup_audit(source));
    let without_guard =
        source.replace("require_setup_browser_audit(&tx).await?", "");
    assert!(!local_setup_audit(&without_guard));
    let extra_commit = format!("{source}\nother_tx.commit()");
    assert!(!local_setup_audit(&extra_commit));
}

/// The route files that commit a transaction, with whether they audit.
fn committing_routes() -> BTreeMap<String, bool> {
    let routes = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/routes");
    std::fs::read_dir(routes)
        .expect("routes directory")
        .map(|entry| entry.expect("route file").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .filter_map(|path| {
            let source = std::fs::read_to_string(&path).expect("route source");
            source.contains(".commit()").then(|| {
                let name = path.file_name().unwrap().to_string_lossy();
                let audits = AUDITS.iter().any(|audit| source.contains(audit))
                    || (name == "local_initialize_self_setup.rs"
                        && local_setup_audit(&source));
                (name.to_string(), audits)
            })
        })
        .collect()
}

#[test]
fn every_route_that_commits_logs_its_changes() {
    let exempt = exemptions();
    let routes = committing_routes();
    let unaudited: Vec<&String> = routes
        .iter()
        .filter(|(file, audits)| !**audits && !exempt.contains_key(*file))
        .map(|(file, _)| file)
        .collect();
    assert!(
        unaudited.is_empty(),
        "these routes commit changes without audit::act_for or an entry \
         of their own: {unaudited:?}"
    );
}

#[test]
fn every_exemption_names_a_committing_route_without_an_audit() {
    let routes = committing_routes();
    let stale: Vec<String> = exemptions()
        .into_keys()
        .filter(|file| routes.get(file) != Some(&false))
        .collect();
    assert!(stale.is_empty(), "stale exemptions: {stale:?}");
}

/// The Initialize ceremony's attempt table has no `audit_change` trigger,
/// so the file-level check above is not enough for its routes: each ledger
/// function that changes an attempt is followed, in the same route and
/// before its commit, by an entry the route stages itself.
#[test]
fn each_initialize_attempt_change_stages_its_entry_before_committing() {
    let source = include_str!("../../src/routes/operational_initialize.rs");
    for function in [
        "operational_initialize_discard_unredeemed(",
        "operational_initialize_record_browser_return(",
    ] {
        let calls: Vec<usize> =
            source.match_indices(function).map(|(at, _)| at).collect();
        assert_eq!(calls.len(), 1, "{function} is called once");
        // What is left of the route after the call.
        let rest = &source[calls[0]..];
        let rest = &rest[..rest.find("\n}\n").expect("the route ends")];
        let staged = rest
            .find("audit_log::stage(")
            .unwrap_or_else(|| panic!("{function} stages no audit entry"));
        let committed = rest
            .find(".commit()")
            .unwrap_or_else(|| panic!("{function} is never committed"));
        assert!(
            staged < committed,
            "{function} commits before its audit entry is staged"
        );
    }
}

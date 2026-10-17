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
                (
                    name.to_string(),
                    AUDITS.iter().any(|audit| source.contains(audit)),
                )
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

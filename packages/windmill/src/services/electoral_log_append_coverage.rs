// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Every production append to an electoral log, classified for the logs' order.
//!
//! The invariant (`services::electoral_log_deferral`): an entry of a scope is
//! appended only when admitted for its log's order (no entry of the scope
//! waits, no batch is processing it), except by the batch task, which drains
//! the waiting entries in order under the scope's lease. The test below lists
//! every call, in windmill's and harvest's production sources, of the electoral
//! log's append sinks (`append`, `append_in`, `append_iter`, `import_logs`,
//! `insert_records`) and fails on one that is not classified here, and on a
//! classification that no longer matches a call, so a new append path cannot
//! appear unclassified.

#[cfg(test)]
mod tests {
    use regex::Regex;
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::{Path, PathBuf};

    /// (file, enclosing function) -> (how it keeps the order, a text the file
    /// must contain that implements it).
    fn classified() -> BTreeMap<(&'static str, &'static str), (&'static str, &'static str)> {
        BTreeMap::from([
            (("windmill/src/services/electoral_log.rs", "append_admitted"),
                ("direct posts: admitted (ScopeAdmission) in append_gated / post_built*, signed only once admitted", "ScopeAdmission::admit")),
            (("windmill/src/services/electoral_log.rs", "import_from_csv"),
                ("restore of already-signed records: admitted, held across the append, nothing signed", "ScopeAdmission::admit")),
            (("windmill/src/services/electoral_log_transfer.rs", "import_electoral_log"),
                ("restore of already-signed records: admitted, held across the import, nothing signed", "ScopeAdmission::admit")),
            (("windmill/src/services/audit_log_writer.rs", "post_target"),
                ("audit-log outbox: admitted before signing; refused = Gated, no failure counted", "ScopeAdmission::admit")),
            (("windmill/src/services/ballot_box.rs", "append_pending"),
                ("ballot-box sequencer: each round admitted before signing; refused = nothing signed, items stay pending", "ScopeAdmission::admit")),
            (("windmill/src/services/electoral_log_outbox.rs", "forward_announcement"),
                ("relay's pre-signed announcement: admitted on the tenant scope, released before write_waiting; refused = not a failure", "ScopeAdmission::admit")),
            (("windmill/src/services/signing/log.rs", "append"),
                ("signing-log outbox store: only called by deliver, under SigningLogBoard::admit", "ScopeAdmission::admit")),
            (("windmill/src/services/signing/log.rs", "deliver"),
                ("signing-log outbox: post_outbox_snapshot admits before signing; refused = gated, no failure counted", "board.admit(&scope)")),
            (("windmill/src/services/ballot_box_seal/seal.rs", "seal_prepared"),
                ("ballot-box seal: admitted before the box lock and signing; refused = pending (busy)", "ScopeAdmission::admit")),
            (("windmill/src/services/ballot_box_seal/seal.rs", "fail"),
                ("seal's failure record: only called inside seal_prepared, under its admission", "ScopeAdmission::admit")),
            (("windmill/src/services/cast_vote_seal/mod.rs", "pin_key"),
                ("cast vote seal key pin (manual or verified): event-scope admission held before signing and through append", "ScopeAdmission::admit(&scope)")),
            (("windmill/src/services/ceremonies/sealed_box_ballots.rs", "deliver_once"),
                ("tally's sealed-box records: admitted before signing (post_verified / post_rejected)", "ScopeAdmission::admit")),
            (("windmill/src/tasks/electoral_log.rs", "store_batch_leased"),
                ("the batch task (drainer): under its scopes' leases, stores waiting entries in first-wait order", "take_leases")),
        ])
    }

    fn packages() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                sources(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && !path.to_string_lossy().ends_with("_tests.rs")
            {
                out.push(path);
            }
        }
    }

    /// The source without its `#[cfg(test)] mod ... { }` blocks (lines kept).
    fn without_test_modules(src: &str) -> String {
        let start = Regex::new(
            r"#\[cfg\(test\)\]\s*(?:#\[[^\]]*\]\s*)*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{",
        )
        .unwrap();
        let mut out = String::new();
        let mut at = 0;
        for found in start.find_iter(src) {
            if found.start() < at {
                continue;
            }
            let bytes = src.as_bytes();
            let (mut end, mut depth) = (found.end(), 1);
            while depth > 0 && end < bytes.len() {
                match bytes[end] {
                    b'{' => depth += 1,
                    b'}' => depth -= 1,
                    _ => {}
                }
                end += 1;
            }
            out.push_str(&src[at..found.start()]);
            out.push_str(&"\n".repeat(src[found.start()..end].matches('\n').count()));
            at = end;
        }
        out.push_str(&src[at..]);
        out
    }

    #[test]
    fn every_production_append_to_an_electoral_log_is_classified_for_its_order() {
        let sink = Regex::new(
            r"(?:\.append|\bappend_in|\.append_iter|\.import_logs|\binsert_records)\s*\(",
        )
        .unwrap();
        // Not the electoral log: `Vec::append(&mut ..)`, `OpenOptions::append(bool)`.
        let other = Regex::new(r"^\.append\(\s*(&mut|true|false)").unwrap();
        let function = Regex::new(r"\bfn\s+(\w+)").unwrap();
        let root = packages();
        let mut found: BTreeSet<(String, String)> = BTreeSet::new();
        for crate_dir in ["windmill/src", "harvest/src"] {
            let mut files = Vec::new();
            sources(&root.join(crate_dir), &mut files);
            for file in files {
                let src = without_test_modules(&std::fs::read_to_string(&file).unwrap());
                let name = file
                    .strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .to_string();
                if name.ends_with("electoral_log_append_coverage.rs") {
                    continue;
                }
                for call in sink.find_iter(&src) {
                    if other.is_match(&src[call.start()..]) {
                        continue;
                    }
                    let enclosing = function
                        .captures_iter(&src[..call.start()])
                        .last()
                        .map(|captures| captures[1].to_string())
                        .unwrap_or_default();
                    found.insert((name.clone(), enclosing));
                }
            }
        }
        let classified = classified();
        let unclassified: Vec<_> = found
            .iter()
            .filter(|(file, function)| {
                !classified.contains_key(&(file.as_str(), function.as_str()))
            })
            .collect();
        assert!(
            unclassified.is_empty(),
            "appends to an electoral log not classified for its order: {unclassified:?}"
        );
        for ((file, function), (how, marker)) in &classified {
            assert!(
                found.contains(&(file.to_string(), function.to_string())),
                "stale classification (no such append any more): {file} {function}: {how}"
            );
            let src = std::fs::read_to_string(root.join(file)).unwrap();
            assert!(
                src.contains(marker),
                "{file} {function} is classified as \"{how}\" but has no `{marker}`"
            );
        }
    }
}

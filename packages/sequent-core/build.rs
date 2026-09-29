// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Lists the monitoring presets, so a preset is a directory under
//! `src/monitoring/presets/` and adding one needs no code.

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

const PRESETS_DIR: &str = "src/monitoring/presets";

/// The directories a preset keeps its documents in, and their kind.
const KINDS: [(&str, &str); 3] = [
    ("themes", "Theme"),
    ("widgets", "Widget"),
    ("dashboards", "Dashboard"),
];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var_os("CARGO_FEATURE_MONITORING").is_none() {
        return;
    }
    println!("cargo:rerun-if-changed={PRESETS_DIR}");
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("cargo sets it"));
    let root = manifest_dir.join(PRESETS_DIR);
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets it"))
        .join("monitoring_presets.rs");
    fs::write(&out, registry(&root))
        .unwrap_or_else(|why| panic!("{}: {why}", out.display()));
}

/// `&[PresetSource { .. }, ..]`, one per preset directory, in id order.
fn registry(root: &Path) -> String {
    let mut code = String::from("&[\n");
    for preset in sorted(root) {
        let id = name(&preset);
        assert!(
            preset.is_dir(),
            "monitoring presets: '{id}' is not a preset; each preset is a directory under {PRESETS_DIR}/"
        );
        assert!(
            !id.is_empty()
                && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "monitoring preset '{id}': a preset's directory is named with lower-case letters, digits and dashes"
        );
        let mut files = Vec::new();
        for entry in sorted(&preset) {
            let file = name(&entry);
            match (entry.is_dir(), file.as_str()) {
                (false, "preset.yaml") => {}
                (false, "settings.yaml") => {
                    files.push(("Settings", file.clone(), entry.clone()))
                }
                (true, directory) => {
                    let Some((_, kind)) =
                        KINDS.iter().find(|(name, _)| *name == directory)
                    else {
                        panic!("monitoring preset '{id}': '{directory}/' is not a place for documents; use themes/, widgets/ or dashboards/");
                    };
                    for document in sorted(&entry) {
                        let document_name = name(&document);
                        assert!(
                            document.is_file() && document_name.ends_with(".yaml"),
                            "monitoring preset '{id}': {directory}/{document_name} is not a .yaml document"
                        );
                        files.push((kind, format!("{directory}/{document_name}"), document));
                    }
                }
                (false, other) => panic!(
                    "monitoring preset '{id}': '{other}' is not a preset file; a preset has preset.yaml, settings.yaml, themes/, widgets/ and dashboards/"
                ),
            }
        }
        let manifest = preset.join("preset.yaml");
        assert!(
            manifest.is_file(),
            "monitoring preset '{id}' has no preset.yaml"
        );
        writeln!(
            code,
            "    PresetSource {{\n        id: {id:?},\n        manifest: include_str!({:?}),\n        files: &[",
            manifest.display().to_string()
        )
        .expect("writes to a string");
        for (kind, path, file) in files {
            writeln!(
                code,
                "            PresetFile {{ kind: ConfigKind::{kind}, path: {path:?}, yaml: include_str!({:?}) }},",
                file.display().to_string()
            )
            .expect("writes to a string");
        }
        code.push_str("        ],\n    },\n");
    }
    code.push(']');
    code
}

fn sorted(directory: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|why| panic!("{}: {why}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        // Editors' and systems' hidden files are not documents.
        .filter(|path| !name(path).starts_with('.'))
        .collect();
    entries.sort();
    entries
}

fn name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

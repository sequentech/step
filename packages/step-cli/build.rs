// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Bundle the load runtime with the CLI so installed binaries need no source checkout.
use std::{env, fs, path::Path};

fn collect(directory: &Path, root: &Path, entries: &mut Vec<String>) {
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut paths: Vec<_> = fs::read_dir(directory)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    for path in paths {
        if path.is_dir() {
            if path.file_name().unwrap() == "fixtures" {
                collect(&path, root, entries);
            }
        } else if matches!(
            path.extension().and_then(|x| x.to_str()),
            Some("js" | "json" | "html" | "css" | "rs" | "toml" | "lock")
        ) || path.file_name().unwrap() == "Dockerfile"
        {
            let relative = path.strip_prefix(root).unwrap().to_str().unwrap();
            entries.push(format!(
                "({relative:?}, include_bytes!({:?})),",
                path.to_str().unwrap()
            ));
        }
    }
}

fn main() {
    let manifest = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.parent().unwrap().parent().unwrap();
    let mut entries = Vec::new();
    collect(&root.join("packages/voting-load"), root, &mut entries);
    for relative in [
        "packages/step-cli/src/load/config.rs",
        "packages/step-cli/src/load/files.rs",
        "packages/step-cli/src/load/input.rs",
        "packages/step-cli/src/load/worker.rs",
        "packages/voting-portal/src/queries/GetVoterStatus.ts",
        "packages/voting-portal/src/queries/InsertCastVote.ts",
        "packages/admin-portal/public/roboto/Roboto_latin_400.woff2",
        "packages/admin-portal/public/roboto/Roboto_latin_700.woff2",
        "LICENSES/Apache-2.0.txt",
        "packages/voting-portal/playwright.scale.config.ts",
        "packages/voting-portal/test/load/scale.spec.ts",
        "packages/voting-portal/test/load/flow.ts",
    ] {
        let path = root.join(relative);
        println!("cargo:rerun-if-changed={}", path.display());
        entries.push(format!(
            "({relative:?}, include_bytes!({:?})),",
            path.to_str().unwrap()
        ));
    }
    fs::write(
        std::path::PathBuf::from(env::var("OUT_DIR").unwrap()).join("load_assets.rs"),
        format!(
            "const ASSETS: &[(&str, &[u8])] = &[{}];",
            entries.join("\n")
        ),
    )
    .unwrap();
}

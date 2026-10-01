// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Bundle the load runtime with the CLI so installed binaries need no source checkout.
use std::{env, fs, path::Path};

fn collect(
    directory: &Path,
    root: &Path,
    entries: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut paths: Vec<_> = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<_, _>>()?;
    paths.sort();
    for path in paths {
        if path.is_dir() {
            if path.file_name().ok_or("Asset has no file name")? == "fixtures" {
                collect(&path, root, entries)?;
            }
        } else if matches!(
            path.extension().and_then(|x| x.to_str()),
            Some("js" | "json" | "html" | "css" | "rs" | "toml" | "lock")
        ) || path.file_name().ok_or("Asset has no file name")? == "Dockerfile"
        {
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .ok_or("Asset path must be UTF-8")?;
            entries.push(format!(
                "({relative:?}, include_bytes!({:?})),",
                path.to_str().ok_or("Asset path must be UTF-8")?
            ));
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manifest = std::path::PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let root = manifest
        .parent()
        .and_then(Path::parent)
        .ok_or("Missing repository root")?;
    let mut entries = Vec::new();
    collect(&root.join("packages/voting-load"), root, &mut entries)?;
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
            path.to_str().ok_or("Asset path must be UTF-8")?
        ));
    }
    fs::write(
        std::path::PathBuf::from(env::var("OUT_DIR")?).join("load_assets.rs"),
        format!(
            "const ASSETS: &[(&str, &[u8])] = &[{}];",
            entries.join("\n")
        ),
    )?;
    Ok(())
}

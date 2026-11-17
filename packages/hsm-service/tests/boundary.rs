// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Only this service may reach a key device: no other package of the
//! workspace may depend on a PKCS#11, YubiHSM or KMS library. Callers use
//! the transport-only `hsm-client`.

use std::path::Path;

const DEVICE_CRATES: [&str; 5] = [
    "cryptoki",
    "cryptoki-sys",
    "yubihsm",
    "aws-sdk-kms",
    "pkcs11",
];

fn manifests(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if matches!(
                name.as_str(),
                "node_modules" | "target" | "rust-local-target" | ".git" | "hsm-service"
            ) {
                continue;
            }
            manifests(&path, out);
        } else if name == "Cargo.toml" {
            out.push(path);
        }
    }
}

#[test]
fn no_other_package_links_a_key_device() {
    let packages = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut found = Vec::new();
    manifests(packages, &mut found);
    assert!(found.len() > 10, "the workspace's manifests were not found");
    let mut offenders = Vec::new();
    for manifest in found {
        let text = std::fs::read_to_string(&manifest).unwrap();
        for line in text.lines() {
            let key = line
                .split('=')
                .next()
                .unwrap_or_default()
                .trim()
                .trim_matches('"');
            if DEVICE_CRATES.contains(&key) {
                offenders.push(format!("{}: {}", manifest.display(), line.trim()));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "only hsm-service may reach a key device: {offenders:#?}"
    );
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const STORE_DIRECTORY: &str = "message_store";

fn trustee(directory: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_trustee"))
        .current_dir(directory)
        .env_remove("TRUSTEE_CONFIG_PATH")
        .args(args)
        .output()
        .unwrap()
}

/// A keys file and a session store where `trustee start` would keep them,
/// and what `trustee generate` printed.
fn trustee_files(directory: &Path) -> String {
    let generated =
        trustee(directory, &["generate", "--trustee-config", "trustee.toml"]);
    assert!(generated.status.success());
    let stores = directory.join(STORE_DIRECTORY);
    fs::create_dir(&stores).unwrap();
    fs::write(stores.join("board-1.sqlite"), [0x5a; 4096]).unwrap();
    String::from_utf8(generated.stdout).unwrap()
}

#[test]
fn destroy_removes_the_keys_file_and_the_stores_under_the_working_directory() {
    let directory = tempfile::tempdir().unwrap();
    let public_keys = trustee_files(directory.path());

    let destroyed = trustee(
        directory.path(),
        &["destroy", "--trustee-config", "trustee.toml"],
    );

    let stderr = String::from_utf8_lossy(&destroyed.stderr);
    assert!(destroyed.status.success(), "{stderr}");
    assert!(!directory.path().join("trustee.toml").exists());
    assert!(!directory.path().join(STORE_DIRECTORY).exists());
    let report = String::from_utf8(destroyed.stdout).unwrap();
    let mut lines = report.lines();
    let first = lines.next().unwrap();
    assert!(
        first.starts_with("destroyed the keys file trustee.toml (")
            && first.ends_with(" bytes) of"),
        "{report}"
    );
    // The public keys of the keys file destroyed are those it was made with.
    assert!(report.contains(&public_keys), "{report}");
    let stores = directory.path().join(STORE_DIRECTORY);
    assert!(report.ends_with(&format!(
        "destroyed the session store {} (4096 bytes)\n\
         removed the session store directory {}\n",
        stores.join("board-1.sqlite").display(),
        stores.display()
    )));
}

#[test]
fn destroy_reads_the_keys_file_from_the_variable_start_reads() {
    let directory = tempfile::tempdir().unwrap();
    trustee_files(directory.path());

    let destroyed = Command::new(env!("CARGO_BIN_EXE_trustee"))
        .current_dir(directory.path())
        .env("TRUSTEE_CONFIG_PATH", "trustee.toml")
        .arg("destroy")
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&destroyed.stderr);
    assert!(destroyed.status.success(), "{stderr}");
    assert!(!directory.path().join("trustee.toml").exists());
}

#[test]
fn destroy_without_a_keys_file_removes_nothing() {
    let directory = tempfile::tempdir().unwrap();
    trustee_files(directory.path());
    fs::remove_file(directory.path().join("trustee.toml")).unwrap();

    let unnamed = trustee(directory.path(), &["destroy"]);
    assert_eq!(unnamed.status.code(), Some(1));
    let missing = trustee(
        directory.path(),
        &["destroy", "--trustee-config", "trustee.toml"],
    );
    assert_eq!(missing.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("nothing was removed"), "{stderr}");

    let store = directory
        .path()
        .join(STORE_DIRECTORY)
        .join("board-1.sqlite");
    assert_eq!(fs::read(store).unwrap(), [0x5a; 4096]);
}

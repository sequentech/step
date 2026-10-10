// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! The voter CSV is written through a generic writer, so its layout is checked
//! in memory. File naming and ordering are checked in a private directory.

use super::*;
use rand::{rngs::StdRng, SeedableRng};
use serde_json::json;
use std::fs;
use std::io;

fn external_config(fields: &[&str], excluded_columns: &[&str]) -> Value {
    json!({
        "election_event_json_file": "event.json",
        "realm_name": "synthetic-realm",
        "tenant_id": "synthetic-tenant",
        "election_event_id": "synthetic-event",
        "area_id": "synthetic-area",
        "election_id": "synthetic-election",
        "generate_voters": {
            "csv_file_name": "voters",
            "fields": fields,
            "excluded_columns": excluded_columns,
            "email_prefix": "synthetic",
            "domain": "example.test",
            "sequence_email_number": true,
            "sequence_start_number": 0,
            "username_start_number": 100,
            "voter_password": "synthetic-password",
            "password_salt": "synthetic-salt",
            "hashed_password": "synthetic-hash",
            "overseas_reference": "B",
            "min_age": 18,
            "max_age": 90,
            "authorized_elections_count": 0,
            "email_verified": true
        },
        "duplicate_votes": {"row_id_to_clone": "synthetic-row"},
        "generate_applications": {"applicant_data": {}, "annotations": {}}
    })
}

fn voters_config(fields: &[&str], excluded_columns: &[&str]) -> VotersConfig {
    serde_json::from_value(external_config(fields, excluded_columns)["generate_voters"].clone())
        .unwrap()
}

fn event() -> Value {
    json!({
        "areas": [{"id": "area-north", "name": "North"}, {"id": "area-south", "name": "South"}],
        "area_contests": [{"area_id": "area-north", "contest_id": "contest-mayor"}],
        "contests": [{"id": "contest-mayor", "election_id": "election-local"}],
        "elections": [{"id": "election-local"}]
    })
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 9, 24).unwrap()
}

fn write_to_memory(cfg: &VotersConfig, num_users: usize) -> String {
    let mut writer = Writer::from_writer(Vec::new());
    write_voters(
        &mut writer,
        &ElectionIndex::from_event(&event()),
        cfg,
        num_users,
        &mut StdRng::seed_from_u64(1),
        today,
    )
    .unwrap();
    String::from_utf8(writer.into_inner().unwrap()).unwrap()
}

struct Workspace {
    directory: tempfile::TempDir,
    command: GenerateVoters,
}

impl Workspace {
    fn new(num_users: usize, event: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            directory.path().join("external_config.json"),
            external_config(&["username", "area_name"], &[]).to_string(),
        )
        .unwrap();
        fs::write(directory.path().join("event.json"), event).unwrap();
        let command = GenerateVoters {
            working_directory: directory.path().to_str().unwrap().into(),
            num_users,
        };
        Self { directory, command }
    }

    fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.command
            .run_generate_voters(&self.command.working_directory, self.command.num_users)
    }

    fn files(&self) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(self.directory.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

#[test]
fn the_header_lists_the_configured_columns_in_order_without_excluded_ones() {
    let cfg = voters_config(
        &[
            "email",
            "password",
            "username",
            "area_name",
            "nickname",
            "hashed_password",
        ],
        &["password", "hashed_password", "not-a-field"],
    );
    assert_eq!(
        write_to_memory(&cfg, 0),
        "email,username,area_name,nickname\n"
    );
}

#[test]
fn each_record_follows_the_header_order() {
    let cfg = voters_config(
        &["area_name", "username", "email", "password"],
        &["password"],
    );
    assert_eq!(
        write_to_memory(&cfg, 3),
        "area_name,username,email\n\
         North,100,synthetic+0@example.test\n\
         South,101,synthetic+1@example.test\n\
         North,102,synthetic+2@example.test\n"
    );
}

#[test]
fn voters_are_written_to_a_csv_named_after_the_voter_count() {
    let workspace = Workspace::new(2, &event().to_string());
    workspace.run().unwrap();
    assert_eq!(
        workspace.files(),
        ["event.json", "external_config.json", "voters_2.csv"]
    );
    assert_eq!(
        fs::read_to_string(workspace.directory.path().join("voters_2.csv")).unwrap(),
        "username,area_name\n100,North\n101,South\n"
    );
}

#[test]
fn an_unreadable_event_creates_no_csv() {
    let workspace = Workspace::new(2, "{broken");
    let error = workspace.run().unwrap_err();
    assert!(
        error.downcast_ref::<serde_json::Error>().is_some(),
        "{error}"
    );
    assert_eq!(workspace.files(), ["event.json", "external_config.json"]);
}

#[test]
fn a_failed_write_is_returned_with_its_io_error() {
    struct FullDisk;
    impl Write for FullDisk {
        fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(
                io::ErrorKind::StorageFull,
                "synthetic full disk",
            ))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let cfg = voters_config(&["username"], &[]);
    let mut writer = Writer::from_writer(FullDisk);
    let error = write_voters(
        &mut writer,
        &ElectionIndex::from_event(&event()),
        &cfg,
        2,
        &mut StdRng::seed_from_u64(1),
        today,
    )
    .unwrap_err();
    // Small outputs stay in the CSV buffer until the final flush.
    let error = error.downcast_ref::<io::Error>().unwrap();
    assert_eq!(error.kind(), io::ErrorKind::StorageFull);
    assert_eq!(error.to_string(), "synthetic full disk");
}

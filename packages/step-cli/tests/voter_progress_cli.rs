// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Progress reports completed CSV records at each full reporting interval.
use serde_json::json;
use std::{
    fs,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[test]
fn voter_generation_reports_completed_rows_without_a_zero_or_offset_count() {
    for count in [1, 10000, 10001] {
        let dir = tempfile::tempdir().unwrap();
        let config = json!({
            "election_event_json_file":"event.json", "realm_name":"synthetic", "tenant_id":"tenant",
            "election_event_id":"event", "area_id":"area", "election_id":"election",
            "generate_voters":{"csv_file_name":"voters","fields":["username"],
                "excluded_columns":[],"email_prefix":"voter","domain":"example.test","sequence_email_number":true,
                "sequence_start_number":0,"voter_password":"test","password_salt":"salt","hashed_password":"hash",
                "overseas_reference":"B","min_age":18,"max_age":90,"authorized_elections_count":0,"email_verified":true},
            "duplicate_votes":{"row_id_to_clone":"row"},"generate_applications":{"applicant_data":{},"annotations":{}}
        });
        fs::write(dir.path().join("external_config.json"), config.to_string()).unwrap();
        fs::write(dir.path().join("event.json"), "{}").unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_step-cli"))
            .args([
                "step",
                "generate-voters",
                "--num-users",
                &count.to_string(),
                "--working-directory",
            ])
            .arg(dir.path())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(30);
        while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let timed_out = child.try_wait().unwrap().is_none();
        if timed_out {
            child.kill().unwrap();
        }
        let output = child.wait_with_output().unwrap();
        assert!(!timed_out, "bounded local generation timed out");
        assert!(output.status.success());
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        let progress: Vec<_> = stdout
            .lines()
            .filter(|line| line.starts_with("Generated "))
            .collect();
        let expected = if count >= 10000 {
            vec!["Generated 10000 users..."]
        } else {
            vec![]
        };
        assert_eq!(progress, expected, "count={count}");
        assert!(stdout.contains(&format!(
            "Successfully generated {count} users. CSV file created at:"
        )));
        let rows = fs::read_to_string(dir.path().join(format!("voters_{count}.csv"))).unwrap();
        let rows: Vec<_> = rows.lines().collect();
        assert_eq!(rows.len(), count + 1);
        assert_eq!(rows.first(), Some(&"username"));
        assert_eq!(rows[1], "0");
        assert_eq!(rows[rows.len() - 1], (count - 1).to_string());
    }
}

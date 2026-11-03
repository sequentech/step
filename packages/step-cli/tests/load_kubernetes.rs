// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use serde_json::json;
use std::{fs, os::unix::fs::PermissionsExt, process::Command};

#[test]
fn timed_out_jobs_stop_before_collection_and_failed_cancellation_keeps_the_pvc() {
    for cancellation_fails in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let inputs = root.join("run/inputs");
        fs::create_dir_all(&inputs).unwrap();
        let settings = json!({
            "target":{"tenant_id":"synthetic"},
            "workload":{"count":1},
            "execution":{"executor":"kubernetes","storage_class":"synthetic"}
        });
        fs::write(root.join("run/settings.yaml"), settings.to_string()).unwrap();
        fs::write(
            inputs.join("config.json"),
            json!({
                "settings":settings, "election_event_id":"event", "election_id":"election",
                "realm":"realm", "area_name":"area", "login_url":"http://localhost:3000/login",
                "style_id":null, "publication_version":null, "profile":{}, "cast_query":""
            })
            .to_string(),
        )
        .unwrap();
        let kubectl = root.join("kubectl");
        // A fake API boundary: collection fails unless worker termination finished.
        fs::write(
            &kubectl,
            r#"#!/bin/sh
set -eu
shift 2
case "$1" in
  create) cat >/dev/null ;;
  wait) case "$2" in job/*) exit 1 ;; esac ;;
  delete)
    case "$2" in
      job/*)
        [ "$3" = "--cascade=foreground" ]
        [ "$4" = "--wait=true" ]
        [ "$MOCK_CANCEL_FAIL" = "false" ] || exit 1
        touch "$MOCK_ROOT/stopped"
        ;;
    esac
    ;;
  cp)
    case "$2" in
      *:/load/.)
        [ -f "$MOCK_ROOT/stopped" ]
        touch "$MOCK_ROOT/collected"
        ;;
    esac
    ;;
  *) exit 9 ;;
esac
"#,
        )
        .unwrap();
        fs::set_permissions(&kubectl, fs::Permissions::from_mode(0o700)).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_step-cli"))
            .args(["load", "run"])
            .arg(root.join("run"))
            .env(
                "PATH",
                format!("{}:{}", root.display(), std::env::var("PATH").unwrap()),
            )
            .env("XDG_CACHE_HOME", root.join("cache"))
            .env("LOAD_PASSWORD", "synthetic")
            .env("MOCK_ROOT", root)
            .env("MOCK_CANCEL_FAIL", cancellation_fails.to_string())
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(
            root.join("collected").exists(),
            !cancellation_fails,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(root.join("stopped").exists(), !cancellation_fails);
        if cancellation_fails {
            assert!(String::from_utf8_lossy(&output.stderr).contains("may still be running"));
        }
        assert!(root.join("run/job.json").exists());
    }
}

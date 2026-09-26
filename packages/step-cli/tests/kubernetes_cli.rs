// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! The real coordinator must collect every completed index before reporting job failure.
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

#[test]
fn terminal_jobs_collect_partial_results_and_cleanup_before_returning() {
    for terminal in ["Complete", "Failed", "pending"] {
        let root = tempfile::tempdir().unwrap();
        let run = root.path().join("run");
        fs::create_dir_all(run.join("inputs")).unwrap();
        let settings = json!({"target":{"tenant_id":"00000000-0000-0000-0000-000000000001"},
            "workload":{"count":2,"shard_size":1},
            "execution":{"storage_class":"test-rwx","workers":2,"wait_timeout":"100ms"}});
        fs::write(run.join("settings.yaml"), settings.to_string()).unwrap();
        fs::write(run.join("inputs/config.json"), json!({"settings":settings,
            "election_event_id":"event", "election_id":"election", "realm":"realm", "area_name":"Area",
            "login_url":"http://localhost:3000/login", "style_id":null,"publication_version":null,
            "profile":null,"cast_query":""}).to_string()).unwrap();
        let shim = root.path().join("kubectl");
        // Only the external process boundary is faked. The production run, manifests,
        // status interpretation, result collection, cleanup and failure propagation execute.
        fs::write(&shim, r#"#!/usr/bin/python3
import json, os, pathlib, sys
args = sys.argv[3:]
root = pathlib.Path(os.environ['FAKE_ROOT'])
with (root/'calls.jsonl').open('a') as out: out.write(json.dumps(args)+'\n')
if args[0] == 'create':
    value = json.load(sys.stdin)
    if value['kind'] == 'Job': (root/'job.json').write_text(json.dumps(value))
elif args[0] == 'get':
    condition = os.environ['TERMINAL']
    print(json.dumps({'status':{'conditions':[{'type':condition,'status':'True'}]}}))
elif args[0] == 'wait' and args[1].startswith('job/'):
    sys.exit('Complete-only wait cannot observe a failed job')
elif args[0] == 'cp' and ':/load/.' in args[1]:
    pathlib.Path(args[2], 'collected.txt').write_text('completed index 1')
    for index in (range(2) if os.environ["TERMINAL"] == "Complete" else [1]):
        directory = pathlib.Path(args[2], 'results', f'{index:06}')
        directory.mkdir(parents=True)
        (directory/'exit.json').write_text('{"code":0}')
        (directory/'samples.jsonl').write_text(json.dumps({'index':index,'passed':True,'start':1000,'end':2000,'status_ms':1,'cast_ms':10,'receipt':f'receipt-{index}'})+'\n')
"#).unwrap();
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o700)).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_step-cli"));
        command
            .args(["load", "run"])
            .arg(&run)
            .args(["--executor", "kubernetes"])
            .env_clear()
            .env("PATH", root.path())
            .env("XDG_CACHE_HOME", root.path().join("cache"))
            .env("LOAD_PASSWORD", "synthetic")
            .env("FAKE_ROOT", root.path())
            .env("TERMINAL", terminal)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for key in ["LLVM_PROFILE_FILE", "LD_LIBRARY_PATH"] {
            if let Some(value) = std::env::var_os(key) {
                command.env(key, value);
            }
        }
        let mut child = command.spawn().unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        let timeout = child.try_wait().unwrap().is_none();
        if timeout {
            child.kill().unwrap();
        }
        let output = child.wait_with_output().unwrap();
        assert!(!timeout, "coordinator did not honor wait_timeout");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(run.join("inputs/collected.txt").exists(), "{stderr}");
        let calls: Vec<Vec<String>> = fs::read_to_string(root.path().join("calls.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(calls.iter().any(|args| args[0] == "get"), "{calls:?}");
        assert_eq!(&calls.last().unwrap()[..2], ["delete", "pod"]);
        let job: Value =
            serde_json::from_slice(&fs::read(root.path().join("job.json")).unwrap()).unwrap();
        assert_eq!(job["spec"]["backoffLimitPerIndex"], 0);
        assert_eq!(job["spec"]["maxFailedIndexes"], 2);
        assert!(job["spec"].get("backoffLimit").is_none());
        match terminal {
            "Failed" => {
                assert!(!output.status.success());
                assert!(stderr.contains("Kubernetes worker job failed"), "{stderr}");
            }
            "pending" => {
                assert!(!output.status.success());
                assert!(
                    stderr.contains("Timed out waiting for Kubernetes worker job"),
                    "{stderr}"
                );
            }
            _ => assert!(output.status.success(), "{stderr}"),
        }
    }
}

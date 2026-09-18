// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Behavioral regressions for native preparation, ownership and reporting boundaries.
use super::*;
use serde_json::{json, Value};
use std::{collections::BTreeMap, io::Write};

fn input() -> input::Input {
    let mut settings = config::Settings::default();
    settings.target.tenant_id = "00000000-0000-0000-0000-000000000001".into();
    settings.workload.count = 3;
    settings.workload.shard_size = 2;
    settings.workload.start = 100;
    input::protocol(
        &settings,
        input::Event {
            election_event_id: "00000000-0000-0000-0000-000000000002".into(),
            election_id: "00000000-0000-0000-0000-000000000003".into(),
            realm: "test-realm".into(),
            area_name: "District, North".into(),
            login_url: "http://localhost:3000/login".into(),
        },
    )
}
fn samples(directory: &Path, input: &input::Input, rows: &[Value]) {
    input.save(&directory.join("config.json")).unwrap();
    for shard in 0..input.shards() {
        let out = directory.join("results").join(format!("{shard:06}"));
        files::directory(&out).unwrap();
        files::save(&out.join("exit.json"), &json!({"code":0})).unwrap();
        let (start, count) = input.bounds(shard).unwrap();
        let mut file = files::create(&out.join("samples.jsonl")).unwrap();
        for row in rows.iter().filter(|row| {
            row["index"]
                .as_u64()
                .is_some_and(|index| index >= start && index < start + count as u64)
        }) {
            writeln!(file, "{row}").unwrap();
        }
    }
}
fn successful_rows() -> Vec<Value> {
    (100..103).map(|index|json!({"index":index,"passed":true,"start":1000,"end":2000,"status_ms":index-99,"cast_ms":10,"receipt":format!("receipt-{index}")})).collect()
}

#[test]
fn million_voters_have_disjoint_bounded_shards() {
    let mut input = input();
    input.settings.workload.count = 1_000_001;
    input.settings.workload.shard_size = 10_000;
    assert_eq!(input.shards(), 101);
    assert_eq!(input.bounds(100).unwrap(), (1_000_100, 1));
    assert!(input.bounds(101).is_err());
    let mut end = input.settings.workload.start;
    for shard in 0..input.shards() {
        let (first, count) = input.bounds(shard).unwrap();
        assert_eq!(first, end);
        end += count as u64;
    }
    assert_eq!(end, 1_000_101);
}
#[test]
fn census_uses_one_valid_hash_and_quoted_csv_with_unique_names() {
    use base64::Engine as _;
    let directory = tempfile::tempdir().unwrap();
    let output = directory.path().join("census");
    let mut input = input();
    let key = format!("LOAD_TEST_{}", uuid::Uuid::new_v4().simple());
    input.settings.workload.password_env = key.clone();
    std::env::set_var(&key, "Synthetic test password");
    census::generate(&input, &output).unwrap();
    use std::os::unix::fs::PermissionsExt;
    assert_eq!(
        std::fs::metadata(&output).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert!(census::generate(&input, &output).is_err());
    std::env::remove_var(&key);
    let mut rows = Vec::new();
    let mut hash = None;
    for shard in 0..input.shards() {
        let mut reader = csv::Reader::from_path(output.join(format!("{shard:06}.csv"))).unwrap();
        assert!(!reader
            .headers()
            .unwrap()
            .iter()
            .any(|field| field == "password"));
        for record in reader.records() {
            let record = record.unwrap();
            assert_eq!(&record[1], "District, North");
            let credential = (&record[5], &record[6], &record[7]);
            if let Some(previous) = &hash {
                assert_eq!(
                    previous,
                    &(
                        credential.0.to_owned(),
                        credential.1.to_owned(),
                        credential.2.to_owned()
                    )
                );
            }
            let digest = base64::engine::general_purpose::STANDARD
                .decode(credential.0)
                .unwrap();
            let salt = base64::engine::general_purpose::STANDARD
                .decode(credential.1)
                .unwrap();
            ring::pbkdf2::verify(
                ring::pbkdf2::PBKDF2_HMAC_SHA256,
                std::num::NonZeroU32::new(credential.2.parse().unwrap()).unwrap(),
                &salt,
                b"Synthetic test password",
                &digest,
            )
            .unwrap();
            hash = Some((
                credential.0.to_owned(),
                credential.1.to_owned(),
                credential.2.to_owned(),
            ));
            rows.push(record[0].to_owned());
        }
    }
    assert_eq!(rows, vec!["load-100", "load-101", "load-102"]);
    let metadata: Value = files::read(&output.join("census.json")).unwrap();
    assert_eq!(metadata["password_hash_computations"], 1);
}
#[test]
fn protocol_uses_portal_queries_without_a_browser_capture() {
    let input = input();
    let wire = input.wire().unwrap();
    assert_eq!(wire["count"], 3);
    assert_eq!(wire["vus"], 2);
    assert_eq!(
        wire["profile"]["steps"][3]["payload"]["operationName"],
        "GetVoterStatus"
    );
    assert_eq!(
        wire["profile"]["steps"].as_array().unwrap().last().unwrap()["kind"],
        "cast"
    );
    assert!(wire["allowed_origins"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "http://localhost:3000"));
    assert!(!serde_json::to_string(&wire).unwrap().contains("auth_token"));
}
#[test]
fn status_protocol_has_no_publication_or_cast_steps() {
    let mut settings = input().settings;
    settings.workload.mode = "status".into();
    let input = input::protocol(&settings, input().event);
    assert_eq!(input.profile["steps"].as_array().unwrap().len(), 4);
}
#[test]
fn exact_global_quantiles_and_throughput_are_reported() {
    let directory = tempfile::tempdir().unwrap();
    let input = input();
    samples(directory.path(), &input, &successful_rows());
    report::generate(directory.path(), None).unwrap();
    let summary: Value = files::read(&directory.path().join("results.json")).unwrap();
    assert_eq!(summary["casts_per_second"], 3.0);
    assert_eq!(summary["latency"]["status_ms"]["p50"], 2.0);
    assert_eq!(summary["latency"]["status_ms"]["p99"], 2.98);
    let html = fs::read_to_string(directory.path().join("report.html")).unwrap();
    assert!(!html.contains("receipt-100"));
    assert!(!html.contains("GraphQL operations"));
    assert!(html.contains("data:font/woff2;base64,"));
    assert!(!html.contains("$chart"));
}
#[test]
fn duplicate_receipts_fail_but_keep_a_readable_report() {
    let directory = tempfile::tempdir().unwrap();
    let input = input();
    let mut rows = successful_rows();
    rows[1]["receipt"] = rows[0]["receipt"].clone();
    samples(directory.path(), &input, &rows);
    assert!(report::generate(directory.path(), None).is_err());
    let summary: Value = files::read(&directory.path().join("results.json")).unwrap();
    assert_eq!(summary["accepted_casts"], 2);
    assert!(fs::read_to_string(directory.path().join("report.html"))
        .unwrap()
        .contains("Duplicate voter or API receipt"));
}
#[test]
fn interrupted_runs_and_failed_goals_preserve_results() {
    let directory = tempfile::tempdir().unwrap();
    let mut input = input();
    input.settings.goals =
        BTreeMap::from([("status_ms".into(), BTreeMap::from([("p99".into(), 1.0)]))]);
    samples(directory.path(), &input, &successful_rows());
    fs::remove_file(directory.path().join("results/000001/exit.json")).unwrap();
    assert!(report::generate(directory.path(), None).is_err());
    let result: Value = files::read(&directory.path().join("results.json")).unwrap();
    assert_eq!(result["errors"].as_array().unwrap().len(), 2);
    assert!(directory.path().join("performance.svg").exists());
}
#[test]
fn tampered_configuration_fails_before_a_worker_claim() {
    let directory = tempfile::tempdir().unwrap();
    let input = input();
    input.save(&directory.path().join("config.json")).unwrap();
    files::save(
        &directory.path().join("ready.json"),
        &json!({"config_sha256":"wrong"}),
    )
    .unwrap();
    assert!(worker::shard(directory.path(), 0, directory.path()).is_err());
    assert!(!directory.path().join("results/000000/attempted").exists());
}
#[test]
fn existing_claim_is_never_reused() {
    let directory = tempfile::tempdir().unwrap();
    let mut input = input();
    input.settings.workload.mode = "status".into();
    input.save(&directory.path().join("config.json")).unwrap();
    files::save(
        &directory.path().join("ready.json"),
        &json!({"config_sha256":files::digest(&directory.path().join("config.json")).unwrap()}),
    )
    .unwrap();
    files::directory(&directory.path().join("results/000000")).unwrap();
    files::create(&directory.path().join("results/000000/attempted")).unwrap();
    let error = worker::shard(directory.path(), 0, directory.path()).unwrap_err();
    assert!(error.to_string().contains("Cannot claim"));
}
#[test]
fn docker_mapping_uses_the_most_specific_enclosing_mount() {
    let mounts = vec![
        json!({"Source":"/host","Destination":"/workspaces"}),
        json!({"Source":"/cache","Destination":"/workspaces/step/runs"}),
    ];
    assert_eq!(
        executor::mapped_mount(Path::new("/workspaces/step/runs/smoke"), &mounts).unwrap(),
        Path::new("/cache/smoke")
    );
    assert!(executor::mapped_mount(Path::new("/unshared"), &mounts).is_err());
}
#[test]
fn indexed_jobs_have_no_retries_or_embedded_passwords() {
    let input = input();
    let job = executor::job(&input.settings, "run-test", 40, 1005, 1005);
    assert_eq!(job["spec"]["parallelism"], 40);
    assert_eq!(job["spec"]["backoffLimit"], 0);
    assert_eq!(
        job["spec"]["template"]["spec"]["containers"][0]["command"][0],
        "/usr/local/bin/step-load-worker"
    );
    assert!(job.to_string().contains("secretKeyRef"));
    assert!(!job.to_string().contains("python"));
}
#[test]
fn fixture_configures_exact_login_and_retains_one_eligible_contest() {
    let input = input();
    let fixture: Value =
        serde_json::from_str(include_str!("../../../voting-load/fixtures/election.json")).unwrap();
    let fixture = provision::fixture(fixture, &input.settings).unwrap();
    assert_eq!(fixture["elections"].as_array().unwrap().len(), 1);
    assert_eq!(fixture["contests"][0]["min_votes"], 1);
    for config in fixture["keycloak_event_realm"]["authenticatorConfig"]
        .as_array()
        .unwrap()
    {
        if let Some(value) = config["config"].get("matchAttributes") {
            assert_eq!(value, "username");
        }
    }
}
#[test]
fn default_choices_reject_impossible_ballots() {
    let invalid = json!({"contests":[{"id":"contest","min_votes":2,"max_votes":1,"candidates":[{"id":"candidate"}]}]});
    assert!(coordinator::default_choices(&invalid).is_err());
    assert_eq!(
        presentation::escape("<script>&\""),
        "&lt;script&gt;&amp;&quot;"
    );
}
#[test]
fn duration_parser_preserves_fractional_and_compound_timeouts() {
    assert_eq!(coordinator::duration("1m500ms").unwrap().as_millis(), 60500);
    for bad in ["0s", "-1s", "1day", "NaNs", ""] {
        assert!(coordinator::duration(bad).is_err());
    }
}

#[test]
fn every_worker_is_joined_even_when_one_fails() {
    let completed = std::sync::atomic::AtomicUsize::new(0);
    assert!(coordinator::parallel(4, |index| {
        completed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        anyhow::ensure!(index != 0, "simulated worker failure");
        Ok(())
    })
    .is_err());
    assert_eq!(completed.load(std::sync::atomic::Ordering::SeqCst), 4);
}

#[test]
fn engine_failure_still_publishes_the_run_report() {
    let directory = tempfile::tempdir().unwrap();
    let inputs = directory.path().join("inputs");
    files::directory(&inputs).unwrap();
    let mut input = input();
    input.settings.workload.mode = "status".into();
    input.settings.runtime.k6 = "/bin/false".into();
    let key = format!("LOAD_TEST_{}", uuid::Uuid::new_v4().simple());
    input.settings.workload.password_env = key.clone();
    std::env::set_var(&key, "synthetic");
    input.save(&inputs.join("config.json")).unwrap();
    files::save(
        &inputs.join("ready.json"),
        &json!({"config_sha256":files::digest(&inputs.join("config.json")).unwrap()}),
    )
    .unwrap();
    assert!(executor::run(
        directory.path(),
        &input.settings,
        2,
        "local",
        directory.path()
    )
    .is_err());
    std::env::remove_var(&key);
    assert!(directory.path().join("report.html").exists());
    for shard in 0..input.shards() {
        assert!(inputs
            .join(format!("results/{shard:06}/exit.json"))
            .exists());
    }
    assert!(executor::run(
        directory.path(),
        &input.settings,
        2,
        "local",
        directory.path()
    )
    .is_err());
}

#[test]
fn quantiles_handle_empty_singleton_and_repeated_positions() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE samples(status_ms REAL)")
        .unwrap();
    assert_eq!(
        report::percentile(&db, report::Stage::Status, 0.5).unwrap(),
        None
    );
    db.execute("INSERT INTO samples VALUES (42)", []).unwrap();
    assert_eq!(
        report::quantiles(&db, report::Stage::Status, &[0.0, 0.5, 0.5, 1.0]).unwrap(),
        vec![Some(42.0); 4]
    );
    assert!(report::quantiles(&db, report::Stage::Status, &[1.0, 0.0]).is_err());
}

#[test]
#[ignore = "writes one million synthetic observations to exercise disk-backed aggregation"]
fn million_sample_report() {
    let directory = tempfile::tempdir().unwrap();
    let mut input = input();
    input.settings.workload.count = 1_000_000;
    input.settings.workload.shard_size = 50_000;
    input.settings.workload.start = 0;
    input.save(&directory.path().join("config.json")).unwrap();
    for shard in 0..input.shards() {
        let out = directory.path().join(format!("results/{shard:06}"));
        files::directory(&out).unwrap();
        files::save(&out.join("exit.json"), &json!({"code":0})).unwrap();
        let mut file = std::io::BufWriter::new(files::create(&out.join("samples.jsonl")).unwrap());
        let (first, count) = input.bounds(shard).unwrap();
        for index in first..first + count as u64 {
            writeln!(file, "{}", json!({"index":index,"passed":true,"start":index,"end":index+100,"status_ms":index%100,"cast_ms":7,"receipt":format!("synthetic-{index}")})).unwrap();
        }
        file.flush().unwrap();
    }
    let start = std::time::Instant::now();
    report::generate(directory.path(), None).unwrap();
    let result: Value = files::read(&directory.path().join("results.json")).unwrap();
    assert_eq!(result["completed"], 1_000_000);
    assert_eq!(result["accepted_casts"], 1_000_000);
    assert_eq!(result["latency"]["status_ms"]["p50"], 49.5);
    println!(
        "Aggregated 1,000,000 synthetic samples in {:.2}s",
        start.elapsed().as_secs_f64()
    );
}

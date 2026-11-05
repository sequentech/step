// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A stage from its definition to its verdict, over an in-memory election event.
use super::{
    ledger::tests::now,
    verdict::assess,
    voting::tests::{since, Event},
    *,
};
use chrono::Duration;

struct Stage {
    _directory: tempfile::TempDir,
    ledger: PathBuf,
    evidence: PathBuf,
}

fn opened(template: Template) -> Stage {
    let directory = tempfile::tempdir().unwrap();
    let definition = directory.path().join("stage.yaml");
    template.create(&definition).unwrap();
    let ledger = directory.path().join("ledger.jsonl");
    open(
        &definition,
        &ledger,
        Target {
            tenant_id: "tenant".into(),
            election_event_id: "event".into(),
            started_at: since(),
            release: Some("10.0.0".into()),
        },
        "operator",
        now(),
    )
    .unwrap();
    let evidence = directory.path().join("screen.png");
    fs::write(&evidence, b"picture").unwrap();
    Stage {
        _directory: directory,
        ledger,
        evidence,
    }
}

fn voters() -> Vec<String> {
    vec!["ana".into(), "ben".into()]
}

fn run_all(stage: &Stage, event: &Event, ballot_ids: Vec<String>) -> Vec<(String, Ran)> {
    run(
        &stage.ledger,
        event,
        &voters(),
        ballot_ids,
        &[],
        "operator",
        now(),
    )
    .unwrap()
}

fn witness(stage: &Stage, check: &str, outcome: Outcome) {
    record(
        &stage.ledger,
        Observation {
            check: check,
            outcome: outcome,
            witness: "Witness",
            note: &None,
            evidence: &[],
        },
        "operator",
        now(),
    )
    .unwrap();
}

fn stage_verdict(stage: &Stage) -> Verdict {
    assess(&Ledger::read(&stage.ledger).unwrap()).verdict
}

#[test]
fn the_voting_stage_passes_when_every_condition_holds() {
    let stage = opened(Template::Voting);
    let event = Event::voted(&["ana", "ben"]);
    let ran = run_all(&stage, &event, event.ballot_ids());
    assert_eq!(ran.len(), 5);
    assert!(ran
        .iter()
        .all(|(_, outcome)| *outcome == Ran::Recorded(Outcome::Pass)));
    assert_eq!(stage_verdict(&stage), Verdict::Incomplete);

    witness(&stage, "voting.ballot-displayed", Outcome::Pass);
    assert_eq!(stage_verdict(&stage), Verdict::Passed);
    assert!(verdict(&stage.ledger, &None).is_ok());

    let opening = Ledger::read(&stage.ledger).unwrap().opening;
    assert_eq!(opening.release.as_deref(), Some("10.0.0"));
    assert_eq!(opening.definition_sha256.len(), 64);
}

#[test]
fn each_voting_failure_condition_fails_the_stage() {
    type Break = fn(&mut Event);
    let cases: [(&str, Break); 5] = [
        ("voting.authentication", |event| {
            event.events.remove("id-ben");
        }),
        ("voting.ballot-published", |event| {
            event.published.clear();
        }),
        ("voting.ballot-cast", |event| {
            event.ballots.pop();
        }),
        ("voting.receipt", |event| {
            event.ballots[1].ballot_id = "another".into();
            event.logged.get_mut("id-ben").unwrap()[0].ballot_id = "another".into();
        }),
        ("voting.ballot-stored", |event| {
            event.logged.get_mut("id-ana").unwrap()[0].vote_hash = "00".repeat(64);
        }),
    ];
    for (check, breakage) in cases {
        let stage = opened(Template::Voting);
        let mut event = Event::voted(&["ana", "ben"]);
        let presented = event.ballot_ids();
        breakage(&mut event);
        let ran = run_all(&stage, &event, presented);
        witness(&stage, "voting.ballot-displayed", Outcome::Pass);
        for (id, outcome) in &ran {
            let failed = *outcome == Ran::Recorded(Outcome::Fail);
            // Without ben's ballot nothing was stored for him either way.
            assert_eq!(
                failed,
                id == check || (check == "voting.ballot-cast" && id == "voting.receipt"),
                "{check}: {id}"
            );
        }
        assert_eq!(stage_verdict(&stage), Verdict::Failed, "{check}");
        let error = verdict(&stage.ledger, &None).unwrap_err();
        assert!(error.to_string().contains("is failed"));
    }
}

#[test]
fn a_ballot_that_is_not_displayed_fails_the_stage() {
    let stage = opened(Template::Voting);
    let event = Event::voted(&["ana", "ben"]);
    run_all(&stage, &event, event.ballot_ids());
    witness(&stage, "voting.ballot-displayed", Outcome::Fail);
    assert_eq!(stage_verdict(&stage), Verdict::Failed);
}

#[test]
fn a_failed_annex_condition_fails_the_pqri_voting_stage() {
    let stage = opened(Template::PqriVoting);
    let event = Event::voted(&["ana", "ben"]);
    run_all(&stage, &event, event.ballot_ids());
    let witnessed: Vec<String> = Ledger::read(&stage.ledger)
        .unwrap()
        .opening
        .stage
        .checks
        .iter()
        .filter(|check| check.method == Method::Witnessed)
        .map(|check| check.id.clone())
        .collect();
    assert_eq!(witnessed.len(), 21);
    for check in &witnessed[1..] {
        witness(&stage, check, Outcome::Pass);
    }
    assert_eq!(stage_verdict(&stage), Verdict::Incomplete);
    assert!(verdict(&stage.ledger, &None)
        .unwrap_err()
        .to_string()
        .contains("is incomplete"));

    witness(&stage, &witnessed[0], Outcome::Pass);
    assert_eq!(stage_verdict(&stage), Verdict::Passed);

    witness(&stage, "annex-a.d.4.8.1", Outcome::Fail);
    assert_eq!(stage_verdict(&stage), Verdict::Failed);

    witness(&stage, "annex-a.d.4.8.1", Outcome::Pass);
    assert_eq!(stage_verdict(&stage), Verdict::Passed);
}

#[test]
fn receipts_wait_for_the_presented_ballot_ids() {
    let stage = opened(Template::Voting);
    let event = Event::voted(&["ana", "ben"]);
    let ran = run_all(&stage, &event, vec![]);
    let receipt = ran.iter().find(|(id, _)| id == "voting.receipt").unwrap();
    assert!(matches!(receipt.1, Ran::NotRun(_)));
    witness(&stage, "voting.ballot-displayed", Outcome::Pass);
    assert_eq!(stage_verdict(&stage), Verdict::Incomplete);

    let only = vec!["voting.receipt".to_string()];
    let ran = run(
        &stage.ledger,
        &event,
        &voters(),
        event.ballot_ids(),
        &only,
        "operator",
        now(),
    )
    .unwrap();
    assert_eq!(
        ran,
        [("voting.receipt".to_string(), Ran::Recorded(Outcome::Pass))]
    );
    assert_eq!(stage_verdict(&stage), Verdict::Passed);
}

#[test]
fn records_before_the_run_started_are_not_evidence() {
    let stage = opened(Template::Voting);
    let mut event = Event::voted(&["ana", "ben"]);
    for ballot in &mut event.ballots {
        ballot.cast_at = since() - Duration::hours(1);
    }
    let ran = run_all(&stage, &event, event.ballot_ids());
    let failed: Vec<&str> = ran
        .iter()
        .filter(|(_, outcome)| *outcome == Ran::Recorded(Outcome::Fail))
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        failed,
        [
            "voting.ballot-cast",
            "voting.receipt",
            "voting.ballot-stored"
        ]
    );
}

#[test]
fn run_refuses_witnessed_or_unknown_checks_and_missing_voters() {
    let stage = opened(Template::Voting);
    let event = Event::voted(&["ana"]);
    let before = fs::read_to_string(&stage.ledger).unwrap();
    for (voters, only) in [
        (voters(), vec!["voting.ballot-displayed".to_string()]),
        (voters(), vec!["unknown".to_string()]),
        (vec![], vec![]),
    ] {
        assert!(run(
            &stage.ledger,
            &event,
            &voters,
            vec![],
            &only,
            "operator",
            now()
        )
        .is_err());
    }
    assert_eq!(fs::read_to_string(&stage.ledger).unwrap(), before);
}

#[test]
fn record_refuses_automatic_checks_and_keeps_evidence_hashes() {
    let stage = opened(Template::Voting);
    let automatic = record(
        &stage.ledger,
        Observation {
            check: "voting.ballot-cast",
            outcome: Outcome::Pass,
            witness: "Witness",
            note: &None,
            evidence: &[],
        },
        "operator",
        now(),
    );
    assert!(automatic.is_err());
    let unnamed = record(
        &stage.ledger,
        Observation {
            check: "voting.ballot-displayed",
            outcome: Outcome::Pass,
            witness: " ",
            note: &None,
            evidence: &[],
        },
        "operator",
        now(),
    );
    assert!(unnamed.is_err());

    let ledger = record(
        &stage.ledger,
        Observation {
            check: "voting.ballot-displayed",
            outcome: Outcome::Pass,
            witness: "Witness",
            note: &Some("Seen on a phone".into()),
            evidence: std::slice::from_ref(&stage.evidence),
        },
        "operator",
        now(),
    )
    .unwrap();
    let (_, result) = ledger.results().last().unwrap();
    assert_eq!(result.note.as_deref(), Some("Seen on a phone"));
    assert_eq!(result.findings[0].name, "screen.png");
    assert_eq!(
        result.findings[0].detail,
        format!("sha256 {}", ledger::file_sha256(&stage.evidence).unwrap())
    );
}

#[test]
fn an_altered_ledger_has_no_verdict() {
    let stage = opened(Template::Voting);
    let event = Event::voted(&["ana", "ben"]);
    run_all(&stage, &event, event.ballot_ids());
    witness(&stage, "voting.ballot-displayed", Outcome::Fail);
    let text = fs::read_to_string(&stage.ledger).unwrap();
    let forged = text.replace(
        "\"method\":\"witnessed\",\"outcome\":\"fail\"",
        "\"method\":\"witnessed\",\"outcome\":\"pass\"",
    );
    assert_ne!(forged, text);
    fs::write(&stage.ledger, forged).unwrap();
    let error = format!("{:#}", verdict(&stage.ledger, &None).unwrap_err());
    assert!(error.contains("entry 6 was altered"), "{error}");
}

#[test]
fn the_report_is_written_whatever_the_verdict() {
    let stage = opened(Template::Voting);
    let report = stage.ledger.with_extension("md");
    assert!(verdict(&stage.ledger, &Some(report.clone())).is_err());
    let text = fs::read_to_string(report).unwrap();
    assert!(text.starts_with("# Voting: incomplete"));
    assert!(text.contains("- Release: 10.0.0"));
}

#[test]
fn names_are_trimmed_and_not_repeated() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("voters.txt");
    fs::write(&file, "ben\n\n  carl \nana\n").unwrap();
    assert_eq!(
        names(&["ana".into(), " ben".into()], &Some(file)).unwrap(),
        ["ana", "ben", "carl"]
    );
    assert!(names(&[], &Some(directory.path().join("missing"))).is_err());
}

#[test]
fn a_ledger_cannot_start_in_the_future_or_from_a_bad_definition() {
    let directory = tempfile::tempdir().unwrap();
    let definition = directory.path().join("stage.yaml");
    Template::Voting.create(&definition).unwrap();
    let ledger = directory.path().join("ledger.jsonl");
    let target = |started_at| Target {
        tenant_id: "t".into(),
        election_event_id: "e".into(),
        started_at,
        release: None,
    };
    let future = now() + Duration::seconds(1);
    assert!(open(&definition, &ledger, target(future), "operator", now()).is_err());
    fs::write(&definition, "id: voting\ntitle: V\nchecks: []\n").unwrap();
    assert!(open(&definition, &ledger, target(now()), "operator", now()).is_err());
    assert!(!ledger.exists());
}

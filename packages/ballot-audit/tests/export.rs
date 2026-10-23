// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use b3::messages::{
    artifact::{Ballots, Configuration},
    message::Message,
    newtypes::{PublicKeyHash, MAX_TRUSTEES, NULL_TRUSTEE},
    protocol_manager::ProtocolManager,
};
use ballot_audit::{digest, ExitStatus};
use base64::{engine::general_purpose::STANDARD_NO_PAD, Engine};
use serde_json::{json, Value};
use std::{
    fs,
    marker::PhantomData,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};
use strand::{
    backend::ristretto::RistrettoCtx,
    serialization::{StrandDeserialize, StrandSerialize},
    signature::{StrandSignaturePk, StrandSignatureSk},
};
use tokio_postgres::{config::Host, Client, Config, NoTls};

const TENANT: &str = "00000000-0000-4000-8000-000000000010";
const EVENT: &str = "00000000-0000-4000-8000-000000000020";
const ELECTION: &str = "00000000-0000-4000-8000-000000000030";
const AREA: &str = "00000000-0000-4000-8000-000000000040";
const SESSION: &str = "00000000-0000-4000-8000-000000000050";
const MAPPING: &str = "00000000-0000-4000-8000-000000000060";
const CEREMONY: &str = "00000000-0000-4000-8000-000000000070";
const BOARD: &str = "audit_board";
const ALIAS: &str = "alias' OR '1'='1";
const READER_PASSWORD: &str = "synthetic-reader";
const CONTENT: &str = " \t{\"ciphertexts\" : [\"α\", \"\\u0061\"]}\r\n";

struct TempDir(PathBuf);
impl TempDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ballot-export-integration-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

async fn fixture_client(name: &str, expected_database: &str) -> (Client, String) {
    let dsn = std::env::var(name).expect("use a disposable synthetic PostgreSQL fixture");
    let config: Config = dsn.parse().unwrap();
    assert_eq!(config.get_dbname(), Some(expected_database));
    assert_eq!(config.get_user(), Some("audit_fixture"));
    assert!(config.get_hostaddrs().is_empty());
    assert!(matches!(config.get_hosts(), [Host::Tcp(host)] if host == "127.0.0.1"));
    let (client, connection) = config.connect(NoTls).await.unwrap();
    tokio::spawn(async move {
        connection.await.unwrap();
    });
    // The exporter must connect as the unprivileged reader: the fixture role
    // owns the tables and would bypass row-level security.
    let port = config.get_ports().first().copied().unwrap_or(5432);
    let reader = format!(
        "host=127.0.0.1 port={port} user=audit_reader password={READER_PASSWORD} dbname={expected_database}"
    );
    (client, reader)
}

fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn invoke(root: &Path, name: &str, scope: &Value, with_blobs: bool) -> Output {
    let scope_path = root.join("scope-input.json");
    write_json(&scope_path, scope);
    let pin = digest(&fs::read(&scope_path).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_export-ballot-audit"));
    command.args([
        "--scope",
        scope_path.to_str().unwrap(),
        "--scope-sha512",
        &pin,
        "--connections",
        root.join("connections.json").to_str().unwrap(),
        "--output",
        root.join(name).to_str().unwrap(),
    ]);
    if with_blobs {
        command.arg("--b3-blob-root").arg(root.join("blobs"));
    }
    command.output().unwrap()
}

fn assert_exit(output: Output, status: ExitStatus) {
    assert_eq!(
        output.status.code(),
        Some(status.code()),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn load(root: &Path, directory: &str, file: &str) -> Value {
    serde_json::from_slice(&fs::read(root.join(directory).join(file)).unwrap()).unwrap()
}

fn assert_success(output: Output) {
    assert!(
        output.status.success(),
        "export failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("postgresql://"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("postgresql://"));
}

/// Requires three fresh disposable databases and an explicit test-only opt-in.
/// Never run against a restored election database or existing application schema.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires freshly provisioned synthetic local PostgreSQL databases"]
async fn exports_native_postgres_snapshots_without_application_servers() {
    assert_eq!(
        std::env::var("BALLOT_AUDIT_RUN_SYNTHETIC_DB_TESTS").as_deref(),
        Ok("1")
    );
    let (hasura, hasura_url) = fixture_client("BALLOT_AUDIT_TEST_HASURA_URL", "audit_hasura").await;
    let (keycloak, keycloak_url) =
        fixture_client("BALLOT_AUDIT_TEST_KEYCLOAK_URL", "audit_keycloak").await;
    let (b3, b3_url) = fixture_client("BALLOT_AUDIT_TEST_B3_URL", "audit_b3").await;
    let root = TempDir::new();
    hasura.batch_execute(&format!("CREATE ROLE audit_reader LOGIN PASSWORD '{READER_PASSWORD}'
        NOSUPERUSER NOCREATEDB NOCREATEROLE;
        ALTER ROLE audit_reader SET default_transaction_read_only = on;
        CREATE SCHEMA sequent_backend;
        CREATE TABLE sequent_backend.election_event (id uuid PRIMARY KEY, tenant_id uuid, bulletin_board_reference jsonb);
        CREATE TABLE sequent_backend.election (id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, external_id text);
        CREATE TABLE sequent_backend.keys_ceremony (id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, is_default boolean);
        CREATE TABLE sequent_backend.tally_session (id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid, keys_ceremony_id uuid, configuration jsonb);
        CREATE TABLE sequent_backend.tally_session_contest (id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid,
            election_id uuid, area_id uuid, tally_session_id uuid, contest_id uuid, session_id integer, annotations jsonb);
        CREATE TABLE sequent_backend.tally_session_execution (id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid,
            tally_session_id uuid, created_at timestamptz);
        CREATE TABLE sequent_backend.cast_vote (id uuid PRIMARY KEY, tenant_id uuid, election_event_id uuid,
            election_id uuid, area_id uuid, created_at timestamptz, voter_id_string text, content text,
            ballot_id text, status text, annotations jsonb);
        GRANT USAGE ON SCHEMA sequent_backend TO audit_reader;
        GRANT SELECT ON ALL TABLES IN SCHEMA sequent_backend TO audit_reader;")).await.unwrap();
    hasura
        .execute(
            "INSERT INTO sequent_backend.election_event VALUES ($1::text::uuid,$2::text::uuid,$3)",
            &[&EVENT, &TENANT, &json!({"database_name":BOARD})],
        )
        .await
        .unwrap();
    hasura.execute("INSERT INTO sequent_backend.election VALUES ($1::text::uuid,$2::text::uuid,$3::text::uuid,$4)",
        &[&ELECTION, &TENANT, &EVENT, &ALIAS]).await.unwrap();
    hasura.execute("INSERT INTO sequent_backend.keys_ceremony VALUES ($1::text::uuid,$2::text::uuid,$3::text::uuid,true)",
        &[&CEREMONY, &TENANT, &EVENT]).await.unwrap();
    hasura.execute("INSERT INTO sequent_backend.tally_session VALUES ($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,$5)",
        &[&SESSION,&TENANT,&EVENT,&CEREMONY,&json!({"contest_encryption_policy":"multiple-contests"})]).await.unwrap();
    hasura.execute("INSERT INTO sequent_backend.tally_session_contest VALUES ($1::text::uuid,$2::text::uuid,$3::text::uuid,
        $4::text::uuid,$5::text::uuid,$6::text::uuid,NULL,10,'{}')",
        &[&MAPPING,&TENANT,&EVENT,&ELECTION,&AREA,&SESSION]).await.unwrap();
    for (index, status, voter, content) in [
        (1, "valid", "alice", "older"),
        (2, "valid", "alice", CONTENT),
        (3, "discarded", "alice", "discarded"),
        (4, "in-progress", "unknown", "pending"),
        (5, "valid", "disabled", "excluded"),
    ] {
        let id = format!("00000000-0000-4000-8000-{index:012}");
        hasura.execute("INSERT INTO sequent_backend.cast_vote VALUES ($1::text::uuid,$2::text::uuid,$3::text::uuid,
            $4::text::uuid,$5::text::uuid,'2026-10-01T00:00:00Z',$6,$7,'receipt',$8,'{}')",
            &[&id,&TENANT,&EVENT,&ELECTION,&AREA,&voter,&content,&status]).await.unwrap();
    }
    hasura.execute("INSERT INTO sequent_backend.cast_vote SELECT '00000000-0000-4000-8000-000000000099',
        tenant_id,election_event_id,'00000000-0000-4000-8000-000000000999',area_id,created_at,
        voter_id_string,content,ballot_id,status,annotations FROM sequent_backend.cast_vote LIMIT 1", &[]).await.unwrap();
    hasura
        .execute(
            "INSERT INTO sequent_backend.cast_vote SELECT '00000000-0000-4000-8000-000000000098',
        tenant_id,election_event_id,election_id,'00000000-0000-4000-8000-000000000998',created_at,
        voter_id_string,content,ballot_id,status,annotations FROM sequent_backend.cast_vote
        WHERE id='00000000-0000-4000-8000-000000000004'",
            &[],
        )
        .await
        .unwrap();

    keycloak.batch_execute("CREATE TABLE public.realm (id text PRIMARY KEY, name text UNIQUE);
        CREATE TABLE public.user_entity (id text PRIMARY KEY, realm_id text, username text,
            enabled boolean, service_account_client_link text);
        CREATE TABLE public.user_attribute (id text PRIMARY KEY, user_id text, name text, value text);
        GRANT SELECT ON ALL TABLES IN SCHEMA public TO audit_reader;").await.unwrap();
    let realm = format!("tenant-{TENANT}-event-{EVENT}");
    keycloak
        .execute(
            "INSERT INTO public.realm VALUES ('event-realm',$1),('other-realm','other')",
            &[&realm],
        )
        .await
        .unwrap();
    keycloak.batch_execute("INSERT INTO public.user_entity VALUES
        ('alice','event-realm','alice',true,NULL), ('bob','event-realm','bob',true,NULL),
        ('null-auth','event-realm','null-auth',true,NULL), ('disabled','event-realm','disabled',false,NULL),
        ('service','event-realm','service',true,'client'), ('no-area','event-realm','no-area',true,NULL),
        ('wrong-auth','event-realm','wrong-auth',true,NULL), ('empty-auth','event-realm','empty-auth',true,NULL),
        ('null-enabled','event-realm','null-enabled',NULL,NULL),
        ('outside','other-realm','outside',false,NULL), ('outside-service','other-realm','outside-service',true,'client');").await.unwrap();
    for user in [
        "alice",
        "bob",
        "null-auth",
        "disabled",
        "service",
        "wrong-auth",
        "empty-auth",
        "null-enabled",
    ] {
        keycloak
            .execute(
                "INSERT INTO public.user_attribute VALUES ($1,$2,'area-id',$3)",
                &[&format!("area-{user}"), &user, &AREA],
            )
            .await
            .unwrap();
    }
    keycloak.execute("INSERT INTO public.user_attribute VALUES ('auth-alice','alice','authorized-election-ids',$1),
        ('auth-bob','bob','authorized-election-ids',$1)", &[&ALIAS]).await.unwrap();
    keycloak
        .batch_execute(
            "INSERT INTO public.user_attribute VALUES
        ('auth-null','null-auth','authorized-election-ids',NULL),
        ('auth-wrong','wrong-auth','authorized-election-ids','unrelated'),
        ('auth-empty','empty-auth','authorized-election-ids',''),
        ('weight-empty','alice','vote-weight',''), ('weight-seven','alice','vote-weight','7'),
        ('weight-three','alice','vote-weight','3'),
        ('delegate-self','alice','delegate-vote-to','alice'),
        ('delegate-disabled','disabled','delegate-vote-to','alice'),
        ('delegate-outside-1','outside','delegate-vote-to','alice'),
        ('delegate-outside-2','outside','delegate-vote-to','alice'),
        ('delegate-service','outside-service','delegate-vote-to','alice');",
        )
        .await
        .unwrap();

    b3.batch_execute("CREATE TABLE public.index (id integer PRIMARY KEY, board_name text UNIQUE, message_count integer);
        CREATE TABLE public.audit_board (id bigint PRIMARY KEY, sender_pk text, statement_kind text,
            batch integer, mix_number integer, message bytea, version text);
        INSERT INTO public.index VALUES (1,'audit_board',2);
        GRANT SELECT ON ALL TABLES IN SCHEMA public TO audit_reader;").await.unwrap();
    let pm = ProtocolManager::<RistrettoCtx>::new(StrandSignatureSk::gen().unwrap());
    let pk = StrandSignaturePk::from_sk(&pm.signing_key).unwrap();
    let configuration = Configuration::<RistrettoCtx>::new(
        1,
        pk.clone(),
        vec![
            StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap(),
            StrandSignaturePk::from_sk(&StrandSignatureSk::gen().unwrap()).unwrap(),
        ],
        2,
        PhantomData,
    );
    let mut trustees = [NULL_TRUSTEE; MAX_TRUSTEES];
    trustees[0] = 1;
    trustees[1] = 2;
    let messages = [
        Message::bootstrap_msg(&configuration, &pm).unwrap(),
        Message::ballots_msg(
            &configuration,
            10,
            &Ballots::new(vec![]),
            trustees,
            PublicKeyHash([7; 64]),
            &pm,
        )
        .unwrap(),
    ];
    fs::create_dir_all(root.0.join("blobs").join(BOARD)).unwrap();
    let mut expected_bytes = Vec::new();
    let mut blob_paths = Vec::new();
    for (i, message) in messages.iter().enumerate() {
        let bytes = message.strand_serialize().unwrap();
        let sender = message.sender.pk.to_der_b64_string().unwrap();
        let kind = message.statement.get_kind().to_string();
        let batch = message.statement.get_batch_number() as i32;
        let mix_number = message.statement.get_mix_number() as i32;
        let database_bytes = if i == 0 { bytes.clone() } else { vec![] };
        b3.execute(
            "INSERT INTO public.audit_board VALUES ($1,$2,$3,$4,$5,$6,'test')",
            &[
                &((i + 1) as i64),
                &sender,
                &kind,
                &batch,
                &mix_number,
                &database_bytes,
            ],
        )
        .await
        .unwrap();
        let path = root
            .0
            .join("blobs")
            .join(BOARD)
            .join(format!("{kind}-{sender}-{batch}-{mix_number}").replace('/', ":"));
        fs::write(&path, &bytes).unwrap();
        blob_paths.push(path);
        expected_bytes.push(STANDARD_NO_PAD.encode(bytes));
    }
    write_json(
        &root.0.join("connections.json"),
        &json!({"format_version":1,
        "hasura":{"connection_string":hasura_url,"restore_reference":"synthetic-hasura"},
        "keycloak":{"connection_string":keycloak_url,"restore_reference":"synthetic-keycloak"},
        "b3":{"connection_string":b3_url,"restore_reference":"synthetic-b3"}}),
    );
    let mut scope = json!({"format_version":1,"scope":{"tenant_id":TENANT,"election_event_id":EVENT,
        "election_id":ELECTION,"area_id":AREA,"tally_session_id":SESSION,"tally_session_contest_id":MAPPING,
        "board_name":BOARD,"public_key_sha512":hex::encode([7;64]),
        "electoral_log_system_public_key":pk.to_der_b64_string().unwrap(),
        "contest_id":null,"base_batch":10,"policy":"unweighted"},"batches":[10]});
    assert_success(invoke(&root.0, "plain", &scope, true));
    tally_run_derives_the_pinned_scope(&root.0, &scope);
    let acquisition = load(&root.0, "plain", "acquisition.json");
    for name in ["hasura", "keycloak", "b3"] {
        assert_eq!(acquisition["snapshots"][name]["read_only"], "on");
        assert_eq!(
            acquisition["snapshots"][name]["isolation"],
            "repeatable read"
        );
    }
    assert_eq!(acquisition["counts"]["cast_vote_rows_including_revotes"], 5);
    assert_eq!(acquisition["counts"]["cast_vote_history_rows_all_areas"], 6);
    assert_eq!(
        acquisition["cast_vote_history_scope"]["election_id"],
        ELECTION
    );
    assert!(acquisition["cast_vote_history_scope"]["area_filter"].is_null());
    let history = load(&root.0, "plain", "cast-vote-history.json");
    let history = history.as_array().unwrap();
    assert_eq!(history.len(), 6);
    assert!(history.iter().all(|row| row["election_id"] == ELECTION));
    assert!(history
        .iter()
        .any(|row| row["area_id"] == "00000000-0000-4000-8000-000000000998"));
    for status in ["valid", "discarded", "in-progress"] {
        assert!(history.iter().any(|row| row["status"] == status));
    }
    assert_eq!(
        acquisition["files"]["cast-vote-history.json"]["sha512"],
        digest(&fs::read(root.0.join("plain/cast-vote-history.json")).unwrap()).unwrap()
    );
    let votes = load(&root.0, "plain", "cast-votes.json");
    for vote in votes.as_array().unwrap() {
        assert!(history.contains(vote));
    }
    assert_eq!(votes[1]["content"].as_str(), Some(CONTENT));
    assert_eq!(votes[2]["status"], "discarded");
    assert_eq!(votes[3]["status"], "in-progress");
    chrono::DateTime::parse_from_rfc3339(votes[0]["created_at"].as_str().unwrap()).unwrap();
    assert_eq!(
        load(&root.0, "plain", "board-messages.json"),
        json!(expected_bytes)
    );
    let reasons = load(&root.0, "plain", "eligibility-reasons.json");
    assert_eq!(reasons["source_users"], 8);
    assert_eq!(reasons["eligible_users"], 3);
    assert_eq!(reasons["reason_counts"]["not_enabled"], 2);
    assert_eq!(reasons["reason_counts"]["service_account"], 1);
    assert_eq!(reasons["reason_counts"]["election_not_authorized"], 2);
    let keycloak_source =
        String::from_utf8(fs::read(root.0.join("plain/keycloak-source.json")).unwrap()).unwrap();
    for absent in [
        "no-area",
        "outside",
        "\"username\"",
        "vote-weight",
        "delegate-vote-to\",",
    ] {
        assert!(!keycloak_source.contains(absent), "exported {absent}");
    }
    for file in acquisition["files"]
        .as_object()
        .unwrap()
        .keys()
        .chain(std::iter::once(&"acquisition.json".to_string()))
    {
        assert_eq!(
            fs::metadata(root.0.join("plain").join(file))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert_eq!(
        fs::metadata(root.0.join("plain"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let before = fs::read(root.0.join("plain/acquisition.json")).unwrap();
    assert_exit(
        invoke(&root.0, "plain", &scope, true),
        ExitStatus::InvalidInput,
    );
    assert_eq!(
        fs::read(root.0.join("plain/acquisition.json")).unwrap(),
        before
    );
    assert_exit(
        invoke(&root.0, "no-blobs", &scope, false),
        ExitStatus::Incomplete,
    );
    assert!(!root.0.join("no-blobs/acquisition.json").exists());

    hasura
        .batch_execute("UPDATE sequent_backend.tally_session_contest SET session_id = 0;")
        .await
        .unwrap();
    scope["scope"]["base_batch"] = json!(0);
    scope["batches"] = json!([0]);
    assert_success(invoke(&root.0, "first-batch", &scope, true));
    assert_eq!(
        load(&root.0, "first-batch", "board-messages.json"),
        json!([expected_bytes[0]])
    );
    let first_batch = load(&root.0, "first-batch", "acquisition.json");
    assert_eq!(first_batch["board_index_count"]["observed"], 2);
    assert_eq!(first_batch["board_index_count"]["exported"], 1);
    hasura
        .batch_execute("UPDATE sequent_backend.tally_session_contest SET session_id = 10;")
        .await
        .unwrap();
    scope["scope"]["base_batch"] = json!(10);
    scope["batches"] = json!([10]);

    hasura.batch_execute("UPDATE sequent_backend.tally_session SET configuration =
        '{\"contest_encryption_policy\":\"multiple-contests\",\"weighted_voting_policy\":\"voters-weighted-voting\"}';
        UPDATE sequent_backend.tally_session_contest SET annotations = '{\"weight_bit_mask\":3}';").await.unwrap();
    scope["scope"]["policy"] = json!("weighted");
    scope["batches"] = json!([10, 11]);
    assert_success(invoke(&root.0, "weighted", &scope, true));
    assert_eq!(
        load(&root.0, "weighted", "eligible-voters.json")["voters"],
        json!([
        {"voter_id":"alice","multiplicity":3},{"voter_id":"bob","multiplicity":1},{"voter_id":"null-auth","multiplicity":1}])
    );
    scope["batches"] = json!([10]);
    assert_exit(
        invoke(&root.0, "wrong-mask", &scope, true),
        ExitStatus::VerificationFailed,
    );
    scope["batches"] = json!([10, 11]);
    keycloak.batch_execute("INSERT INTO public.user_attribute VALUES ('bad-weight','bob','vote-weight','not-a-number');").await.unwrap();
    assert_exit(
        invoke(&root.0, "malformed-weight", &scope, true),
        ExitStatus::InvalidInput,
    );
    keycloak
        .batch_execute("UPDATE public.user_attribute SET value='100001' WHERE id='bad-weight';")
        .await
        .unwrap();
    assert_exit(
        invoke(&root.0, "weight-limit", &scope, true),
        ExitStatus::InvalidInput,
    );
    keycloak
        .batch_execute("DELETE FROM public.user_attribute WHERE id='bad-weight';")
        .await
        .unwrap();
    scope["batches"] = json!([10]);

    hasura.batch_execute("UPDATE sequent_backend.tally_session SET configuration =
        '{\"contest_encryption_policy\":\"multiple-contests\",\"delegated_voting_policy\":\"enabled\"}';").await.unwrap();
    scope["scope"]["policy"] = json!("delegated");
    assert_success(invoke(&root.0, "delegated", &scope, true));
    assert_eq!(
        load(&root.0, "delegated", "eligible-voters.json")["voters"][0]["multiplicity"],
        5
    );
    let delegated_source = load(&root.0, "delegated", "keycloak-source.json");
    assert_eq!(delegated_source["delegations"].as_array().unwrap().len(), 2);
    assert_eq!(
        delegated_source["cross_realm_delegation_counts"],
        json!([{"delegate_user_id":"alice","counted_delegations":2}])
    );
    assert!(!delegated_source.to_string().contains("outside"));
    let diagnostics =
        &load(&root.0, "delegated", "eligibility-reasons.json")["delegation_diagnostics"];
    assert_eq!(diagnostics["cross_realm_delegations"], 2);
    assert_eq!(diagnostics["not_enabled_delegations"], 1);

    hasura
        .batch_execute("UPDATE sequent_backend.keys_ceremony SET is_default=false;")
        .await
        .unwrap();
    let result = invoke(&root.0, "non-default", &scope, true);
    assert!(String::from_utf8_lossy(&result.stderr).contains("non-default"));
    assert_exit(result, ExitStatus::InvalidInput);
    hasura
        .batch_execute("UPDATE sequent_backend.keys_ceremony SET is_default=true;")
        .await
        .unwrap();
    fs::write(&blob_paths[0], b"mismatch").unwrap();
    let result = invoke(&root.0, "blob-conflict", &scope, true);
    assert!(String::from_utf8_lossy(&result.stderr).contains("disagree"));
    assert_exit(result, ExitStatus::VerificationFailed);

    fs::write(
        &blob_paths[0],
        STANDARD_NO_PAD.decode(&expected_bytes[0]).unwrap(),
    )
    .unwrap();
    fs::write(&blob_paths[1], b"invalid-borsh").unwrap();
    assert_exit(
        invoke(&root.0, "corrupt-blob", &scope, true),
        ExitStatus::InvalidInput,
    );
    fs::write(
        &blob_paths[1],
        STANDARD_NO_PAD.decode(&expected_bytes[1]).unwrap(),
    )
    .unwrap();

    hasura
        .batch_execute(
            "ALTER TABLE sequent_backend.cast_vote ENABLE ROW LEVEL SECURITY;
        CREATE POLICY hide_all ON sequent_backend.cast_vote USING (false);",
        )
        .await
        .unwrap();
    let result = invoke(&root.0, "rls-filtered", &scope, true);
    assert_exit(result, ExitStatus::InvalidInput);
    assert!(!root.0.join("rls-filtered/acquisition.json").exists());
    hasura
        .batch_execute("ALTER TABLE sequent_backend.cast_vote DISABLE ROW LEVEL SECURITY;")
        .await
        .unwrap();
    b3.batch_execute("UPDATE public.index SET message_count=999;")
        .await
        .unwrap();
    assert_success(invoke(&root.0, "stale-count", &scope, true));
    let diagnostic = load(&root.0, "stale-count", "acquisition.json");
    assert_eq!(diagnostic["board_index_count"]["matches"], false);
    assert_eq!(diagnostic["board_index_count"]["observed"], 2);
    assert_eq!(diagnostic["board_index_count"]["exported"], 2);
    assert_eq!(diagnostic["board_index_count"]["stored"], 999);

    let count: i64 = hasura
        .query_one("SELECT count(*) FROM sequent_backend.cast_vote", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(count, 7);
    let count: i64 = keycloak
        .query_one("SELECT count(*) FROM public.user_entity", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(count, 11);
    let count: i64 = b3
        .query_one("SELECT count(*) FROM public.audit_board", &[])
        .await
        .unwrap()
        .get(0);
    assert_eq!(count, 2);

    let long_board = format!("{}suffix", "A".repeat(63));
    let relation = "a".repeat(63);
    b3.batch_execute(&format!(
        "ALTER TABLE public.audit_board RENAME TO \"{relation}\";"
    ))
    .await
    .unwrap();
    b3.execute(
        "UPDATE public.index SET board_name=$1, message_count=2",
        &[&long_board],
    )
    .await
    .unwrap();
    hasura
        .execute(
            "UPDATE sequent_backend.election_event SET bulletin_board_reference=$1",
            &[&json!({"database_name":long_board})],
        )
        .await
        .unwrap();
    fs::rename(
        root.0.join("blobs").join(BOARD),
        root.0.join("blobs").join(&long_board),
    )
    .unwrap();
    scope["scope"]["board_name"] = json!(long_board);
    assert_success(invoke(&root.0, "long-board", &scope, true));
    let collision = format!("{}other", "a".repeat(63));
    b3.execute("INSERT INTO public.index VALUES (2,$1,2)", &[&collision])
        .await
        .unwrap();
    assert_exit(
        invoke(&root.0, "colliding-board", &scope, true),
        ExitStatus::InvalidInput,
    );

    if let Ok(input) = std::env::var("BALLOT_AUDIT_SIGNED_FIXTURE_DIR") {
        let log = std::env::var("BALLOT_AUDIT_SIGNED_ELECTORAL_LOG")
            .expect("native ImmuDB acquisition file is required");
        let output = std::env::var("BALLOT_AUDIT_SIGNED_DEMO_OUTPUT")
            .expect("new signed demonstration directory is required");
        signed_database_pipeline(
            &hasura,
            &keycloak,
            &b3,
            &root.0,
            Path::new(&input),
            Path::new(&log),
            Path::new(&output),
        )
        .await;
    }
}

/// audit-tally must derive, from the tally ID alone, the same scope that the
/// hand-written fixture pins, and the real export must accept it.
fn tally_run_derives_the_pinned_scope(root: &Path, scope: &Value) {
    let log = root.join("tally-log");
    fs::create_dir(&log).unwrap();
    write_json(&log.join("electoral-log.json"), &json!([]));
    let output = root.join("tally");
    let result = Command::new(env!("CARGO_BIN_EXE_audit-tally"))
        .arg("--connections")
        .arg(root.join("connections.json"))
        .args(["--tally-session-id", SESSION, "--public-key-sha512"])
        .arg(scope["scope"]["public_key_sha512"].as_str().unwrap())
        .arg("--electoral-log-system-public-key")
        .arg(
            scope["scope"]["electoral_log_system_public_key"]
                .as_str()
                .unwrap(),
        )
        .arg("--electoral-log-export")
        .arg(&log)
        .arg("--b3-blob-root")
        .arg(root.join("blobs"))
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert_ne!(result.status.code(), Some(0));
    let derived: Value =
        serde_json::from_slice(&fs::read(output.join("scopes/001/scope.json")).unwrap()).unwrap();
    assert_eq!(&derived, scope);
    let record: Value =
        serde_json::from_slice(&fs::read(output.join("audit-record.json")).unwrap()).unwrap();
    let steps = record["steps"].as_array().unwrap();
    let export = steps
        .iter()
        .find(|step| step["step"] == "export-ballot-audit")
        .unwrap();
    assert_eq!(export["status"], "success", "{export}");
    assert!(export["outputs"]["scopes/001/postgres-export/acquisition.json"].is_string());
    assert!(steps
        .iter()
        .any(|step| step["step"] == "verify" && step["status"] == "not_run"));
    let summary = fs::read_to_string(output.join("public-summary.json")).unwrap();
    for private in ["\"alice\"", "\"receipt\"", "synthetic-reader"] {
        assert!(
            !summary.contains(private),
            "public summary contains {private}"
        );
    }
    let summary: Value = serde_json::from_str(&summary).unwrap();
    assert_eq!(summary["scopes"][0]["area_id"], AREA);
    assert_eq!(summary["braid"]["tally_batches"], json!([10]));
    assert_eq!(summary["braid"]["board_ballot_batches"], json!([10]));
}

async fn signed_database_pipeline(
    hasura: &Client,
    keycloak: &Client,
    b3: &Client,
    root: &Path,
    input: &Path,
    acquired_log: &Path,
    output: &Path,
) {
    fs::create_dir(output).unwrap();
    fs::set_permissions(output, fs::Permissions::from_mode(0o700)).unwrap();
    let mut scope: Value =
        serde_json::from_slice(&fs::read(input.join("scope.json")).unwrap()).unwrap();
    assert_eq!(scope["scope"]["tenant_id"], TENANT);
    assert_eq!(scope["scope"]["election_event_id"], EVENT);
    assert_eq!(scope["scope"]["election_id"], ELECTION);
    assert_eq!(scope["scope"]["area_id"], AREA);
    assert_eq!(scope["scope"]["tally_session_id"], SESSION);
    assert_eq!(scope["scope"]["tally_session_contest_id"], MAPPING);
    assert_eq!(scope["scope"]["policy"], "unweighted");
    scope["scope"]["board_name"] = json!("synthetic_board");
    let contest = scope["scope"]["contest_id"].as_str().unwrap();
    hasura.batch_execute("DELETE FROM sequent_backend.cast_vote;
        UPDATE sequent_backend.tally_session SET configuration = '{\"contest_encryption_policy\":\"single-contest\"}';").await.unwrap();
    hasura
        .execute(
            "UPDATE sequent_backend.election_event SET bulletin_board_reference=$1",
            &[&json!({"database_name":"synthetic_board"})],
        )
        .await
        .unwrap();
    hasura.execute("UPDATE sequent_backend.tally_session_contest SET contest_id=$1::text::uuid, annotations='{}'", &[&contest]).await.unwrap();
    let mut votes: Vec<Value> =
        serde_json::from_slice(&fs::read(input.join("cast-votes.json")).unwrap()).unwrap();
    let selected = votes
        .iter()
        .find(|row| row["id"] == "00000000-0000-4000-8000-000000000002")
        .unwrap()
        .clone();
    for (id, voter, status, date) in [
        (
            "00000000-0000-4000-8000-000000000081",
            "synthetic-disabled-voter",
            "valid",
            "01/10/2026",
        ),
        (
            "00000000-0000-4000-8000-000000000082",
            "synthetic-discarded-voter",
            "discarded",
            "02/10/2026",
        ),
    ] {
        let mut row = selected.clone();
        let mut ballot: sequent_core::ballot::HashableBallot =
            serde_json::from_str(selected["content"].as_str().unwrap()).unwrap();
        ballot.issue_date = date.into();
        row["id"] = json!(id);
        row["voter_id_string"] = json!(voter);
        row["status"] = json!(status);
        row["content"] = json!(serde_json::to_string(&ballot).unwrap());
        row["ballot_id"] = json!(sequent_core::encrypt::hash_ballot(&ballot).unwrap());
        votes.push(row);
    }
    for vote in &votes {
        hasura.execute("INSERT INTO sequent_backend.cast_vote SELECT * FROM jsonb_populate_record(NULL::sequent_backend.cast_vote,$1)",
            &[vote]).await.unwrap();
    }
    keycloak
        .batch_execute("DELETE FROM public.user_attribute; DELETE FROM public.user_entity;")
        .await
        .unwrap();
    let eligibility: Value =
        serde_json::from_slice(&fs::read(input.join("eligible-voters.json")).unwrap()).unwrap();
    for (i, voter) in eligibility["voters"].as_array().unwrap().iter().enumerate() {
        assert_eq!(voter["multiplicity"], 1);
        let id = voter["voter_id"].as_str().unwrap();
        keycloak
            .execute(
                "INSERT INTO public.user_entity VALUES ($1,'event-realm',$1,true,NULL)",
                &[&id],
            )
            .await
            .unwrap();
        keycloak
            .execute(
                "INSERT INTO public.user_attribute VALUES ($1,$2,'area-id',$3)",
                &[&format!("signed-area-{i}"), &id, &AREA],
            )
            .await
            .unwrap();
    }
    for (id, enabled) in [
        ("synthetic-disabled-voter", false),
        ("synthetic-discarded-voter", true),
    ] {
        keycloak
            .execute(
                "INSERT INTO public.user_entity VALUES ($1,'event-realm',$1,$2,NULL)",
                &[&id, &enabled],
            )
            .await
            .unwrap();
        keycloak
            .execute(
                "INSERT INTO public.user_attribute VALUES ($1,$2,'area-id',$3)",
                &[&format!("area-{id}"), &id, &AREA],
            )
            .await
            .unwrap();
    }
    b3.batch_execute("CREATE TABLE public.synthetic_board (id bigint PRIMARY KEY, sender_pk text, statement_kind text,
        batch integer, mix_number integer, message bytea, version text);
        GRANT SELECT ON public.synthetic_board TO audit_reader;
        DELETE FROM public.index;").await.unwrap();
    let encoded: Vec<String> =
        serde_json::from_slice(&fs::read(input.join("board-messages.json")).unwrap()).unwrap();
    for (i, value) in encoded.iter().enumerate() {
        let bytes = STANDARD_NO_PAD.decode(value).unwrap();
        let message = Message::strand_deserialize(&bytes).unwrap();
        b3.execute(
            "INSERT INTO public.synthetic_board VALUES ($1,$2,$3,$4,$5,$6,'test')",
            &[
                &((i + 1) as i64),
                &message.sender.pk.to_der_b64_string().unwrap(),
                &message.statement.get_kind().to_string(),
                &(message.statement.get_batch_number() as i32),
                &(message.statement.get_mix_number() as i32),
                &bytes,
            ],
        )
        .await
        .unwrap();
    }
    b3.execute(
        "INSERT INTO public.index VALUES (1,'synthetic_board',$1)",
        &[&(encoded.len() as i32)],
    )
    .await
    .unwrap();
    let postgres = output.join("postgres");
    assert_success(invoke(root, postgres.to_str().unwrap(), &scope, false));
    let exported = load(output, "postgres", "cast-votes.json");
    assert_eq!(exported.as_array().unwrap().len(), 5);
    assert_eq!(
        load(output, "postgres", "cast-vote-history.json")
            .as_array()
            .unwrap()
            .len(),
        6
    );
    for row in exported.as_array().unwrap() {
        let original = votes
            .iter()
            .find(|original| original["id"] == row["id"])
            .unwrap();
        assert_eq!(row["content"], original["content"]);
    }
    assert_eq!(
        load(output, "postgres", "board-messages.json"),
        json!(encoded)
    );
    let native_log: Value = serde_json::from_slice(&fs::read(acquired_log).unwrap()).unwrap();
    let expected_log: Value =
        serde_json::from_slice(&fs::read(input.join("electoral-log.json")).unwrap()).unwrap();
    assert_eq!(native_log, expected_log);
    let prepared = output.join("prepared");
    assert_success(prepare_acquired(&postgres, acquired_log, &prepared));
    let source_reasons = load(output, "postgres", "eligibility-reasons.json");
    assert_eq!(source_reasons["source_users"], 3);
    assert_eq!(source_reasons["eligible_users"], 2);
    assert_eq!(source_reasons["reason_counts"]["not_enabled"], 1);
    let selection = load(output, "prepared", "selection.json");
    assert_eq!(selection["accounting"]["exported_rows"], 5);
    assert_eq!(
        selection["accounting"]["rows_by_reason"]["selected"]["rows"],
        1
    );
    assert_eq!(
        selection["accounting"]["rows_by_reason"]["superseded_revote"]["rows"],
        2
    );
    assert_eq!(
        selection["accounting"]["stage_exclusions"]["no_eligible_voter"],
        1
    );
    assert_eq!(selection["accounting"]["stage_exclusions"]["discarded"], 1);
    for row in selection["rows"].as_array().unwrap() {
        if row["reason"] != "selected" {
            assert_eq!(row["receipt_verification"], "not_checked");
        }
    }
    let anchors = load(output, "prepared", "anchors.json");
    let result = Command::new(env!("CARGO_BIN_EXE_ballot-audit"))
        .arg("--manifest")
        .arg(prepared.join("manifest.json"))
        .arg("--manifest-sha512")
        .arg(anchors["manifest_sha512"].as_str().unwrap())
        .arg("--configuration")
        .arg(prepared.join("configuration.borsh"))
        .arg("--configuration-sha512")
        .arg(anchors["configuration_sha512"].as_str().unwrap())
        .arg("--cast-votes")
        .arg(prepared.join("cast-votes.json"))
        .arg("--messages")
        .arg(prepared.join("messages.json"))
        .arg("--electoral-log")
        .arg(prepared.join("electoral-log.json"))
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "signed DB pipeline audit failed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["checked_cast_votes"], 1);
    assert_eq!(report["checked_batches"], 1);
    assert_eq!(report["checked_electoral_log_messages"], 1);
    write_json(&output.join("audit-report.json"), &report);
    let selected_id = selected["id"].as_str().unwrap();
    let original_content = selected["content"].as_str().unwrap();
    let mut tampered: Value = serde_json::from_str(original_content).unwrap();
    tampered["issue_date"] = json!("01/01/1900");
    let tampered_content = serde_json::to_string(&tampered).unwrap();
    hasura
        .execute(
            "UPDATE sequent_backend.cast_vote SET content=$1 WHERE id=$2::text::uuid",
            &[&tampered_content, &selected_id],
        )
        .await
        .unwrap();
    let tampered_capture = output.join("postgres-tampered-content");
    assert_success(invoke(
        root,
        tampered_capture.to_str().unwrap(),
        &scope,
        false,
    ));
    let tampered_result = prepare_acquired(
        &tampered_capture,
        acquired_log,
        &output.join("rejected-tampered-content"),
    );
    assert!(!tampered_result.status.success());
    let content_error = String::from_utf8(tampered_result.stderr).unwrap();
    assert!(
        content_error.contains("no matching signed electoral-log evidence"),
        "{content_error}"
    );
    hasura
        .execute(
            "UPDATE sequent_backend.cast_vote SET content=$1 WHERE id=$2::text::uuid",
            &[&original_content, &selected_id],
        )
        .await
        .unwrap();
    b3.batch_execute(
        "DELETE FROM public.synthetic_board WHERE statement_kind='Ballots' AND batch=10;",
    )
    .await
    .unwrap();
    let missing_capture = output.join("postgres-missing-batch");
    assert_success(invoke(
        root,
        missing_capture.to_str().unwrap(),
        &scope,
        false,
    ));
    let missing_result = prepare_acquired(
        &missing_capture,
        acquired_log,
        &output.join("rejected-missing-batch"),
    );
    assert!(!missing_result.status.success());
    let missing_error = String::from_utf8(missing_result.stderr).unwrap();
    assert!(
        missing_error.contains("generated bundle failed complete input audit"),
        "{missing_error}"
    );
    let prepared_rows: Vec<ballot_audit::CastVote> =
        serde_json::from_slice(&fs::read(prepared.join("cast-votes.json")).unwrap()).unwrap();
    let prepared_logs: Vec<String> =
        serde_json::from_slice(&fs::read(prepared.join("electoral-log.json")).unwrap()).unwrap();
    let prepared_logs: Vec<Vec<u8>> = prepared_logs
        .iter()
        .map(|value| STANDARD_NO_PAD.decode(value).unwrap())
        .collect();
    let missing_messages: Vec<String> =
        serde_json::from_slice(&fs::read(missing_capture.join("board-messages.json")).unwrap())
            .unwrap();
    let scoped_messages: Vec<Vec<u8>> = missing_messages
        .iter()
        .map(|value| STANDARD_NO_PAD.decode(value).unwrap())
        .filter(|bytes| {
            let message = Message::strand_deserialize(bytes).unwrap();
            message.statement.get_kind().to_string() == "Ballots"
                && message.statement.get_batch_number() == 10
        })
        .collect();
    assert!(scoped_messages.is_empty());
    let core_error = ballot_audit::audit(
        &fs::read(prepared.join("manifest.json")).unwrap(),
        anchors["manifest_sha512"].as_str().unwrap(),
        &prepared_rows,
        &fs::read(prepared.join("configuration.borsh")).unwrap(),
        anchors["configuration_sha512"].as_str().unwrap(),
        &scoped_messages,
        None,
        &prepared_logs,
    )
    .unwrap_err()
    .to_string();
    assert_eq!(core_error, "missing B3 batch");
    write_json(
        &output.join("negative-checks.json"),
        &json!({
            "tampered_content": {"database_change":"selected ballot issue_date; receipt, signed log and saved scope unchanged",
                "fresh_acquisition": "postgres-tampered-content", "source_sha512_recomputed":true,
                "exit_code":tampered_result.status.code(), "error":content_error.trim()},
            "missing_batch": {"database_change":"removed scoped signed Ballots message; saved scope and remaining signed bytes unchanged",
                "fresh_acquisition":"postgres-missing-batch", "source_sha512_recomputed":true,
                "exit_code":missing_result.status.code(), "error":missing_error.trim(),
                "independent_core_error":core_error}
        }),
    );
    println!("Signed database acquisition, eligibility/exclusions, preparation, audit and cryptographic negative cases passed: {}", output.display());
}

fn prepare_acquired(postgres: &Path, acquired_log: &Path, output: &Path) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_prepare-ballot-audit"));
    for (option, file) in [
        ("scope", "scope.json"),
        ("cast-votes", "cast-votes.json"),
        ("eligible-voters", "eligible-voters.json"),
        ("board-messages", "board-messages.json"),
    ] {
        let path = postgres.join(file);
        command
            .arg(format!("--{option}"))
            .arg(&path)
            .arg(format!("--{option}-sha512"))
            .arg(digest(&fs::read(path).unwrap()).unwrap());
    }
    command
        .arg("--electoral-log")
        .arg(acquired_log)
        .arg("--electoral-log-sha512")
        .arg(digest(&fs::read(acquired_log).unwrap()).unwrap())
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}

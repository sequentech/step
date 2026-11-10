// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Minimal rows on the migrated test database. Every event gets a fresh
//! tenant unless the test names one, so tests never see each other's rows.

use deadpool_postgres::Pool;
use electoral_log::adapters::ballot_box::{
    AcceptBallot, AcceptOutcome, BallotStatus,
};
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::messages::message::{Message, SigningData};
use electoral_log::messages::newtypes::{
    CastVoteHash, ElectionIdString, EventIdString, PseudonymHash,
    VoterCountryString, VoterIpString,
};
use electoral_log::ports::ElectoralLogStore;
use electoral_log::{ElectoralLogMessage, LogEntry};
use sequent_core::signing::{
    RequesterSigning, RevocationStatus, SignatureAlgorithm, SigningAction,
    SigningRequirement, SigningRule, StaffCertificateRegistration,
};
use serde_json::json;
use std::future::Future;
use std::sync::OnceLock;
use tokio::runtime::Runtime;
use tokio_postgres::types::ToSql;
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use windmill::postgres::signing::{
    insert_signing_approval, insert_staff_certificate,
    update_signing_request_status, upsert_signing_rule, NewSigningApproval,
    NewStaffCertificate, SigningRequestTransition,
};
use windmill::services::ballot_box::BallotFormat;
use windmill::services::electoral_log_board::get_event_databases;

/// Runs a test that reaches election events' electoral-log databases on the
/// runtime those tests share. The services keep the event databases'
/// connections in one pool per process, and a connection dies with the
/// runtime that opened it, so such a test must not run on a runtime of its
/// own that ends while another test still uses its connections.
pub fn run<F: Future>(test: F) -> F::Output {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("the event database tests' runtime")
        })
        .block_on(test)
}

pub struct Event {
    pub tenant_id: String,
    pub election_event_id: String,
}

/// Run one statement on its own connection.
pub async fn execute(pool: &Pool, sql: &str, params: &[&(dyn ToSql + Sync)]) {
    pool.get()
        .await
        .expect("test database connection")
        .execute(sql, params)
        .await
        .unwrap_or_else(|error| panic!("{sql}: {error:?}"));
}

/// The rows a query returns, on its own connection.
pub async fn query(
    pool: &Pool,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
) -> Vec<tokio_postgres::Row> {
    pool.get()
        .await
        .expect("test database connection")
        .query(sql, params)
        .await
        .unwrap_or_else(|error| panic!("{sql}: {error:?}"))
}

fn uuid(id: &str) -> Uuid {
    Uuid::parse_str(id).expect("fixture ids are UUIDs")
}

/// A tenant with one election event.
pub async fn event(pool: &Pool) -> Event {
    event_of(pool, &Uuid::new_v4().to_string()).await
}

/// An election event of `tenant_id`, which is created unless it exists.
pub async fn event_of(pool: &Pool, tenant_id: &str) -> Event {
    let tenant_id = uuid(tenant_id);
    let election_event_id = Uuid::new_v4();
    execute(
        pool,
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        &[&tenant_id, &tenant_id.to_string()],
    )
    .await;
    execute(
        pool,
        "INSERT INTO sequent_backend.election_event
            (id, tenant_id, encryption_protocol)
         VALUES ($1, $2, 'RSA256')",
        &[&election_event_id, &tenant_id],
    )
    .await;
    Event {
        tenant_id: tenant_id.to_string(),
        election_event_id: election_event_id.to_string(),
    }
}

impl Event {
    fn ids(&self) -> (Uuid, Uuid) {
        (uuid(&self.tenant_id), uuid(&self.election_event_id))
    }

    pub async fn election(&self, pool: &Pool) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let id = Uuid::new_v4();
        execute(
            pool,
            "INSERT INTO sequent_backend.election
                (id, tenant_id, election_event_id)
             VALUES ($1, $2, $3)",
            &[&id, &tenant_id, &election_event_id],
        )
        .await;
        id.to_string()
    }

    /// An election that only admins whose permission labels include `label`
    /// may see.
    pub async fn labelled_election(&self, pool: &Pool, label: &str) -> String {
        let id = self.election(pool).await;
        execute(
            pool,
            "UPDATE sequent_backend.election SET permission_label = $2
             WHERE id = $1",
            &[&uuid(&id), &label],
        )
        .await;
        id
    }

    /// A contest of `election_id` with the given counting algorithm.
    pub async fn contest(
        &self,
        pool: &Pool,
        election_id: &str,
        counting_algorithm: &str,
    ) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let id = Uuid::new_v4();
        execute(
            pool,
            "INSERT INTO sequent_backend.contest
                (id, tenant_id, election_event_id, election_id,
                 counting_algorithm, max_votes, external_id)
             VALUES ($1, $2, $3, $4, $5, 1, $6)",
            &[
                &id,
                &tenant_id,
                &election_event_id,
                &uuid(election_id),
                &counting_algorithm,
                &id.to_string(),
            ],
        )
        .await;
        id.to_string()
    }

    pub async fn area(&self, pool: &Pool, name: &str) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let id = Uuid::new_v4();
        execute(
            pool,
            "INSERT INTO sequent_backend.area
                (id, tenant_id, election_event_id, name)
             VALUES ($1, $2, $3, $4)",
            &[&id, &tenant_id, &election_event_id, &name],
        )
        .await;
        id.to_string()
    }

    /// Put `contest_id` on the ballots of `area_id`.
    pub async fn area_contest(
        &self,
        pool: &Pool,
        area_id: &str,
        contest_id: &str,
    ) {
        let (tenant_id, election_event_id) = self.ids();
        execute(
            pool,
            "INSERT INTO sequent_backend.area_contest
                (id, tenant_id, election_event_id, area_id, contest_id)
             VALUES (gen_random_uuid(), $1, $2, $3, $4)",
            &[
                &tenant_id,
                &election_event_id,
                &uuid(area_id),
                &uuid(contest_id),
            ],
        )
        .await;
    }

    /// A candidate whose external id is its id.
    pub async fn candidate(&self, pool: &Pool, contest_id: &str) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let id = Uuid::new_v4();
        execute(
            pool,
            "INSERT INTO sequent_backend.candidate
                (id, tenant_id, election_event_id, contest_id, external_id)
             VALUES ($1, $2, $3, $4, $5)",
            &[
                &id,
                &tenant_id,
                &election_event_id,
                &uuid(contest_id),
                &id.to_string(),
            ],
        )
        .await;
        id.to_string()
    }

    /// Merge `presentation` into the election event's presentation.
    /// A fixture, so it may set the lockdown as the server does.
    pub async fn present(&self, pool: &Pool, presentation: serde_json::Value) {
        let (_, election_event_id) = self.ids();
        execute(
            pool,
            "UPDATE sequent_backend.election_event
             SET presentation = coalesce(presentation, '{}'::jsonb) || $2
             WHERE id = $1 AND set_config('sequent.trusted_write', 'on', true) = 'on'",
            &[&election_event_id, &presentation],
        )
        .await;
    }

    /// A report of `report_type` rendered with the template `alias`.
    pub async fn report(
        &self,
        pool: &Pool,
        report_type: &str,
        template_alias: &str,
    ) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let id = Uuid::new_v4();
        execute(
            pool,
            "INSERT INTO sequent_backend.report
                (id, tenant_id, election_event_id, report_type, template_alias)
             VALUES ($1, $2, $3, $4, $5)",
            &[
                &id,
                &tenant_id,
                &election_event_id,
                &report_type,
                &template_alias,
            ],
        )
        .await;
        id.to_string()
    }

    /// A tenant template stored under `alias`.
    pub async fn template(
        &self,
        pool: &Pool,
        alias: &str,
        template: serde_json::Value,
    ) {
        let (tenant_id, _) = self.ids();
        execute(
            pool,
            "INSERT INTO sequent_backend.template
                (tenant_id, alias, template, created_by, communication_method, type)
             VALUES ($1, $2, $3, 'fixture', 'EMAIL', 'CREDENTIALS')",
            &[&tenant_id, &alias, &template],
        )
        .await;
    }

    /// A ballot publication; `published` also gives it the object root a
    /// publication's files live under.
    pub async fn ballot_publication(
        &self,
        pool: &Pool,
        is_generated: bool,
        published: bool,
    ) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let id = Uuid::new_v4();
        let annotations = published.then(|| {
            serde_json::json!({
                windmill::domain::publication_files::FILES_ANNOTATION:
                    windmill::domain::publication_files::publication_root(
                        tenant_id,
                        election_event_id,
                        id,
                        Uuid::new_v4(),
                    )
            })
        });
        execute(
            pool,
            "INSERT INTO sequent_backend.ballot_publication
                (id, tenant_id, election_event_id, is_generated,
                 published_at, annotations)
             VALUES ($1, $2, $3, $4,
                CASE WHEN $5 THEN now() END, $6)",
            &[
                &id,
                &tenant_id,
                &election_event_id,
                &is_generated,
                &published,
                &annotations,
            ],
        )
        .await;
        id.to_string()
    }

    /// The board [`Event::ballot_box`] names on the event's row.
    pub fn board(&self) -> String {
        let (tenant_id, election_event_id) = self.ids();
        format!(
            "tenant{}event{}",
            tenant_id.simple(),
            election_event_id.simple()
        )
    }

    /// The event's ballot box, in an electoral-log database of its own on the
    /// server `ELECTORAL_LOG_PG_*` names, and its board on the event's row.
    pub async fn ballot_box(&self, pool: &Pool) -> PostgresStore {
        let (_, election_event_id) = self.ids();
        let board = self.board();
        execute(
            pool,
            "UPDATE sequent_backend.election_event
             SET bulletin_board_reference = $2 WHERE id = $1",
            &[
                &election_event_id,
                &json!({"id": 1, "database_name": board, "is_archived": false}),
            ],
        )
        .await;
        let store = get_event_databases()
            .await
            .expect("ELECTORAL_LOG_PG_* must name the test PostgreSQL server")
            .create_event(&self.tenant_id, &self.election_event_id)
            .await
            .expect("event database");
        store
            .create_ballot_box(&self.election_event_id)
            .await
            .expect("ballot box");
        store
    }

    /// A valid vote `voter` cast in `election_id` from `area_id`, accepted
    /// into the event's `ballot_box`.
    pub async fn cast_vote(
        &self,
        ballot_box: &PostgresStore,
        election_id: &str,
        area_id: &str,
        voter: &str,
    ) {
        let ballot_id = Uuid::new_v4().simple().to_string();
        let format = BallotFormat::HashableBallot.to_string();
        let outcome = ballot_box
            .accept_ballot(&AcceptBallot {
                election_event_id: &self.election_event_id,
                election_id,
                area_id,
                voter_id: voter,
                ballot_id: &ballot_id,
                format: &format,
                content: "ballot",
                voter_signature: None,
                admission_record: &format!(r#"{{"payload":{{"accepted_at":"{}","sequence":{}}},"signature":""}}"#, chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as i64).into_bytes(),
                pseudonym_hash: &[0; 64],
                ballot_hash: &[0; 64],
                voting_channel: "ONLINE",
                status: BallotStatus::Valid,
                voter_ip: Some("192.0.2.1"),
                voter_country: Some("ES"),
                username: Some(voter),
                allowed_votes: 0,
            })
            .await
            .expect("accepted ballot");
        assert!(
            matches!(outcome, AcceptOutcome::Accepted { .. }),
            "{outcome:?}"
        );
    }

    /// The signed record of a vote `voter` cast in `election_id`, appended to
    /// `board` in the event's electoral-log database `store`. Returns its
    /// position.
    pub async fn cast_vote_record(
        &self,
        store: &PostgresStore,
        board: &str,
        election_id: &str,
        voter: &str,
    ) -> i64 {
        let key = SigningKey::generate().expect("signing key");
        let message = Message::cast_vote_message(
            EventIdString(self.election_event_id.clone()),
            ElectionIdString(Some(election_id.to_string())),
            PseudonymHash::new([1; 64]),
            CastVoteHash::new([2; 64]),
            &SigningData::new(key.clone(), "voting", key),
            VoterIpString("192.0.2.1".into()),
            VoterCountryString("ES".into()),
            Some(voter.to_string()),
            Some(voter.to_string()),
            String::new(),
        )
        .expect("cast vote message");
        let entry = LogEntry {
            delivery_id: Uuid::new_v4().to_string(),
            message: ElectoralLogMessage::try_from(&message)
                .expect("cast vote record"),
        };
        store.create_board(board).await.expect("board");
        store
            .append(board, &mut std::iter::once(Ok(entry)))
            .await
            .expect("appended record")
            .first()
            .expect("stored record")
            .id
    }

    /// The Keycloak realm of the event's voters.
    pub fn realm(&self) -> String {
        sequent_core::services::keycloak::get_event_realm(
            &self.tenant_id,
            &self.election_event_id,
        )
    }

    /// A document row; its contents live in object storage.
    pub async fn document(
        &self,
        pool: &Pool,
        name: &str,
        media_type: &str,
        size: Option<i64>,
    ) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let id = Uuid::new_v4();
        execute(
            pool,
            "INSERT INTO sequent_backend.document
                (id, tenant_id, election_event_id, name, media_type, size)
             VALUES ($1, $2, $3, $4, $5, $6)",
            &[
                &id,
                &tenant_id,
                &election_event_id,
                &name,
                &media_type,
                &size,
            ],
        )
        .await;
        id.to_string()
    }

    /// A tally session of `election_ids` whose tally finished with
    /// `execution_status`, with one recorded execution.
    pub async fn tally_session(
        &self,
        pool: &Pool,
        election_ids: &[String],
        execution_status: &str,
    ) -> String {
        let (tenant_id, election_event_id) = self.ids();
        let keys_ceremony_id = Uuid::new_v4();
        execute(
            pool,
            "INSERT INTO sequent_backend.keys_ceremony
                (id, tenant_id, election_event_id, trustee_ids, threshold)
             VALUES ($1, $2, $3, '{}', 1)",
            &[&keys_ceremony_id, &tenant_id, &election_event_id],
        )
        .await;
        let id = Uuid::new_v4();
        let election_uuids: Vec<Uuid> =
            election_ids.iter().map(|id| uuid(id)).collect();
        execute(
            pool,
            "INSERT INTO sequent_backend.tally_session
                (id, tenant_id, election_event_id, keys_ceremony_id,
                 threshold, election_ids, is_execution_completed,
                 execution_status, tally_type)
             VALUES ($1, $2, $3, $4, 1, $5, true, $6, 'ELECTORAL_RESULTS')",
            &[
                &id,
                &tenant_id,
                &election_event_id,
                &keys_ceremony_id,
                &election_uuids,
                &execution_status,
            ],
        )
        .await;
        execute(
            pool,
            "INSERT INTO sequent_backend.tally_session_execution
                (tenant_id, election_event_id, tally_session_id,
                 current_message_id, status)
             VALUES ($1, $2, $3, 1,
                '{\"logs\": [], \"trustees\": [], \"elections_status\": []}')",
            &[&tenant_id, &election_event_id, &id],
        )
        .await;
        id.to_string()
    }

    /// The executions recorded for a tally session, oldest first.
    pub async fn tally_session_executions(
        &self,
        pool: &Pool,
        tally_session_id: &str,
    ) -> usize {
        query(
            pool,
            "SELECT id FROM sequent_backend.tally_session_execution
             WHERE tally_session_id = $1",
            &[&uuid(tally_session_id)],
        )
        .await
        .len()
    }

    /// The authenticated configuration package the event was imported from.
    pub async fn configuration_package(&self, pool: &Pool) {
        let (tenant_id, election_event_id) = self.ids();
        // A tenant that imports packages has settings.
        execute(
            pool,
            "UPDATE sequent_backend.tenant SET settings = '{}' WHERE id = $1",
            &[&tenant_id],
        )
        .await;
        execute(
            pool,
            "INSERT INTO sequent_backend.configuration_package
                 (tenant_id, election_event_id, external_id, revision,
                  manifest_sha256, manifest, signer_subject, signer_serial,
                  signer_fingerprint, approvers)
             VALUES ($1, $2, 'synthetic', 1, repeat('0', 64), $3, 'CN=synthetic',
                     '1', repeat('0', 64), '[]')",
            &[
                &tenant_id,
                &election_event_id,
                &json!({
                    "format": "sequent.configuration-manifest/1",
                    "configuration": {
                        "external_id": "synthetic",
                        "name": "synthetic",
                        "revision": 1,
                        "previous": null,
                    },
                    "content_sha256": "0".repeat(64),
                    "content": {"files": [], "ballot_designs": [], "reports": []},
                    "approvals": [],
                    "produced": {
                        "at": "2026-01-01T00:00:00Z",
                        "custodian": "synthetic",
                        "key_label": "synthetic",
                        "producer": {},
                    },
                }),
            ],
        )
        .await;
    }

    /// The completed export of the event's current execution data.
    pub async fn current_export(&self, pool: &Pool) {
        let (tenant_id, election_event_id) = self.ids();
        execute(
            pool,
            "INSERT INTO sequent_backend.execution_export
                 (tenant_id, election_event_id, document_id, request_id,
                  source_revision, lifecycle_generation, manifest_sha256,
                  archive_sha256, signing_public_key)
             SELECT l.tenant_id, l.election_event_id, gen_random_uuid(),
                    gen_random_uuid(), v.revision, l.generation, repeat('0', 64),
                    repeat('0', 64), 'synthetic'
             FROM sequent_backend.election_lifecycle l
             JOIN sequent_backend.election_execution_revision v
               ON v.election_event_id = l.election_event_id
             WHERE l.tenant_id = $1 AND l.election_event_id = $2
               AND set_config('sequent.trusted_write', 'on', true) = 'on'",
            &[&tenant_id, &election_event_id],
        )
        .await;
    }

    /// The event's electoral phase becomes `phase`, as the server's workflow
    /// moves it.
    pub async fn enter_phase(&self, pool: &Pool, phase: &str) {
        let (tenant_id, election_event_id) = self.ids();
        execute(
            pool,
            "UPDATE sequent_backend.election_lifecycle SET phase = $3
             WHERE tenant_id = $1 AND election_event_id = $2
               AND set_config('sequent.trusted_write', 'on', true) = 'on'",
            &[&tenant_id, &election_event_id, &phase],
        )
        .await;
    }

    /// The rule that makes `action` wait for two board signatures.
    pub async fn board_rule(&self, pool: &Pool, action: SigningAction) {
        let (tenant_id, election_event_id) = self.ids();
        let mut client = pool.get().await.expect("test database connection");
        let tx = client.transaction().await.unwrap();
        upsert_signing_rule(
            &tx,
            tenant_id,
            election_event_id,
            &SigningRule {
                action,
                requirement: SigningRequirement::Required,
                signatures: 2,
                requester_signing: RequesterSigning::NotAllowed,
                expires_minutes: Some(60),
                revision: 0,
            },
            0,
            "manager",
            None,
        )
        .await
        .unwrap()
        .unwrap();
        tx.commit().await.unwrap();
    }

    /// Two board members, each with a certificate valid now, sign the waiting
    /// signing request `request`, which completes it.
    pub async fn board_signs(&self, pool: &Pool, request: Uuid) {
        let (tenant_id, election_event_id) = self.ids();
        let mut client = pool.get().await.expect("test database connection");
        let tx = client.transaction().await.unwrap();
        let now = chrono::Utc::now();
        for member in ["board-member-1", "board-member-2"] {
            let digest = || format!("{:064x}", Uuid::new_v4().as_u128());
            let certificate = insert_staff_certificate(
                &tx,
                &NewStaffCertificate {
                    tenant_id,
                    election_event_id,
                    user_id: member.to_string(),
                    username: member.to_string(),
                    user_display_name: None,
                    election_id: None,
                    fingerprint_sha256: digest(),
                    spki_sha256: digest(),
                    holder_sha256: digest(),
                    serial: member.to_string(),
                    subject: format!("CN={member}"),
                    issuer: "CN=synthetic".to_string(),
                    not_before: now - chrono::Duration::days(1),
                    not_after: now + chrono::Duration::days(1),
                    pem: "synthetic".to_string(),
                    registration: StaffCertificateRegistration::FirstUse,
                    linked_to: None,
                    registered_by: member.to_string(),
                    registered_by_name: None,
                },
            )
            .await
            .unwrap();
            insert_signing_approval(
                &tx,
                &NewSigningApproval {
                    tenant_id,
                    election_event_id,
                    request_id: request,
                    user_id: certificate.user_id.clone(),
                    username: certificate.username.clone(),
                    display_name: None,
                    auth_time: None,
                    certificate_id: certificate.id,
                    certificate_pem: certificate.pem.clone(),
                    chain_pem: certificate.pem.clone(),
                    fingerprint_sha256: certificate.fingerprint_sha256.clone(),
                    spki_sha256: certificate.spki_sha256.clone(),
                    holder_sha256: certificate.holder_sha256.clone(),
                    algorithm: SignatureAlgorithm::RsaPkcs1Sha256,
                    payload_signature: vec![1],
                    document_signature: None,
                    pdf_cms: None,
                    revocation_status: RevocationStatus::Checked,
                },
            )
            .await
            .unwrap()
            .unwrap();
        }
        update_signing_request_status(
            &tx,
            tenant_id,
            election_event_id,
            request,
            &SigningRequestTransition::Complete,
        )
        .await
        .unwrap()
        .expect("the request waits for its signatures");
        tx.commit().await.unwrap();
    }
}

/// A Keycloak realm named `name`; returns its id.
pub async fn keycloak_realm(pool: &Pool, name: &str) -> String {
    let id = Uuid::new_v4().to_string();
    execute(
        pool,
        "INSERT INTO realm (id, name) VALUES ($1, $2)",
        &[&id, &name],
    )
    .await;
    id
}

/// A Keycloak user of the realm with id `realm_id`; returns its id.
pub async fn keycloak_user(
    pool: &Pool,
    realm_id: &str,
    username: &str,
    enabled: bool,
) -> String {
    let id = Uuid::new_v4().to_string();
    execute(
        pool,
        "INSERT INTO user_entity (id, realm_id, username, enabled, email_verified)
         VALUES ($1, $2, $3, $4, true)",
        &[&id, &realm_id, &username, &enabled],
    )
    .await;
    id
}

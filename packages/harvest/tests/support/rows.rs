// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Minimal rows on the migrated test database. Every event gets a fresh
//! tenant, so tests never see each other's rows.

use deadpool_postgres::Pool;
use tokio_postgres::types::ToSql;
use uuid::Uuid;

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
    let tenant_id = Uuid::new_v4();
    let election_event_id = Uuid::new_v4();
    execute(
        pool,
        "INSERT INTO sequent_backend.tenant (id, slug) VALUES ($1, $2)",
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
    pub async fn present(&self, pool: &Pool, presentation: serde_json::Value) {
        let (_, election_event_id) = self.ids();
        execute(
            pool,
            "UPDATE sequent_backend.election_event
             SET presentation = coalesce(presentation, '{}'::jsonb) || $2
             WHERE id = $1",
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

    /// A valid vote `voter` cast in `election_id` from `area_id`.
    pub async fn cast_vote(
        &self,
        pool: &Pool,
        election_id: &str,
        area_id: &str,
        voter: &str,
    ) {
        let (tenant_id, election_event_id) = self.ids();
        execute(
            pool,
            "INSERT INTO sequent_backend.cast_vote
                (id, tenant_id, election_event_id, election_id, area_id,
                 voter_id_string, status)
             VALUES (gen_random_uuid(), $1, $2, $3, $4, $5, 'valid')",
            &[
                &tenant_id,
                &election_event_id,
                &uuid(election_id),
                &uuid(area_id),
                &voter,
            ],
        )
        .await;
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

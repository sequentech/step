// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! PostgreSQL contract for election-scoped voter queries. It needs the
//! Hasura schema; each run creates private fixtures and rolls them back.

use super::*;

#[tokio::test]
#[ignore = "requires PostgreSQL configured through HASURA_DB__*"]
async fn election_scoped_voters_are_authorized_by_external_id_or_id() -> Result<()> {
    use sequent_core::services::keycloak::get_event_realm;

    let tenant = Uuid::new_v4();
    let event = Uuid::new_v4();
    let by_external_id = Uuid::new_v4();
    let by_id = Uuid::new_v4();
    let area = Uuid::new_v4();
    let realm = get_event_realm(&tenant.to_string(), &event.to_string());

    let pool = crate::services::database::generate_hasura_pool().await?;
    let mut client = pool.get().await?;
    let transaction = client.transaction().await?;
    transaction
        .batch_execute(&format!(
            r#"
            CREATE SCHEMA IF NOT EXISTS sequent_backend;
            CREATE TABLE IF NOT EXISTS sequent_backend.tenant (id UUID PRIMARY KEY, slug TEXT);
            CREATE TABLE IF NOT EXISTS sequent_backend.election_event (
                id UUID PRIMARY KEY, tenant_id UUID, encryption_protocol TEXT
            );
            CREATE TABLE IF NOT EXISTS sequent_backend.election (
                id UUID PRIMARY KEY, tenant_id UUID, election_event_id UUID,
                external_id TEXT, num_allowed_revotes INT, status JSONB, presentation JSONB
            );
            CREATE TABLE IF NOT EXISTS sequent_backend.area (
                id UUID PRIMARY KEY, tenant_id UUID, election_event_id UUID, name TEXT
            );
            CREATE TABLE IF NOT EXISTS sequent_backend.contest (
                id UUID PRIMARY KEY, tenant_id UUID, election_event_id UUID, election_id UUID
            );
            CREATE TABLE IF NOT EXISTS sequent_backend.area_contest (
                id UUID PRIMARY KEY, tenant_id UUID, election_event_id UUID,
                contest_id UUID, area_id UUID
            );
            INSERT INTO sequent_backend.tenant (id, slug) VALUES ('{tenant}', '{tenant}');
            INSERT INTO sequent_backend.election_event (id, tenant_id, encryption_protocol)
            VALUES ('{event}', '{tenant}', 'RSA256');
            INSERT INTO sequent_backend.election
                (id, tenant_id, election_event_id, external_id, num_allowed_revotes, status, presentation)
            VALUES
                ('{by_external_id}', '{tenant}', '{event}', 'election-a', 1, '{{}}',
                 '{{"i18n": {{"en": {{"alias": "Mayor"}}}}}}'),
                ('{by_id}', '{tenant}', '{event}', NULL, 1, '{{}}', NULL);
            INSERT INTO sequent_backend.area (id, tenant_id, election_event_id, name)
            VALUES ('{area}', '{tenant}', '{event}', 'Area');
            INSERT INTO sequent_backend.contest (id, tenant_id, election_event_id, election_id)
            VALUES (gen_random_uuid(), '{tenant}', '{event}', '{by_external_id}'),
                   (gen_random_uuid(), '{tenant}', '{event}', '{by_id}');
            INSERT INTO sequent_backend.area_contest (id, tenant_id, election_event_id, contest_id, area_id)
            SELECT gen_random_uuid(), tenant_id, election_event_id, id, '{area}'
            FROM sequent_backend.contest
            WHERE tenant_id = '{tenant}';

            CREATE TEMP TABLE realm (id TEXT PRIMARY KEY, name TEXT) ON COMMIT DROP;
            CREATE TEMP TABLE user_entity (
                id TEXT PRIMARY KEY,
                realm_id TEXT,
                username TEXT,
                email TEXT,
                email_verified BOOLEAN,
                enabled BOOLEAN,
                first_name TEXT,
                last_name TEXT,
                created_timestamp BIGINT,
                service_account_client_link TEXT
            ) ON COMMIT DROP;
            CREATE TEMP TABLE user_attribute (
                id SERIAL PRIMARY KEY,
                user_id TEXT,
                name TEXT,
                value TEXT
            ) ON COMMIT DROP;
            INSERT INTO realm (id, name) VALUES ('realm', '{realm}');
            INSERT INTO user_entity (id, realm_id, username, enabled, email_verified)
            VALUES ('unrestricted', 'realm', 'unrestricted', true, false),
                   ('external-id', 'realm', 'external-id', true, false),
                   ('election-id', 'realm', 'election-id', true, false),
                   ('display-alias', 'realm', 'display-alias', true, false),
                   ('null-value', 'realm', 'null-value', true, false),
                   ('null-and-key', 'realm', 'null-and-key', true, false),
                   ('no-area', 'realm', 'no-area', true, false);
            INSERT INTO user_attribute (user_id, name, value)
            VALUES ('unrestricted', '{AREA_ID_ATTR_NAME}', '{area}'),
                   ('external-id', '{AREA_ID_ATTR_NAME}', '{area}'),
                   ('external-id', '{AUTHORIZED_ELECTION_IDS_NAME}', 'election-a'),
                   ('election-id', '{AREA_ID_ATTR_NAME}', '{area}'),
                   ('election-id', '{AUTHORIZED_ELECTION_IDS_NAME}', '{by_id}'),
                   ('display-alias', '{AREA_ID_ATTR_NAME}', '{area}'),
                   ('display-alias', '{AUTHORIZED_ELECTION_IDS_NAME}', 'Mayor'),
                   ('null-value', '{AREA_ID_ATTR_NAME}', '{area}'),
                   ('null-value', '{AUTHORIZED_ELECTION_IDS_NAME}', NULL),
                   ('null-and-key', '{AREA_ID_ATTR_NAME}', '{area}'),
                   ('null-and-key', '{AUTHORIZED_ELECTION_IDS_NAME}', NULL),
                   ('null-and-key', '{AUTHORIZED_ELECTION_IDS_NAME}', 'election-a'),
                   ('no-area', '{AUTHORIZED_ELECTION_IDS_NAME}', 'election-a');
            "#
        ))
        .await?;

    let voters = |election: Uuid, alias: Option<&str>| {
        let transaction = &transaction;
        let filter = ListUsersFilter {
            election_event_id: Some(event.to_string()),
            election_id: Some(election.to_string()),
            authorized_to_election_alias: alias.map(str::to_string),
            ..ListUsersFilter::new(&tenant.to_string(), &realm)
        };
        async move {
            let count = count_keycloak_users(transaction, transaction, filter.clone()).await?;
            let (users, total) = list_users(transaction, transaction, filter).await?;
            assert_eq!(total, count);
            let mut ids: Vec<String> = users.into_iter().filter_map(|user| user.id).collect();
            ids.sort();
            anyhow::Ok(ids)
        }
    };

    // The election's display alias is not what the attribute holds, so a
    // caller passing it must not hide the voters authorized by external id.
    // A NULL value authorizes the voter, as in the census, and a voter with
    // several values is listed once.
    let by_external_id_voters = vec!["external-id", "null-and-key", "null-value", "unrestricted"];
    assert_eq!(
        voters(by_external_id, Some("Mayor")).await?,
        by_external_id_voters
    );
    assert_eq!(voters(by_external_id, None).await?, by_external_id_voters);
    // Without an external id, Keycloak names the election by its id.
    assert_eq!(
        voters(by_id, None).await?,
        vec!["election-id", "null-and-key", "null-value", "unrestricted"]
    );
    // An election that doesn't exist matches no voter instead of failing.
    assert!(voters(Uuid::new_v4(), None).await?.is_empty());
    assert!(voters(Uuid::new_v4(), Some("Mayor")).await?.is_empty());
    transaction.rollback().await?;
    Ok(())
}

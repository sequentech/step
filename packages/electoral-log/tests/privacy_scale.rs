// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Explicit storage smoke benchmark, not an HTTP/cast/cryptographic throughput test.
//! Run only against an isolated ELECTORAL_LOG_TEST_DATABASE_URL. The registry uses
//! the production allocator; SQL seeds synthetic ballot rows to exercise read indexes.

use anyhow::Result;
use electoral_log::adapters::postgres::PostgresStore;
use electoral_log::adapters::privacy::{BallotIdentity, PrivacyBallotQuery, PrivacyBallotSearch};
use std::time::Instant;
use tokio_postgres::types::ToSql;
use uuid::Uuid;

#[tokio::test]
#[ignore = "explicit large isolated database smoke benchmark"]
async fn privacy_registry_and_scoped_reads_at_large_event_shape() -> Result<()> {
    let voters: usize = std::env::var("PRIVACY_SCALE_VOTERS")
        .unwrap_or_else(|_| "1500000".into())
        .parse()?;
    let elections: i64 = 104;
    let sample_ballots = voters.min(104_000) as i64;
    let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
    store.initialize().await?;
    let event = Uuid::new_v4().to_string();
    store.create_ballot_box(&event).await?;
    store
        .set_ballot_identity(&event, BallotIdentity::Pseudonym)
        .await?;
    let started = Instant::now();
    for first in (0..voters).step_by(5_000) {
        let ids = (first..(first + 5_000).min(voters))
            .map(|id| format!("scale-voter-{id:08}"))
            .collect::<Vec<_>>();
        assert_eq!(
            store.allocate_voter_pseudonyms(&event, &ids).await?,
            ids.len()
        );
        if first % 100_000 == 0 {
            println!("allocated {} of {voters}", (first + 5_000).min(voters));
        }
    }
    println!(
        "registry voters={voters} allocation_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    let client = store.client().await?;
    let identities: i64 = client.query_one(
        "SELECT count(*) FROM sequent_backend.voter_pseudonym WHERE election_event_id=$1::text::uuid", &[&event],
    ).await?.get(0);
    assert_eq!(identities, voters as i64);
    let started = Instant::now();
    client
        .execute(
            "CREATE TEMP TABLE privacy_scale_fixture AS
         SELECT row_number() OVER (ORDER BY voter_id)::bigint AS n, pseudonym
         FROM sequent_backend.voter_pseudonym WHERE election_event_id=$1::text::uuid
         ORDER BY voter_id LIMIT $2",
            &[&event, &sample_ballots],
        )
        .await?;
    client
        .execute(
            "ALTER TABLE privacy_scale_fixture ADD COLUMN ballot_id text",
            &[],
        )
        .await?;
    client.execute(
        "UPDATE privacy_scale_fixture SET ballot_id=upper(substr(lpad(to_hex(n),8,'0'),1,4)||'-'||substr(lpad(to_hex(n),8,'0'),5,4))", &[],
    ).await?;
    // The shared namespace may already contain a generated receipt code: omit
    // that synthetic row, exactly as production would reserve a different code.
    client
        .execute(
            "WITH reserved AS (
             INSERT INTO sequent_backend.ballot_code (election_event_id,code,kind)
             SELECT $1::text::uuid,ballot_id,'ballot' FROM privacy_scale_fixture
             ON CONFLICT DO NOTHING RETURNING code
         ) DELETE FROM privacy_scale_fixture WHERE ballot_id NOT IN (SELECT code FROM reserved)",
            &[&event],
        )
        .await?;
    client.execute(
        "INSERT INTO ballot_box_received
         (election_event_id,ballot_id,election_id,area_id,voter_id,ballot_hash,content,
          voter_signing_pk,voter_ballot_signature,received_at,key_id,received_signature,status,cast_at)
         SELECT $1::text::uuid,ballot_id,md5('scale-election-'||((n-1)%$2)::text)::uuid,
             md5('scale-area-'||(((n-1)/$2)%$2)::text)::uuid,pseudonym,'scale-hash-'||n,
             'synthetic-encrypted-content','synthetic-key','synthetic-signature',now(),
             'synthetic-key-id','synthetic-receipt','cast',now() FROM privacy_scale_fixture", &[&event, &elections],
    ).await?;
    let inserted = client.execute(
        "INSERT INTO ballot_box_ballot
         (election_event_id,ballot_id,election_id,area_id,voter_id,format,content,pseudonym_hash,ballot_hash,voting_channel,status)
         SELECT $1::text::uuid,'scale-hash-'||n,md5('scale-election-'||((n-1)%$2)::text)::uuid,
             md5('scale-area-'||(((n-1)/$2)%$2)::text)::uuid,pseudonym,'hashable-ballot','synthetic-encrypted-content',
             decode(repeat('00',64),'hex'),decode(repeat('00',64),'hex'),'ONLINE','valid'
         FROM privacy_scale_fixture", &[&event, &elections],
    ).await?;
    client.batch_execute("ANALYZE ballot_box_ballot; ANALYZE ballot_box_received; ANALYZE sequent_backend.voter_pseudonym").await?;
    println!(
        "synthetic_ballots={inserted} election_scopes={elections} areas={elections} fixture_seconds={:.3}",
        started.elapsed().as_secs_f64()
    );
    let sample = client
        .query_one(
            "SELECT election_id::text,area_id::text,ballot_id,voter_id FROM ballot_box_received
         WHERE election_event_id=$1::text::uuid LIMIT 1",
            &[&event],
        )
        .await?;
    let election: String = sample.get(0);
    let area: String = sample.get(1);
    let ballot_id: String = sample.get(2);
    let pseudonym: String = sample.get(3);
    let allowed: Vec<String> = client.query(
        "SELECT DISTINCT election_id::text FROM ballot_box_received WHERE election_event_id=$1::text::uuid",
        &[&event],
    ).await?.iter().map(|row| row.get(0)).collect();
    let areas: i64 = client.query_one(
        "SELECT count(DISTINCT area_id) FROM ballot_box_received WHERE election_event_id=$1::text::uuid",
        &[&event],
    ).await?.get(0);
    if sample_ballots >= elections * elections {
        assert_eq!(areas, elections);
        assert_eq!(allowed.len(), elections as usize);
    }
    let mut later = None;
    let mut newest_ids = Vec::new();
    let pseudonym_start = &pseudonym[..4];
    let tracking_start = scale_tracking_id(&format!("scale-hash-{}", 1), "").unwrap();
    for search in [
        None,
        Some(PrivacyBallotSearch {
            pseudonym: Some(pseudonym_start),
            tracking_id: None,
        }),
        Some(PrivacyBallotSearch {
            pseudonym: None,
            tracking_id: Some(tracking_start.as_str()),
        }),
    ] {
        let started = Instant::now();
        let page = store
            .privacy_ballot_page(&PrivacyBallotQuery {
                event: &event,
                area: &area,
                election: None,
                elections: Some(&allowed),
                search: search.clone(),
                before: None,
                limit: 50,
                tracking_id: scale_tracking_id,
            })
            .await?;
        assert!(!page.ballots.is_empty() || search.is_some());
        assert!(page
            .ballots
            .iter()
            .all(|row| allowed.contains(&row.election_id)));
        if search.is_none() {
            later = page.next_cursor;
            newest_ids = page
                .ballots
                .iter()
                .map(|row| row.tracking_id.clone())
                .collect();
        }
        println!(
            "scoped_page search={search:?} rows={} milliseconds={:.3}",
            page.ballots.len(),
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
    let started = Instant::now();
    assert!(store
        .privacy_ballot_details(&event, &area, Some(&election), &ballot_id)
        .await?
        .is_some());
    assert!(store
        .voter_pseudonym(&event, "scale-voter-00000000")
        .await?
        .is_some());
    println!(
        "exact_ballot_and_identity_lookup_ms={:.3}",
        started.elapsed().as_secs_f64() * 1000.0
    );
    let later_page = store
        .privacy_ballot_page(&PrivacyBallotQuery {
            event: &event,
            area: &area,
            election: None,
            elections: Some(&allowed),
            search: None,
            before: later.clone(),
            limit: 50,
            tracking_id: scale_tracking_id,
        })
        .await?;
    if inserted >= 50_000 {
        assert!(!later_page.ballots.is_empty());
        assert!(later_page
            .ballots
            .iter()
            .all(|row| !newest_ids.contains(&row.tracking_id)));
    }
    // These predicates/projections mirror the production page/search/detail
    // queries. Prepared parameter types and authorized election subsets match
    // the real APIs, so plans include their optional-scope clauses.
    let last_step = "COALESCE(r.cast_at, r.audited_at, r.received_at)";
    let from = "ballot_box_received r LEFT JOIN ballot_box_ballot b
        ON b.election_event_id=r.election_event_id AND b.ballot_id=r.ballot_hash
        AND b.election_id=r.election_id AND b.voter_id=r.voter_id AND b.status='valid'";
    let scope = "r.election_event_id=$1::text::uuid AND r.area_id=$2::text::uuid
        AND ($3::text IS NULL OR r.election_id=$3::text::uuid)
        AND ($4::text[] IS NULL OR r.election_id::text=ANY($4))
        AND (r.status <> 'cast' OR b.seq IS NOT NULL)";
    let columns = format!("r.id::text,r.election_id::text,r.voter_id,r.ballot_hash,r.status,{last_step},b.voting_channel");
    let election_scope: Option<&str> = None;
    let allowed_scope = Some(allowed.as_slice());
    let limit = 51i64;
    let large = inserted >= 50_000;
    let page_sql = format!(
        "SELECT {columns} FROM {from} WHERE {scope}
         AND ({last_step}, r.id) < ($5::timestamptz, $6::text::uuid) ORDER BY {last_step} DESC, r.id DESC LIMIT $7"
    );
    let newest = (
        chrono::DateTime::<chrono::Utc>::from_timestamp(253_402_300_799, 0).unwrap(),
        "ffffffff-ffff-ffff-ffff-ffffffffffff".to_string(),
    );
    let later_at = later
        .map(|cursor| (cursor.at, cursor.id))
        .unwrap_or_else(|| newest.clone());
    for (label, (at, id)) in [("newest", newest), ("later", later_at)] {
        explain_indexed(
            &client,
            label,
            &page_sql,
            &[
                &event,
                &area,
                &election_scope,
                &allowed_scope,
                &at,
                &id,
                &limit,
            ],
            large,
        )
        .await?;
    }
    let pattern = format!("{}%", &pseudonym[..4]);
    let search_sql = format!(
        "SELECT {columns} FROM {from} WHERE {scope} AND r.voter_id LIKE $5
         ORDER BY {last_step} DESC, r.id DESC LIMIT $6"
    );
    explain_indexed(
        &client,
        "prefix",
        &search_sql,
        &[
            &event,
            &area,
            &election_scope,
            &allowed_scope,
            &pattern,
            &limit,
        ],
        large,
    )
    .await?;
    explain_indexed(
        &client,
        "details",
        "SELECT * FROM ballot_box_received
        WHERE election_event_id=$1::text::uuid AND area_id=$2::text::uuid
        AND ($3::text IS NULL OR election_id=$3::text::uuid) AND ballot_id=$4 AND status='cast'",
        &[&event, &area, &Some(election.as_str()), &ballot_id],
        large,
    )
    .await?;
    explain_indexed(
        &client,
        "identity",
        "SELECT pseudonym FROM sequent_backend.voter_pseudonym
        WHERE election_event_id=$1::text::uuid AND voter_id='scale-voter-00000000'",
        &[&event],
        voters >= 100_000,
    )
    .await?;
    drop(client);
    store.drop_ballot_box(&event).await?;
    assert_eq!(store.purge_voter_pseudonyms(&event).await?, voters as u64);
    store
        .client()
        .await?
        .execute(
            "DELETE FROM sequent_backend.ballot_code WHERE election_event_id=$1::text::uuid",
            &[&event],
        )
        .await?;
    Ok(())
}

/// Tracking IDs are computed in the crate that depends on this one.
fn scale_tracking_id(ballot_hash: &str, _voter_signing_pk: &str) -> Option<String> {
    let number: u64 = ballot_hash.strip_prefix("scale-hash-")?.parse().ok()?;
    Some(format!("{:04X}-{:04X}", number >> 16, number & 0xffff))
}

async fn explain_indexed(
    client: &tokio_postgres::Client,
    label: &str,
    sql: &str,
    parameters: &[&(dyn ToSql + Sync)],
    large: bool,
) -> Result<()> {
    let rows = client
        .query(
            &format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {sql}"),
            parameters,
        )
        .await?;
    for row in rows {
        let line: String = row.get(0);
        println!("{label}_plan: {line}");
        if large {
            assert!(
                !line.contains("Seq Scan on ballot_box_")
                    && !line.contains("Seq Scan on voter_pseudonym"),
                "{label} unexpectedly scans a large storage table: {line}"
            );
        }
    }
    Ok(())
}

// The event dashboard has no election filter by default. Its preview/count
// subqueries must use an event/address prefix instead of reading every voter.
#[tokio::test]
#[ignore = "explicit isolated PostgreSQL address-index regression"]
async fn forensic_address_lookup_is_indexed_at_large_address_cardinality() -> Result<()> {
    let store = PostgresStore::new(std::env::var("ELECTORAL_LOG_TEST_DATABASE_URL")?.parse()?)?;
    store.initialize().await?;
    let event = Uuid::new_v4().to_string();
    let election = Uuid::new_v4().to_string();
    let area = Uuid::new_v4().to_string();
    let mut client = store.client().await?;
    let tx = client.transaction().await?;
    tx.execute(
        "INSERT INTO sequent_backend.forensic_network_pseudonym
         (election_event_id,election_id,area_id,ip,pseudonym,records)
         SELECT $1::text::uuid,$2::text::uuid,$3::text::uuid,
                host('10.0.0.0'::inet+n), '7QKM-2HXD',1
         FROM generate_series(1,100000) n",
        &[&event, &election, &area],
    )
    .await?;
    tx.batch_execute("ANALYZE sequent_backend.forensic_network_pseudonym")
        .await?;
    let allowed: Option<Vec<String>> = None;
    for sql in [
        "SELECT DISTINCT p.pseudonym FROM sequent_backend.forensic_network_pseudonym p
         WHERE p.election_event_id=$1::text::uuid AND p.ip=$2
         AND ($3::text[] IS NULL OR p.election_id::text=ANY($3))
         ORDER BY p.pseudonym LIMIT 6",
        "SELECT count(DISTINCT p.pseudonym) FROM sequent_backend.forensic_network_pseudonym p
         WHERE p.election_event_id=$1::text::uuid AND p.ip=$2
         AND ($3::text[] IS NULL OR p.election_id::text=ANY($3))",
    ] {
        let rows = tx
            .query(
                &format!("EXPLAIN (ANALYZE, BUFFERS) {sql}"),
                &[&event, &"10.0.0.1", &allowed],
            )
            .await?;
        let plan = rows
            .iter()
            .map(|r| r.get::<_, String>(0))
            .collect::<Vec<_>>()
            .join("\n");
        println!("{plan}");
        assert!(
            !plan.contains("Seq Scan"),
            "address lookup scanned unrelated voters: {plan}"
        );
        assert!(
            plan.contains("forensic_pseudonym_address"),
            "address index was not used: {plan}"
        );
    }
    tx.rollback().await?;
    Ok(())
}

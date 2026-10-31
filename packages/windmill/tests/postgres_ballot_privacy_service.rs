// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! Real receive/cast service cost and ownership, with synthetic published ballots.
#![recursion_limit = "256"]
#[path = "support/schema.rs"]
mod schema;
#[path = "support/statement_proxy.rs"]
mod statement_proxy;

use anyhow::Result;
use chrono::Utc;
use deadpool_postgres::Pool;
use electoral_log::adapters::{postgres::PostgresStore, privacy::BallotIdentity};
use electoral_log::ports::ElectoralLogStore;
use sequent_core::ballot::{
    BallotStyle, HashableBallot, SignedHashableBallot, VotingStatusChannel,
};
use sequent_core::ballot_receipt::{
    sign_cast_statement, verify_cast_receipt, verify_received_ballot,
};
use sequent_core::fixtures::ballot_codec::{
    get_test_contest, get_test_decoded_vote_contest, get_writein_ballot_style,
};
use serde_json::json;
use std::time::Instant;
use uuid::Uuid;
use voting_crypto::signing::SigningKey;
use windmill::services::{
    ballot_box_key, cast_ballot, database::get_hasura_pool, electoral_log_board, receive_ballot,
};

struct Fixture {
    tenant: String,
    event: String,
    election: String,
    area: String,
    style: BallotStyle,
    store: PostgresStore,
}
impl Fixture {
    async fn create(pool: &Pool, mode: usize, voters: &[String]) -> Result<Self> {
        let tenant = Uuid::new_v4().to_string();
        let event = Uuid::new_v4().to_string();
        let election = Uuid::new_v4().to_string();
        let area = Uuid::new_v4().to_string();
        let board =
            electoral_log_board::get_event_board(&tenant, &event, &std::env::var("ENV_SLUG")?);
        let store = electoral_log_board::get_event_databases()
            .await?
            .create_event(&tenant, &event)
            .await?;
        store.create_board(&board).await?;
        store.create_ballot_box(&event).await?;
        if mode != 0 {
            store
                .set_ballot_identity(&event, BallotIdentity::Pseudonym)
                .await?;
            store.allocate_voter_pseudonyms(&event, voters).await?;
        }
        let mut db = pool.get().await?;
        let tx = db.transaction().await?;
        windmill::postgres::trusted_write(&tx).await?;
        tx.execute(
            "INSERT INTO sequent_backend.tenant(id,slug) VALUES($1::text::uuid,$1)",
            &[&tenant],
        )
        .await?;
        let presentation = json!({"ballot_identity":if mode == 0 {"voter-id"} else {"pseudonym"}, "network_data_policy":if mode == 2 {"forensic-store"} else {"do-not-store"}, "voter_signing_policy":"with-signature", "receipts":{"policy":"signed-by-ballot-box"}});
        tx.execute("INSERT INTO sequent_backend.election_event(id,tenant_id,encryption_protocol,presentation,bulletin_board_reference,voting_channels) VALUES($1::text::uuid,$2::text::uuid,'RSA256',$3,$4,$5)", &[&event,&tenant,&presentation,&json!({"id":0,"database_name":board,"is_archived":false}),&json!({"online":true,"telephone":false})]).await?;
        tx.execute("INSERT INTO sequent_backend.election(id,tenant_id,election_event_id,status,voting_channels,num_allowed_revotes) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4,$5,0)", &[&election,&tenant,&event,&json!({"voting_status":"OPEN"}),&json!({"online":true,"telephone":false})]).await?;
        tx.execute("INSERT INTO sequent_backend.area(id,tenant_id,election_event_id,name) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,'Synthetic service fixture')", &[&area,&tenant,&event]).await?;
        electoral_log_board::create_protocol_manager_keys(&tx, &tenant, &event, &board).await?;
        let event_row =
            windmill::postgres::election_event::get_election_event_by_id(&tx, &tenant, &event)
                .await?;
        let key = ballot_box_key::ballot_box_key_for_publication(&tx, &event_row)
            .await?
            .unwrap();
        let style = BallotStyle {
            id: Uuid::new_v4().to_string(),
            tenant_id: tenant.clone(),
            election_event_id: event.clone(),
            election_id: election.clone(),
            area_id: area.clone(),
            contests: vec![get_test_contest()],
            ballot_box_key: Some(key),
            ..get_writein_ballot_style()
        };
        let publication = Uuid::new_v4();
        tx.execute("INSERT INTO sequent_backend.ballot_publication(id,tenant_id,election_event_id,election_ids,is_generated,published_at) VALUES($1,$2::text::uuid,$3::text::uuid,$4,true,now())", &[&publication,&tenant,&event,&vec![election.parse::<Uuid>()?]]).await?;
        tx.execute("INSERT INTO sequent_backend.ballot_style(id,tenant_id,election_event_id,election_id,area_id,ballot_publication_id,ballot_eml) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4::text::uuid,$5::text::uuid,$6,$7)", &[&style.id,&tenant,&event,&election,&area,&publication,&serde_json::to_string(&style)?]).await?;
        tx.commit().await?;
        Ok(Self {
            tenant,
            event,
            election,
            area,
            style,
            store,
        })
    }
    fn ballot(&self) -> Result<(receive_ballot::ReceiveBallotInput, SigningKey)> {
        let voter_key = SigningKey::generate()?;
        let encrypted = sequent_core::encrypt::encrypt_decoded_contest(
            &cryptography::context::RistrettoCtx,
            &vec![get_test_decoded_vote_contest()],
            &self.style,
        )?;
        let mut signed = SignedHashableBallot::try_from(&encrypted)?;
        let hashable = HashableBallot::try_from(&signed)?;
        let hash = sequent_core::encrypt::hash_ballot(&hashable)?;
        let signature = sequent_core::ballot::sign_hashable_ballot_with_voter_signing_key(
            &voter_key,
            &hash,
            &self.election,
            &hashable,
        )
        .map_err(anyhow::Error::msg)?;
        signed.voter_signing_pk = Some(signature.public_key);
        signed.voter_ballot_signature = Some(signature.signature);
        Ok((
            receive_ballot::ReceiveBallotInput {
                election_id: self.election.parse()?,
                ballot_id: hash,
                content: serde_json::to_string(&signed)?,
            },
            voter_key,
        ))
    }
    async fn pair(
        &self,
        voter: &str,
        input: receive_ballot::ReceiveBallotInput,
        key: SigningKey,
        statements: Option<&statement_proxy::Statements>,
    ) -> Result<(f64, f64, Vec<String>)> {
        let auth = Some(Utc::now().timestamp());
        let start = Instant::now();
        let received = receive_ballot::try_receive_ballot_with_network(
            input,
            &self.tenant,
            voter,
            &self.area,
            VotingStatusChannel::ONLINE,
            &auth,
            Some(("198.51.100.8", Some("ZZ"), Some("synthetic-device"))),
        )
        .await
        .map_err(|e| anyhow::anyhow!("receive: {e:?}"))?
        .0;
        let receive_ms = start.elapsed().as_secs_f64() * 1000.0;
        verify_received_ballot(self.style.ballot_box_key.as_ref().unwrap(), &received)?;
        let cast = cast_ballot::CastBallotInput {
            election_id: self.election.parse()?,
            ballot_id: received.ballot_id.clone(),
            cast_signature: sign_cast_statement(&key, &self.election, &received.ballot_id)?,
        };
        if let Some(counter) = statements {
            counter.start();
        }
        let start = Instant::now();
        let casted = cast_ballot::try_cast_ballot_with_network(
            cast,
            &self.tenant,
            voter,
            &self.area,
            VotingStatusChannel::ONLINE,
            &auth,
            &Some("198.51.100.8".into()),
            &Some("ZZ".into()),
            &Some("synthetic-voter".into()),
            &Some("synthetic-device".into()),
        )
        .await;
        let cast_ms = start.elapsed().as_secs_f64() * 1000.0;
        let queries = statements
            .map(|counter| counter.finish())
            .unwrap_or_default();
        let cast_ballot::CastBallotResult::Cast(casted, _) =
            casted.map_err(|e| anyhow::anyhow!("cast: {e:?}"))?
        else {
            anyhow::bail!("expected a stored cast")
        };
        verify_cast_receipt(self.style.ballot_box_key.as_ref().unwrap(), &casted.receipt)?;
        Ok((receive_ms, cast_ms, queries))
    }
}

#[tokio::test]
#[ignore = "isolated PostgreSQL service fixture; run alone with --nocapture"]
async fn real_services_match_cast_query_counts_and_report_receive_cast_cost() -> Result<()> {
    let pool = schema::pool().await;
    let database: String = pool
        .get()
        .await?
        .query_one("SELECT current_database()", &[])
        .await?
        .get(0);
    let host = std::env::var("HASURA_DB__HOST")?;
    let port = std::env::var("HASURA_DB__PORT")?.parse()?;
    let (proxy_port, statements, proxy) = statement_proxy::start(host, port).await;
    // This binary owns its process-wide pools; no other test changes its environment.
    std::env::set_var("HASURA_DB__HOST", "127.0.0.1");
    std::env::set_var("HASURA_DB__PORT", proxy_port.to_string());
    std::env::set_var("HASURA_DB__DBNAME", database);
    std::env::set_var("ELECTORAL_LOG_PG_HOST", "127.0.0.1");
    std::env::set_var("ELECTORAL_LOG_PG_PORT", proxy_port.to_string());
    let services_pool = get_hasura_pool().await;
    const SAMPLES: usize = 8;
    const BATCH: usize = 4;
    let voters: Vec<String> = (0..(SAMPLES * BATCH + 4))
        .map(|_| Uuid::new_v4().to_string())
        .collect();
    let mut fixtures = Vec::new();
    for mode in 0..3 {
        fixtures.push(Fixture::create(&services_pool, mode, &voters).await?);
    }
    // Prepare valid encrypted inputs before any timed or statement-counted service call.
    let mut inputs = Vec::new();
    for fixture in &fixtures {
        let mut ballots = Vec::new();
        for _ in 0..(SAMPLES * BATCH * 2 + 4) {
            ballots.push(fixture.ballot()?);
        }
        inputs.push(ballots.into_iter());
    }
    for mode in 0..3 {
        let (input, key) = inputs[mode].next().unwrap();
        fixtures[mode].pair(&voters[0], input, key, None).await?;
    }
    let mut counts = Vec::new();
    for mode in 0..3 {
        let (input, key) = inputs[mode].next().unwrap();
        let (_, _, queries) = fixtures[mode]
            .pair(&voters[1], input, key, Some(&statements))
            .await?;
        assert!(
            queries.iter().all(|sql| !sql
                .trim_start()
                .starts_with("SELECT pseudonym FROM sequent_backend.voter_pseudonym")),
            "cast made a standalone mapping query: {queries:?}"
        );
        eprintln!(
            "service cast mode={mode} statements={} SQL={queries:?}",
            queries.len()
        );
        counts.push(queries.len());
    }
    assert_eq!(
        counts,
        vec![counts[0]; 3],
        "privacy must not add cast statements"
    );
    for revote in [false, true] {
        let mut samples = [Vec::new(), Vec::new(), Vec::new()];
        for sample in 0..SAMPLES {
            for shift in 0..3 {
                let mode = (sample + shift) % 3;
                let mut receive = 0.0;
                let mut cast = 0.0;
                for offset in 0..BATCH {
                    let voter = &voters[2 + sample * BATCH + offset];
                    let (input, key) = inputs[mode].next().unwrap();
                    let (r, c, _) = fixtures[mode].pair(voter, input, key, None).await?;
                    receive += r;
                    cast += c;
                }
                samples[mode].push((receive / BATCH as f64, cast / BATCH as f64));
            }
        }
        for (mode, samples) in samples.iter().enumerate() {
            let quantiles = |column: usize| {
                let mut values: Vec<f64> = samples
                    .iter()
                    .map(|pair| if column == 0 { pair.0 } else { pair.1 })
                    .collect();
                values.sort_by(f64::total_cmp);
                (
                    values[values.len() / 2],
                    values[(values.len() * 95).div_ceil(100) - 1],
                )
            };
            eprintln!("service mode={mode} revote={revote} batch_mean_ms samples={samples:?} receive_p50_p95={:?} cast_p50_p95={:?}; includes test wire proxy; excludes setup/encryption/HTTP/sequencer",quantiles(0),quantiles(1));
        }
    }
    for fixture in fixtures {
        let stored = fixture
            .store
            .client()
            .await?
            .query_one(
                "SELECT count(*) FROM ballot_box_ballot WHERE election_event_id=$1::text::uuid",
                &[&fixture.event],
            )
            .await?
            .get::<_, i64>(0);
        assert_eq!(stored, (SAMPLES * BATCH * 2 + 2) as i64);
        electoral_log_board::get_event_databases()
            .await?
            .drop_event(&fixture.tenant, &fixture.event)
            .await?;
        let mut db = services_pool.get().await?;
        let tx = db.transaction().await?;
        windmill::postgres::election_event::delete_election_event(
            &tx,
            &fixture.tenant,
            &fixture.event,
        )
        .await?;
        tx.execute(
            "DELETE FROM sequent_backend.tenant WHERE id=$1::text::uuid",
            &[&fixture.tenant],
        )
        .await?;
        tx.commit().await?;
    }
    proxy.abort();
    Ok(())
}

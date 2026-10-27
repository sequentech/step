// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
use crate::services::{
    database::get_hasura_pool,
    election_event_board::get_election_event_board,
    electoral_log::ElectoralLog,
    electoral_log_board::{get_board_client, get_board_signing_key, get_event_store},
};
use anyhow::{ensure, Context, Result};
use electoral_log::messages::{
    message::Message, newtypes::EventIdString, statement::StatementBody,
};
use electoral_log::LogEntry;
use futures::TryStreamExt;
use voting_crypto::{signing::VerificationKey, RecordDeserialize};

/// Write signed records through the real board and verify their exact durable readback.
pub async fn audit_record(
    tenant: &str,
    event: &str,
    run_id: &str,
    stage: &str,
    details: &serde_json::Value,
) -> Result<()> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    let e = crate::postgres::election_event::get_election_event_by_id(&tx, tenant, event).await?;
    let board =
        get_election_event_board(e.bulletin_board_reference).context("Missing event log")?;
    let log = ElectoralLog::new(&tx, tenant, Some(event), &board).await?;
    let message = Message::self_test_event(
        EventIdString(event.into()),
        stage,
        super::snapshot::canonical_details(details),
        &log.sd,
    )?;
    let wire: electoral_log::ElectoralLogMessage = (&message).try_into()?;
    let delivery = format!("self-test:{run_id}:{stage}");
    tx.commit().await?;
    drop(c);
    get_board_client()
        .await?
        .append(
            &board,
            &[LogEntry {
                delivery_id: delivery.clone(),
                message: wire.clone(),
            }],
        )
        .await?;
    let store = get_event_store(event).await?;
    let c = store.client().await?;
    let read: Vec<u8> = c
        .query_one(
            "SELECT message FROM electoral_log_messages WHERE board_name=$1 AND delivery_id=$2",
            &[&board, &delivery],
        )
        .await?
        .try_get(0)?;
    ensure!(
        read == wire.message,
        "Audit-generation readback differs from the signed record"
    );
    Ok(())
}

pub async fn ballot_boxes(tenant: &str, event: &str) -> Result<Option<String>> {
    let mut c = get_hasura_pool().await.get().await?;
    let tx = c.transaction().await?;
    let e = crate::postgres::election_event::get_election_event_by_id(&tx, tenant, event).await?;
    let board = get_election_event_board(e.bulletin_board_reference.clone())
        .context("Missing event log")?;
    let sk = get_board_signing_key(&tx, tenant, Some(event), &board).await?;
    let pk = VerificationKey::from_sk(&sk)?;
    let store = get_event_store(event).await?;
    let mut ec = store.client().await?;
    let et = ec
        .build_transaction()
        .read_only(true)
        .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
        .start()
        .await?;
    et.batch_execute("SET LOCAL statement_timeout='10000ms'")
        .await?;
    // Every sequenced cast-vote record must still have its ballot, and vice versa.
    let missing:i64=et.query_one("SELECT count(*) FROM electoral_log_messages m LEFT JOIN ballot_box_ballot b ON m.delivery_id='ballot-box:'||b.election_event_id::text||':'||b.seq::text WHERE m.board_name=$1 AND m.delivery_id LIKE $2 AND b.seq IS NULL",&[&board,&format!("ballot-box:{event}:%")]).await?.try_get(0)?;
    if missing > 0 {
        return Ok(Some("Cast-vote records have missing ballots".into()));
    }
    let inconsistent:i64=et.query_one("SELECT count(*) FROM ballot_box_voter v FULL JOIN (SELECT election_event_id,election_id,voter_id,count(*) AS votes FROM ballot_box_ballot WHERE election_event_id=$1::text::uuid GROUP BY election_event_id,election_id,voter_id) b USING(election_event_id,election_id,voter_id) WHERE coalesce(v.election_event_id,b.election_event_id)=$1::text::uuid AND (v.votes IS NULL OR b.votes IS NULL OR v.votes<>b.votes)",&[&event]).await?.try_get(0)?;
    if inconsistent > 0 {
        return Ok(Some("Ballot counts differ from the voting records".into()));
    }
    {
        let stream=et.query_raw("SELECT b.seq,b.ballot_id,b.content,b.format,b.ballot_hash,b.pseudonym_hash,b.election_id::text,b.area_id::text,b.voter_id,b.voting_channel,m.message,p.seq IS NOT NULL AS pending,b.voter_signature,b.admission_record,b.status,b.voter_ip,b.voter_country,b.username,floor(extract(epoch FROM b.accepted_at))::bigint,(extract(epoch FROM b.accepted_at)*1000)::bigint FROM ballot_box_ballot b LEFT JOIN electoral_log_messages m ON m.board_name=$2 AND m.delivery_id='ballot-box:'||b.election_event_id::text||':'||b.seq::text LEFT JOIN ballot_box_pending p USING(election_event_id,seq) WHERE b.election_event_id=$1::text::uuid ORDER BY b.seq",[&event as &(dyn tokio_postgres::types::ToSql+Sync),&board]).await?;
        futures::pin_mut!(stream);
        while let Some(row) = stream.try_next().await? {
            let id: String = row.try_get(1)?;
            let content: String = row.try_get(2)?;
            let format: String = row.try_get(3)?;
            let computed =
                crate::services::ballot_box_seal::seal::computed_ballot_id(&content, &format);
            if computed.as_deref().ok() != Some(id.as_str()) {
                return Ok(Some(
                    "A stored ballot does not match its cryptographic identifier".into(),
                ));
            }
            let admission = electoral_log::adapters::ballot_box::PendingBallot {
                seq: row.try_get(0)?,
                ballot_id: id.clone(),
                election_id: row.try_get(6)?,
                area_id: row.try_get(7)?,
                voter_id: row.try_get(8)?,
                pseudonym_hash: row.try_get(5)?,
                ballot_hash: row.try_get(4)?,
                voting_channel: row.try_get(9)?,
                status: row.try_get(14)?,
                voter_ip: row.try_get(15)?,
                voter_country: row.try_get(16)?,
                username: row.try_get(17)?,
                accepted_at: row.try_get(18)?,
                accepted_at_millis: row.try_get(19)?,
                format: format.clone(),
                content: content.clone(),
                voter_signature: row.try_get(12)?,
                admission_record: row.try_get(13)?,
            };
            if crate::services::ballot_admission::verify_pending(&admission, event, &pk).is_err() {
                return Ok(Some(
                    "A ballot differs from its authenticated admission record".into(),
                ));
            }
            let bytes: Option<Vec<u8>> = row.try_get(10)?;
            if let Some(bytes) = bytes {
                let message = match Message::record_deserialize(&bytes) {
                    Ok(m) => m,
                    Err(_) => return Ok(Some("A ballot audit record cannot be decoded".into())),
                };
                if message.verify(&pk).is_err() {
                    return Ok(Some("A ballot audit signature is invalid".into()));
                }
                let (election, pseudonym, hash, channel) = match &message.statement.body {
                    StatementBody::CastVoteWithChannel(e, p, h, _, _, c) => (e, p, h, Some(&c.0)),
                    StatementBody::CastVote(e, p, h, _, _) => (e, p, h, None),
                    _ => return Ok(Some("The ballot has an unexpected audit record".into())),
                };
                let ballot_hash: Vec<u8> = row.try_get(4)?;
                let pseudonym_hash: Vec<u8> = row.try_get(5)?;
                let election_id: String = row.try_get(6)?;
                let area: String = row.try_get(7)?;
                let voter: String = row.try_get(8)?;
                let stored_channel: String = row.try_get(9)?;
                if hash.0.to_inner().as_slice() != ballot_hash
                    || pseudonym.0.to_inner().as_slice() != pseudonym_hash
                    || election.0.as_deref() != Some(election_id.as_str())
                    || message.area_id.as_deref() != Some(area.as_str())
                    || message.user_id.as_deref() != Some(voter.as_str())
                    || channel.is_some_and(|v| v != &stored_channel)
                {
                    return Ok(Some(
                        "A ballot's identity or storage attributes differ from its signed record"
                            .into(),
                    ));
                }
            } else if !row.try_get::<_, bool>(11)? {
                return Ok(Some(
                    "An inserted ballot has neither a signed record nor a pending sequencer entry"
                        .into(),
                ));
            }
        }
    }
    et.commit().await?;
    let log =
        crate::services::ceremonies::sealed_box_ballots::SealedBoxLog::new(&tx, tenant, &e).await?;
    for seal in store.ballot_box_seals(event).await? {
        let ids = crate::services::ceremonies::sealed_box_ballots::SealedBox {
            tenant_id: tenant,
            election_event_id: event,
            election_id: &seal.election_id,
            area_id: &seal.area_id,
            election_name: "",
            area_name: "",
            tally_session_id: "self-test",
        };
        if let Some(reason) =
            crate::services::ceremonies::sealed_box_ballots::self_test_verify_box(&tx, &log, &ids)
                .await?
        {
            return Ok(Some(reason));
        }
    }
    super::protocol::verify(&tx, tenant, event).await
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Verify completed final boxes against the separately stored manager-signed input.
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use protocol_board::{
    verification::{verify_transcript, PublicTranscript, VerificationRequest, VerificationStatus},
    BoardName, SignedConfiguration,
};
use std::time::Duration;
async fn messages(name: &str) -> Result<Vec<Vec<u8>>> {
    let board = BoardName::parse(name)?;
    let endpoint = reqwest::Url::parse(&std::env::var("AWS_S3_PRIVATE_URI")?)?;
    ensure!(
        matches!(endpoint.scheme(), "http" | "https")
            && endpoint.username().is_empty()
            && endpoint.password().is_none()
            && endpoint.query().is_none()
            && endpoint.fragment().is_none(),
        "Invalid protocol storage endpoint"
    );
    let bucket = std::env::var("S3_BUCKET_NAME").unwrap_or_else(|_| "wbraid-messages".into());
    ensure!(
        bucket
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'.')),
        "Invalid protocol bucket"
    );
    let prefix = format!(
        "{}/{}/{}/messages/",
        endpoint.as_str().trim_end_matches('/'),
        bucket,
        board.as_str()
    );
    crate::services::protocol_board::board_handle(&board)?
        .export_messages_bounded(
            Duration::from_secs(10),
            8 * 1024 * 1024,
            64 * 1024 * 1024,
            10_000,
            prefix,
        )
        .await
}
pub async fn verify(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<Option<String>> {
    let rows=tx.query("SELECT b.id::text,b.name,b.manager_message,p.name,p.manager_message FROM sequent_backend.protocol_board b JOIN sequent_backend.protocol_board p ON p.id=b.parent_id AND p.tenant_id=b.tenant_id AND p.election_event_id=b.election_event_id JOIN sequent_backend.tally_session t ON t.id=b.tally_session_id AND t.tenant_id=b.tenant_id AND t.election_event_id=b.election_event_id WHERE b.tenant_id=$1::text::uuid AND b.election_event_id=$2::text::uuid AND t.execution_status='SUCCESS' AND t.is_execution_completed IS TRUE ORDER BY b.id",&[&tenant,&event]).await?;
    for row in rows {
        let id: String = row.try_get(0)?;
        let name: String = row.try_get(1)?;
        let manager: Vec<u8> = row.try_get(2)?;
        let parent: String = row.try_get(3)?;
        let config: Vec<u8> = row.try_get(4)?;
        let configuration = match SignedConfiguration::parse(&config) {
            Ok(v) => v,
            Err(_) => {
                return Ok(Some(
                    "The final-box configuration signature is invalid".into(),
                ))
            }
        };
        let dkg = messages(&parent).await?;
        let tally = messages(&name).await?;
        if !dkg.contains(&config) || !tally.contains(&manager) {
            return Ok(Some(
                "The final-box board is missing its independently stored signed input".into(),
            ));
        }
        for message in dkg.iter().chain(tally.iter()) {
            if configuration.verify_message(message).is_err() {
                return Ok(Some(
                    "A final-box protocol signature or domain is invalid".into(),
                ));
            }
        }
        let report = verify_transcript(
            &PublicTranscript::new(dkg, Some(tally)),
            &VerificationRequest {
                configuration_hash: configuration.hash().clone(),
                tally_id: Some(uuid::Uuid::parse_str(&id)?.as_u128().to_string()),
                ballots_hash: None,
                ballot_ciphertexts: vec![],
            },
        );
        if report.status != VerificationStatus::Verified {
            return Ok(Some(
                "A completed final-box transcript has missing or invalid cryptographic evidence"
                    .into(),
            ));
        }
    }
    Ok(None)
}

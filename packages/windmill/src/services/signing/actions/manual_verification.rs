// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Two separate authorities: the quorum authorizes a private document for one
//! initiation; only the voter redeeming its Keycloak action token changes identity.
use super::{gate, is_required, refuse, subject_of};
use crate::postgres::signing::{
    get_signing_post, get_signing_request, lock_signing_event, SigningRequestRow,
};
use crate::postgres::signing_actions::posts_of_area;
use crate::services::database::get_keycloak_pool;
use crate::services::signing::guard::{
    GuardOutcome, GuardRequest, RequestScope, SigningRequestSummary,
};
use crate::services::signing::operational_policy::lock_signing_policy;
use crate::services::signing::{SigningCaller, SigningError, SigningResult};
use anyhow::{anyhow, ensure, Context, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::keycloak::get_event_realm;
use sequent_core::signing::{SigningAction, SigningRequestStatus};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

const ACTION: SigningAction = SigningAction::AuthorizeManualVerification;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub initiation_id: Uuid,
    pub voter_id: String,
    pub requester_id: String,
    pub post_id: Uuid,
    pub area_id: Uuid,
    // Freeze the names, not values. A template edited after signing must not
    // silently disclose different protected attributes.
    pub secret_attributes: Vec<String>,
}

pub enum Delivery {
    Failed,
    Waiting(SigningRequestSummary),
    Document { id: Uuid, dispatch: bool },
}

async fn lock(tx: &Transaction<'_>, tenant: Uuid, event: Uuid) -> Result<()> {
    lock_signing_policy(tx).await?;
    lock_signing_event(tx, tenant, event).await
}

async fn voter_scope(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    voter: &str,
) -> SigningResult<(Uuid, Uuid)> {
    let mut client = get_keycloak_pool()
        .await
        .get()
        .await
        .context("Voter directory unavailable")?;
    let kc = client.transaction().await?;
    let realm = get_event_realm(&tenant.to_string(), &event.to_string());
    let rows = kc.query(
        "SELECT ua.value FROM user_entity u JOIN realm r ON r.id=u.realm_id JOIN user_attribute ua ON ua.user_id=u.id
         WHERE r.name=$1 AND u.id=$2 AND ua.name=$3",
        &[&realm,&voter,&sequent_core::types::keycloak::AREA_ID_ATTR_NAME],
    ).await?;
    let [assigned] = rows.as_slice() else {
        return Err(SigningError::bad_input(
            "This voter must have exactly one assigned area in this election event.",
        ));
    };
    let area: String = assigned.try_get(0)?;
    let area = Uuid::parse_str(&area)
        .map_err(|_| SigningError::bad_input("The voter’s area is not valid."))?;
    let posts = posts_of_area(tx, tenant, event, area, None).await?;
    let [(post, _)] = posts.as_slice() else {
        return Err(SigningError::invalid(
            crate::services::signing::InvalidReason::AmbiguousPost,
            "The voter’s area must resolve to exactly one Post.",
        ));
    };
    Ok((area, *post))
}

/// A retried initiation is never transferred to another voter, account or
/// event, even if that account can otherwise perform the same action.
pub async fn initiate(
    tx: &Transaction<'_>,
    caller: &SigningCaller,
    tenant: Uuid,
    event: Uuid,
    voter: &str,
    initiation: Uuid,
    names: &[String],
    recover_request: Option<Uuid>,
) -> SigningResult<Delivery> {
    lock(tx, tenant, event).await?;
    if let Some(expected) = recover_request {
        let row = tx.query_opt("SELECT 1 FROM sequent_backend.manual_verification_issuance i
            JOIN sequent_backend.signing_request r ON (r.tenant_id,r.election_event_id,r.id)=(i.tenant_id,i.election_event_id,i.request_id)
            WHERE i.initiation_id=$1 AND i.tenant_id=$2 AND i.election_event_id=$3 AND i.requester_id=$4 AND i.voter_id=$5
              AND i.request_id=$6 AND i.authorized_at IS NOT NULL AND r.status='executed' AND r.action='authorize-manual-verification'
            FOR KEY SHARE OF i,r",
            &[&initiation,&tenant,&event,&caller.user_id,&voter,&expected]).await?;
        if row.is_none() {
            return Err(SigningError::NotFound(
                "No completed authorization matches this initiation.".into(),
            ));
        }
    }
    // A lost worker cannot strand an unminted claim or mint a second token.
    // If it committed the document but died before recording readiness, recover
    // that same document. Never repeat a mint that may have already happened.
    tx.execute("UPDATE sequent_backend.manual_verification_issuance i SET ready_at=clock_timestamp()
        WHERE initiation_id=$1 AND tenant_id=$2 AND election_event_id=$3 AND requester_id=$4 AND voter_id=$5
          AND minted_at IS NOT NULL AND ready_at IS NULL AND failed_at IS NULL
          AND EXISTS(SELECT FROM sequent_backend.document d WHERE d.id=i.document_id AND d.tenant_id=i.tenant_id
                     AND d.election_event_id=i.election_event_id AND d.is_public IS FALSE
                     AND (i.request_id IS NULL OR d.annotations->>'purpose'='signed-manual-verification'))",
         &[&initiation,&tenant,&event,&caller.user_id,&voter]).await?;
    tx.execute("UPDATE sequent_backend.manual_verification_issuance SET
          render_started_at=CASE WHEN minted_at IS NULL THEN NULL ELSE render_started_at END,
          proof_sha256=CASE WHEN minted_at IS NULL THEN NULL ELSE proof_sha256 END,
          failed_at=CASE WHEN minted_at IS NOT NULL THEN clock_timestamp() END
        WHERE initiation_id=$1 AND tenant_id=$2 AND election_event_id=$3 AND requester_id=$4 AND voter_id=$5
          AND render_started_at < clock_timestamp()-interval '30 minutes' AND ready_at IS NULL AND failed_at IS NULL",
         &[&initiation,&tenant,&event,&caller.user_id,&voter]).await?;
    let saved = tx
        .query_opt(
            "SELECT tenant_id, election_event_id, requester_id, voter_id, request_id, document_id,
                secret_attributes, failed_at IS NOT NULL AS failed, ready_at IS NOT NULL AS ready,
                render_started_at IS NOT NULL AS started, post_id
         FROM sequent_backend.manual_verification_issuance WHERE initiation_id=$1 FOR UPDATE",
            &[&initiation],
        )
        .await
        .map_err(anyhow::Error::from)?;
    if let Some(row) = saved {
        let matches = row.get::<_, Uuid>("tenant_id") == tenant
            && row.get::<_, Uuid>("election_event_id") == event
            && row.get::<_, String>("requester_id") == caller.user_id
            && row.get::<_, String>("voter_id") == voter;
        if !matches {
            return Err(SigningError::Forbidden(
                "This is not your initiation.".into(),
            ));
        }
        if row.get::<_, bool>("failed") {
            return Ok(Delivery::Failed);
        }
        if row.get::<_, Vec<String>>("secret_attributes") != names {
            return Err(SigningError::Conflict(
                "Issuance failed or the report template changed; start a new authorization.".into(),
            ));
        }
        if let Some(post) = row.get::<_, Option<Uuid>>("post_id") {
            let p = get_signing_post(tx, tenant, event, post)
                .await?
                .context("Post does not exist")?;
            if !caller.reaches(p.permission_label.as_deref()) {
                return Err(SigningError::Forbidden(
                    "This Post is not accessible.".into(),
                ));
            }
        }
        if let Some(id) = row.get::<_, Option<Uuid>>("request_id") {
            let req = get_signing_request(tx, tenant, event, id)
                .await?
                .context("Signing request is missing")?;
            match req.status {
                SigningRequestStatus::Waiting | SigningRequestStatus::Completed => {
                    return Ok(Delivery::Waiting((&req).into()))
                }
                SigningRequestStatus::Executed => verify_subject(tx, &req, initiation).await?,
                other => {
                    return Err(SigningError::Closed {
                        status: other,
                        message: "This authorization is closed.".into(),
                    })
                }
            }
        } else if is_required(tx, tenant, event, ACTION).await? {
            return Err(SigningError::Conflict(
                "Signatures are now required; start a new authorization.".into(),
            ));
        }
        if let Some(id) = row.get::<_, Option<Uuid>>("document_id") {
            return Ok(Delivery::Document {
                id,
                dispatch: !row.get::<_, bool>("ready") && !row.get::<_, bool>("started"),
            });
        }
    } else {
        // gate() checks effective policy, the user's permission labels and the
        // exact server-built Post before entering the standard quorum ledger.
        let outcome = gate(tx, caller, ACTION, tenant, event, || async {
            let (area_id, post_id) = voter_scope(tx, tenant, event, voter).await?;
            let subject = Subject {
                initiation_id: initiation,
                voter_id: voter.to_owned(),
                requester_id: caller.user_id.clone(),
                post_id,
                area_id,
                secret_attributes: names.to_vec(),
            };
            Ok(GuardRequest {
                action: ACTION,
                scope: RequestScope {
                    tenant_id: tenant,
                    election_event_id: event,
                    election_id: Some(post_id),
                    area_id: None,
                    trustee_id: None,
                    subject_key: Some(initiation.to_string()),
                },
                subject: serde_json::to_value(subject).map_err(anyhow::Error::from)?,
                document: None,
                config_revision: None,
            })
        })
        .await?;
        let request = match &outcome {
            GuardOutcome::SigningRequired(s) => Some(s.id),
            GuardOutcome::Proceed => None,
        };
        let post = if let Some(id) = request {
            get_signing_request(tx, tenant, event, id)
                .await?
                .context("Signing request missing")?
                .election_id
        } else {
            None
        };
        tx.execute("INSERT INTO sequent_backend.manual_verification_issuance
                (initiation_id, tenant_id, election_event_id, requester_id, voter_id, post_id, request_id, authorized_at, secret_attributes)
             VALUES ($1,$2,$3,$4,$5,$6,$7,CASE WHEN $7::uuid IS NULL THEN clock_timestamp() END,$8)",&[&initiation,&tenant,&event,&caller.user_id,&voter,&post,&request,&names]).await.map_err(anyhow::Error::from)?;
        if let GuardOutcome::SigningRequired(s) = outcome {
            return Ok(Delivery::Waiting(s));
        }
    }
    let id = Uuid::new_v4();
    let count = tx
        .execute(
            "UPDATE sequent_backend.manual_verification_issuance SET document_id=$2
          WHERE initiation_id=$1 AND authorized_at IS NOT NULL AND document_id IS NULL",
            &[&initiation, &id],
        )
        .await
        .map_err(anyhow::Error::from)?;
    if count != 1 {
        return Err(SigningError::Conflict(
            "Authorization has not settled.".into(),
        ));
    }
    Ok(Delivery::Document { id, dispatch: true })
}

async fn verify_subject(
    tx: &Transaction<'_>,
    req: &SigningRequestRow,
    initiation: Uuid,
) -> Result<()> {
    ensure!(
        super::super::operational_device::matches_canonical_request(req),
        "Manual verification signing payload changed"
    );
    let s: Subject = subject_of(req)?;
    let expected_scope = RequestScope {
        tenant_id: req.tenant_id,
        election_event_id: req.election_event_id,
        election_id: Some(s.post_id),
        area_id: None,
        trustee_id: None,
        subject_key: Some(initiation.to_string()),
    };
    ensure!(
        req.scope_key == expected_scope.scope_key()
            && req.document_id.is_none()
            && req.document_sha256.is_none(),
        "Manual verification scope changed"
    );
    let binding=tx.query_opt("SELECT 1 FROM sequent_backend.manual_verification_issuance
        WHERE initiation_id=$1 AND tenant_id=$2 AND election_event_id=$3 AND requester_id=$4 AND voter_id=$5
          AND post_id=$6 AND request_id=$7 AND secret_attributes=$8",
        &[&initiation,&req.tenant_id,&req.election_event_id,&s.requester_id,&s.voter_id,&s.post_id,&req.id,&s.secret_attributes]).await?;
    ensure!(binding.is_some(), "Manual verification initiation changed");
    let (area, post) = voter_scope(tx, req.tenant_id, req.election_event_id, &s.voter_id).await?;
    ensure!(
        req.action == ACTION
            && s.initiation_id == initiation
            && req.requested_by == s.requester_id
            && req.election_id == Some(post)
            && s.post_id == post
            && s.area_id == area
            && req.area_id.is_none()
            && req.trustee_id.is_none(),
        "Manual verification scope changed"
    );
    Ok(())
}

/// In the existing quorum executor transaction; no callout, credential,
/// enrollment decision or PDF is an effect of signing.
pub async fn settle(tx: &Transaction<'_>, req: &SigningRequestRow) -> Result<Value> {
    let s: Subject = subject_of(req)?;
    verify_subject(tx, req, s.initiation_id)
        .await
        .map_err(|_| {
            refuse(
                "manual-verification-target-changed",
                "The manual verification target changed.",
            )
        })?;
    let changed=tx.execute("UPDATE sequent_backend.manual_verification_issuance SET authorized_at=COALESCE(authorized_at,clock_timestamp())
          WHERE initiation_id=$1 AND tenant_id=$2 AND election_event_id=$3
            AND voter_id=$4 AND requester_id=$5 AND post_id=$6 AND request_id=$7
            AND secret_attributes=$8 AND document_id IS NULL",&[&s.initiation_id,&req.tenant_id,&req.election_event_id,&s.voter_id,&req.requested_by,&s.post_id,&req.id,&s.secret_attributes]).await?;
    ensure!(
        changed == 1,
        "Manual verification initiation no longer matches"
    );
    Ok(json!({"manual_verification_authorized":true}))
}

// Secret proof for one rendering attempt. Do not derive Debug or Serialize.
pub struct RenderPermit {
    pub initiation: Uuid,
    pub proof: Uuid,
    pub signed: bool,
}

pub async fn claim_render(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    voter: &str,
    doc: Uuid,
    secret_names: &[String],
) -> Result<Option<RenderPermit>> {
    lock(tx, tenant, event).await?;
    let proof = Uuid::new_v4();
    let hash = Sha256::digest(proof.as_bytes()).to_vec();
    let row = tx
        .query_opt(
            "UPDATE sequent_backend.manual_verification_issuance i
        SET render_started_at=clock_timestamp(), proof_sha256=$6
        WHERE tenant_id=$1 AND election_event_id=$2 AND voter_id=$3 AND document_id=$4
          AND secret_attributes=$5 AND authorized_at IS NOT NULL AND render_started_at IS NULL
          AND failed_at IS NULL AND (request_id IS NULL OR EXISTS (
            SELECT 1 FROM sequent_backend.signing_request r WHERE r.id=i.request_id
              AND r.action='authorize-manual-verification' AND r.status='executed'))
        RETURNING initiation_id, request_id",
            &[&tenant, &event, &voter, &doc, &secret_names, &hash],
        )
        .await?;
    let Some(row) = row else { return Ok(None) };
    let init = row.get(0);
    let req = row.get(1);
    check_requirement(tx, tenant, event, init, req).await?;
    Ok(Some(RenderPermit {
        initiation: init,
        proof,
        signed: req.is_some(),
    }))
}

async fn check_requirement(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    init: Uuid,
    req: Option<Uuid>,
) -> Result<()> {
    if let Some(req) = req {
        let r = get_signing_request(tx, tenant, event, req)
            .await?
            .context("Signing request unavailable")?;
        ensure!(
            r.status == SigningRequestStatus::Executed,
            "Authorization not settled"
        );
        verify_subject(tx, &r, init).await?;
    } else {
        ensure!(
            !is_required(tx, tenant, event, ACTION).await?,
            "Manual verification requires authorization"
        );
    }
    Ok(())
}

pub async fn finish_render(
    tx: &Transaction<'_>,
    permit: &RenderPermit,
    success: bool,
) -> Result<()> {
    let sql = if success {
        "UPDATE sequent_backend.manual_verification_issuance SET ready_at=COALESCE(ready_at,clock_timestamp())
          WHERE initiation_id=$1 AND proof_sha256=$2 AND minted_at IS NOT NULL AND failed_at IS NULL"
    } else {
        "UPDATE sequent_backend.manual_verification_issuance SET failed_at=clock_timestamp()
          WHERE initiation_id=$1 AND proof_sha256=$2 AND ready_at IS NULL"
    };
    ensure!(
        tx.execute(
            sql,
            &[
                &permit.initiation,
                &Sha256::digest(permit.proof.as_bytes()).to_vec()
            ]
        )
        .await?
            == 1,
        "Could not settle verification document"
    );
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Issue,
    Redeem,
}

/// Called by the Keycloak realm's service account before signing a bearer
/// link and before mutating the account. Legacy unbound links are accepted
/// only while this event affirmatively needs no manual-verification quorum.
pub async fn authorize_token(
    tx: &Transaction<'_>,
    tenant: Uuid,
    event: Uuid,
    voter: &str,
    phase: Phase,
    init: Option<Uuid>,
    proof: Option<Uuid>,
) -> Result<()> {
    lock(tx, tenant, event).await?;
    ensure!(
        tx.query_opt(
            "SELECT 1 FROM sequent_backend.election_event WHERE tenant_id=$1 AND id=$2",
            &[&tenant, &event]
        )
        .await?
        .is_some(),
        "Unknown event"
    );
    let (init, proof) = match (init, proof) {
        (Some(i), Some(p)) => (i, p),
        (None, None) => {
            ensure!(
                !is_required(tx, tenant, event, ACTION).await?,
                "Signed authorization required"
            );
            return Ok(());
        }
        _ => return Err(anyhow!("Incomplete authorization")),
    };
    let digest = Sha256::digest(proof.as_bytes()).to_vec();
    let row=tx.query_opt("SELECT request_id FROM sequent_backend.manual_verification_issuance WHERE initiation_id=$1 AND tenant_id=$2 AND election_event_id=$3 AND voter_id=$4
          AND proof_sha256=$5 AND failed_at IS NULL AND authorized_at IS NOT NULL FOR UPDATE",&[&init,&tenant,&event,&voter,&digest]).await?.context("Issuance not authorized")?;
    check_requirement(tx, tenant, event, init, row.get(0)).await?;
    let transition=match phase {
        Phase::Issue=>"UPDATE sequent_backend.manual_verification_issuance SET minted_at=clock_timestamp() WHERE initiation_id=$1 AND minted_at IS NULL
              AND render_started_at > clock_timestamp()-interval '10 minutes'",
        Phase::Redeem=>"UPDATE sequent_backend.manual_verification_issuance SET redeemed_at=clock_timestamp() WHERE initiation_id=$1
              AND ready_at IS NOT NULL AND redeemed_at IS NULL",
    };
    ensure!(
        tx.execute(transition, &[&init]).await? == 1,
        "Issuance is not in the required state"
    );
    Ok(())
}

/// A signed manual-verification PDF is a private handoff for the initiating
/// operator, not an ordinary report another document reader can retrieve.
pub async fn may_download(tx: &Transaction<'_>, doc: Uuid, caller: &SigningCaller) -> Result<bool> {
    let row=tx.query_opt("SELECT tenant_id, election_event_id, requester_id, post_id,
                 ready_at IS NOT NULL AS ready, initiation_id, request_id
          FROM sequent_backend.manual_verification_issuance WHERE document_id=$1 AND request_id IS NOT NULL",&[&doc]).await?;
    let Some(row) = row else { return Ok(false) };
    let post = get_signing_post(
        tx,
        row.get("tenant_id"),
        row.get("election_event_id"),
        row.get("post_id"),
    )
    .await?
    .context("Post unavailable")?;
    if row.get::<_, String>("requester_id") != caller.user_id || !row.get::<_, bool>("ready") {
        return Ok(false);
    }
    let req = get_signing_request(
        tx,
        row.get("tenant_id"),
        row.get("election_event_id"),
        row.get("request_id"),
    )
    .await?
    .context("Authorization unavailable")?;
    if req.status != SigningRequestStatus::Executed {
        return Ok(false);
    }
    verify_subject(tx, &req, row.get("initiation_id")).await?;
    Ok(row.get::<_, String>("requester_id") == caller.user_id
        && row.get::<_, bool>("ready")
        && caller.roles.contains(
            &sequent_core::types::permissions::Permissions::VOTER_MANUALLY_VERIFY.to_string(),
        )
        && caller.reaches(post.permission_label.as_deref()))
}

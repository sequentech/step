// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Durable pre-render outbox. Jobs only publish while their lease/generation
//! still matches; readers reject stale backgrounds immediately after an edit.
use crate::services::database::get_hasura_pool;
use anyhow::{anyhow, bail, Result};
use deadpool_postgres::Transaction;
use sequent_core::services::{pdf, reports};
use sequent_core::types::templates::{PrintToPdfOptionsLocal, SendTemplateBody};
use sequent_report_prerender::{fingerprint, prepare, validate_background};
use serde_json::{json, Value};
use std::io::BufWriter;
use uuid::Uuid;

pub use sequent_report_prerender::CachedPdf as CachedLayout;

/// Fetch under the caller's tenant/event boundary. Holding a shared row lock
/// makes a concurrent template update wait until this artifact is committed.
pub async fn get_cached(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    alias: &str,
    election: Option<&str>,
) -> Result<CachedLayout> {
    let tenant = Uuid::parse_str(tenant)?;
    let event = Uuid::parse_str(event)?;
    let election = election.map(Uuid::parse_str).transpose()?;
    let row=tx.query_opt(r#"SELECT cache.background,cache.manifest,cache.state,
        cache.generation=cache.ready_generation AS fresh
        FROM sequent_backend.report_prerender cache JOIN sequent_backend.report r ON r.id=cache.report_id
        WHERE cache.tenant_id=$1 AND cache.election_event_id=$2 AND r.template_alias=$3
        AND (r.election_id IS NOT DISTINCT FROM $4::uuid OR r.election_id IS NULL)
        ORDER BY (r.election_id IS NOT DISTINCT FROM $4::uuid) DESC,r.id LIMIT 1 FOR SHARE OF cache"#,&[&tenant,&event,&alias,&election]).await?
        .ok_or_else(||anyhow!("Pre-rendered report is not ready; wait for the background layout job"))?;
    if row.get::<_, String>("state") != "ready"
        || !row.get::<_, Option<bool>>("fresh").unwrap_or(false)
    {
        bail!("Pre-rendered report is pending or failed; retry after the background layout job completes");
    }
    Ok(CachedLayout {
        background: row.get("background"),
        manifest: serde_json::from_value(row.get("manifest"))?,
    })
}

pub fn fill_to_temp(cached: CachedLayout, data: &Value) -> Result<tempfile::NamedTempFile> {
    let mut file = tempfile::Builder::new()
        .prefix("sequent-prerender-")
        .suffix(".pdf")
        .tempfile()?;
    cached.fill(data, BufWriter::new(file.as_file_mut()))?;
    Ok(file)
}

async fn known_data(
    tx: &Transaction<'_>,
    tenant: &Uuid,
    event: &Uuid,
    election: Option<Uuid>,
) -> Result<Value> {
    // Reuse the same import projection as Studio. No voter/result data enters
    // the immutable background or its cache key.
    let event_row: Value = tx
        .query_one(
            "SELECT to_jsonb(e) FROM sequent_backend.election_event e WHERE tenant_id=$1 AND id=$2",
            &[tenant, event],
        )
        .await?
        .get(0);
    let elections:Value=tx.query_one("SELECT COALESCE(jsonb_agg(to_jsonb(e) ORDER BY e.id),'[]') FROM sequent_backend.election e WHERE tenant_id=$1 AND election_event_id=$2 AND ($3::uuid IS NULL OR id=$3)",&[tenant,event,&election]).await?.get(0);
    let contests:Value=tx.query_one("SELECT COALESCE(jsonb_agg(to_jsonb(c) ORDER BY CASE WHEN c.presentation->>'sort_order' ~ '^-?[0-9]+$' THEN (c.presentation->>'sort_order')::numeric END NULLS LAST,c.id),'[]') FROM sequent_backend.contest c WHERE tenant_id=$1 AND election_event_id=$2 AND ($3::uuid IS NULL OR election_id=$3)",&[tenant,event,&election]).await?.get(0);
    let candidates:Value=tx.query_one("SELECT COALESCE(jsonb_agg(to_jsonb(c) ORDER BY CASE WHEN c.presentation->>'sort_order' ~ '^-?[0-9]+$' THEN (c.presentation->>'sort_order')::numeric END NULLS LAST,c.id),'[]') FROM sequent_backend.candidate c WHERE tenant_id=$1 AND election_event_id=$2 AND contest_id IN (SELECT id FROM sequent_backend.contest WHERE tenant_id=$1 AND election_event_id=$2 AND ($3::uuid IS NULL OR election_id=$3))",&[tenant,event,&election]).await?.get(0);
    let exported = json!({"election_event":event_row,"elections":elections,"contests":contests,"candidates":candidates});
    let sample = reports::sample_data::from_export(&exported).map_err(|e| anyhow!(e))?;
    Ok(sample["knownData"].clone())
}

/// One bounded job per invocation. Database leases recover crashed workers;
/// failed generations back off and remain visibly failed after five attempts.
pub async fn warm_next() -> Result<bool> {
    let mut client = get_hasura_pool().await.get().await?;
    let tx = client.transaction().await?;
    let row=tx.query_opt(r#"SELECT cache.report_id,cache.tenant_id,cache.election_event_id,
        cache.generation,cache.fingerprint,cache.background IS NOT NULL AS has_background,
        r.election_id,t.template
        FROM sequent_backend.report_prerender cache JOIN sequent_backend.report r ON r.id=cache.report_id
        JOIN sequent_backend.template t ON t.tenant_id=r.tenant_id AND t.alias=r.template_alias
        WHERE t.template #>> '{pre_render,enabled}'='true' AND cache.attempts<5
        AND cache.retry_after<=now() AND (cache.state IN ('pending','failed') OR (cache.state='building' AND cache.lease_until<now()))
        ORDER BY cache.updated_at LIMIT 1 FOR UPDATE OF cache SKIP LOCKED"#,&[]).await?;
    let Some(row) = row else {
        return Ok(false);
    };
    let report: Uuid = row.get("report_id");
    let tenant: Uuid = row.get("tenant_id");
    let event: Uuid = row.get("election_event_id");
    let generation: i64 = row.get("generation");
    let election: Option<Uuid> = row.get("election_id");
    let template: Value = row.get("template");
    let old_key: Option<String> = row.get("fingerprint");
    let has_background: bool = row.get("has_background");
    let token = Uuid::new_v4();
    tx.execute("UPDATE sequent_backend.report_prerender SET state='building',lease_token=$2,lease_until=now()+interval '5 minutes',attempts=attempts+1 WHERE report_id=$1",&[&report,&token]).await?;
    // Capture all inputs under this transaction; an edit increments generation
    // after commit, preventing this job from publishing its obsolete result.
    let known = known_data(&tx, &tenant, &event, election).await;
    tx.commit().await?;
    let result=async {
        let known=known?;
        let config:SendTemplateBody=serde_json::from_value(template.clone())?;
        if config.pre_render.as_ref().map(|c|c.version)!=Some(1) { bail!("Unsupported pre-render version"); }
        if let Some(options)=&config.pdf_options {
            if options.landscape==Some(true) || options.scale.is_some_and(|v|v!=1.0) || [options.margin_top,options.margin_bottom,options.margin_left,options.margin_right].iter().flatten().any(|v|*v!=0.0) {
                bail!("Pre-render layouts require scale 1, zero margins and explicit portrait page dimensions");
            }
        }
        let effective=json!({"document":config.document,"assets":config.assets,"pdf_options":template["pdf_options"],"version":1});
        let key=fingerprint(&effective,&known);
        if has_background && old_key.as_deref()==Some(&key) {
            let cached=client.query_one("SELECT background,manifest FROM sequent_backend.report_prerender WHERE report_id=$1 AND generation=$2 AND lease_token=$3",&[&report,&generation,&token]).await?;
            let background:Vec<u8>=cached.get("background");
            let manifest=serde_json::from_value(cached.get("manifest"))?;
            validate_background(&background,&manifest)?;
            return Ok::<_,anyhow::Error>((key,None));
        }
        let mut manifest=prepare(config.document.as_deref().unwrap_or(""),&known).map_err(|e|anyhow!(e))?;
        let html=reports::assets::attach(&manifest.html,&config.assets).map_err(|e|anyhow!(e))?;
        let options:Option<PrintToPdfOptionsLocal>=template.get("pdf_options").filter(|v|!v.is_null()).map(|v|serde_json::from_value(v.clone())).transpose()?;
        let background=pdf::PdfRenderer::render_pdf(html,options.map(|o|o.to_print_to_pdf_options())).await?;
        validate_background(&background,&manifest)?;
        // Keep only the field manifest in PostgreSQL; the HTML is reconstructible.
        manifest.html.clear();
        Ok((key,Some((background,serde_json::to_value(manifest)?))))
    }.await;
    match result {
        Ok((key, Some((background, manifest)))) => {
            client.execute("UPDATE sequent_backend.report_prerender SET fingerprint=$4,background=$5,manifest=$6,ready_generation=generation,state='ready',error=NULL,lease_until=NULL,lease_token=NULL,updated_at=now() WHERE report_id=$1 AND generation=$2 AND lease_token=$3",&[&report,&generation,&token,&key,&background,&manifest]).await?;
        }
        Ok((_, None)) => {
            client.execute("UPDATE sequent_backend.report_prerender SET ready_generation=generation,state='ready',error=NULL,lease_until=NULL,lease_token=NULL,updated_at=now() WHERE report_id=$1 AND generation=$2 AND lease_token=$3",&[&report,&generation,&token]).await?;
        }
        Err(error) => {
            let message = format!("{error:#}").chars().take(2000).collect::<String>();
            client.execute("UPDATE sequent_backend.report_prerender SET state='failed',error=$4,retry_after=now()+interval '30 seconds'*power(2,attempts),lease_until=NULL,lease_token=NULL,updated_at=now() WHERE report_id=$1 AND generation=$2 AND lease_token=$3",&[&report,&generation,&token,&message]).await?;
            tracing::warn!(%report,"Pre-render job failed; see report_prerender.error");
        }
    }
    Ok(true)
}

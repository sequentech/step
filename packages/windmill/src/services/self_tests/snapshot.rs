// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
//! Snapshots are captured in the subject signed by the election board, never learned by a test.
use crate::services::database::get_keycloak_pool;
use crate::services::signing::actions::configuration::canonical_text;
use anyhow::{ensure, Context, Result};
use deadpool_postgres::Transaction;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub fn digest(value: &Value) -> String {
    hex::encode(Sha256::digest(canonical_text(value).as_bytes()))
}

/// Hash sorted rows without keeping voter data in the self-test result/baseline.
async fn rows_digest(
    tx: &Transaction<'_>,
    sql: &str,
    args: &[&(dyn tokio_postgres::types::ToSql + Sync)],
) -> Result<String> {
    use futures::TryStreamExt;
    let stream = tx.query_raw(sql, args.iter().copied()).await?;
    futures::pin_mut!(stream);
    let mut hash = Sha256::new();
    while let Some(row) = stream.try_next().await? {
        let value: Value = row.try_get(0)?;
        let bytes = canonical_text(&value);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes.as_bytes());
    }
    Ok(hex::encode(hash.finalize()))
}

pub async fn capture(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<Value> {
    let mut data = serde_json::Map::new();
    // Status and scheduled lockdown are protected lifecycle attributes, not
    // immutable election input. administration_settings is a generated projection
    // of the original presentation, which remains hashed here.
    // Voter/staff locale is an ordinary login preference; authorization attributes
    // and all other imported data remain covered.
    for (name,sql) in [
        ("event", "SELECT (to_jsonb(t)-ARRAY['created_at','updated_at','last_updated_at','status','is_archived','bulletin_board_reference','statistics','administration_settings']) #- '{presentation,locked_down}' FROM sequent_backend.election_event t WHERE tenant_id=$1::text::uuid AND id=$2::text::uuid ORDER BY id"),
        ("elections", "SELECT to_jsonb(t)-ARRAY['created_at','updated_at','last_updated_at','status','is_archived','statistics','initialization_report_generated','administration_settings'] FROM sequent_backend.election t WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY id"),
        ("ballot_styles", "SELECT to_jsonb(t)-ARRAY['created_at','updated_at','last_updated_at'] FROM sequent_backend.ballot_style t WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY id"),
        ("contests", "SELECT to_jsonb(t)-ARRAY['created_at','updated_at','last_updated_at'] FROM sequent_backend.contest t WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY id"),
        ("candidates", "SELECT to_jsonb(t)-ARRAY['created_at','updated_at','last_updated_at'] FROM sequent_backend.candidate t WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY id"),
        ("areas", "SELECT to_jsonb(t)-ARRAY['created_at','updated_at','last_updated_at'] FROM sequent_backend.area t WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY id"),
        ("area_contests", "SELECT to_jsonb(t)-ARRAY['created_at','updated_at','last_updated_at'] FROM sequent_backend.area_contest t WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY id"),
    ] { data.insert(name.into(),json!(rows_digest(tx,sql,&[&tenant,&event]).await?)); }
    let manifest = crate::postgres::configuration_packages::manifest_of_event(tx, tenant, event)
        .await?
        .context("The event has no authenticated imported configuration package")?;
    let stored:Value=tx.query_one("SELECT manifest FROM sequent_backend.configuration_package WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY revision DESC LIMIT 1",&[&tenant,&event]).await?.try_get(0)?;
    data.insert("dkg_configuration".into(),json!(rows_digest(tx,"SELECT jsonb_build_object('id',id,'name',name,'manager_message',encode(manager_message,'hex')) FROM sequent_backend.protocol_board WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid AND parent_id IS NULL ORDER BY id",&[&tenant,&event]).await?));
    data.insert("manifest".into(), json!(digest(&stored)));
    data.insert("imported_manifest".into(), json!(manifest.1));
    let mut kc = get_keycloak_pool().await.get().await?;
    let kt = kc
        .build_transaction()
        .isolation_level(tokio_postgres::IsolationLevel::RepeatableRead)
        .read_only(true)
        .start()
        .await?;
    kt.batch_execute("SET LOCAL statement_timeout='10000ms'")
        .await?;
    let realm = format!("tenant-{tenant}-event-{event}");
    data.insert("voters".into(),json!(rows_digest(&kt,
        "SELECT jsonb_build_object('id',u.id,'username',u.username,'first_name',u.first_name,'last_name',u.last_name,'email',u.email,'enabled',u.enabled, 'attributes',(SELECT coalesce(jsonb_agg(jsonb_build_array(a.name,coalesce(a.value,a.long_value)) ORDER BY a.name,coalesce(a.value,a.long_value)),'[]') FROM user_attribute a WHERE a.user_id=u.id AND a.name <> 'locale')) FROM user_entity u JOIN realm r ON r.id=u.realm_id WHERE r.name=$1 ORDER BY u.id",&[&realm]).await?));
    let mut access = serde_json::Map::new();
    let realms = vec![format!("tenant-{tenant}"), realm];
    // Include role assignments as well as definitions: a newly granted role is a configuration change.
    for (name,sql) in [
        ("realms","SELECT to_jsonb(r) FROM realm r WHERE r.name=ANY($1) ORDER BY r.id"),
        ("realm_attributes","SELECT to_jsonb(a) FROM realm_attribute a JOIN realm r ON r.id=a.realm_id WHERE r.name=ANY($1) ORDER BY a.name,a.value"),
        ("clients","SELECT to_jsonb(c) FROM client c JOIN realm r ON r.id=c.realm_id WHERE r.name=ANY($1) ORDER BY c.id"),
        ("client_attributes","SELECT to_jsonb(a) FROM client_attributes a JOIN client c ON c.id=a.client_id JOIN realm r ON r.id=c.realm_id WHERE r.name=ANY($1) ORDER BY a.client_id,a.name"),
        ("staff_accounts","SELECT jsonb_build_object('id',u.id,'enabled',u.enabled,'username',u.username,'service',u.service_account_client_link) FROM user_entity u JOIN realm r ON r.id=u.realm_id WHERE r.name=($1::text[])[1] ORDER BY u.id"),
        ("staff_attributes","SELECT to_jsonb(a) FROM user_attribute a JOIN user_entity u ON u.id=a.user_id JOIN realm r ON r.id=u.realm_id WHERE r.name=($1::text[])[1] AND a.name<>'locale' ORDER BY a.user_id,a.name,a.id"),
        ("group_attributes","SELECT to_jsonb(a) FROM group_attribute a JOIN keycloak_group g ON g.id=a.group_id JOIN realm r ON r.id=g.realm_id WHERE r.name=ANY($1) ORDER BY a.group_id,a.name,a.id"),
        ("role_attributes","SELECT to_jsonb(a) FROM role_attribute a JOIN keycloak_role k ON k.id=a.role_id JOIN realm r ON r.id=k.realm_id WHERE r.name=ANY($1) ORDER BY a.role_id,a.name,a.id"),
        ("authentication_flows","SELECT to_jsonb(a) FROM authentication_flow a JOIN realm r ON r.id=a.realm_id WHERE r.name=ANY($1) ORDER BY a.id"),
        ("authentication_execution","SELECT to_jsonb(a) FROM authentication_execution a JOIN realm r ON r.id=a.realm_id WHERE r.name=ANY($1) ORDER BY a.id"),
        ("protocol_mappers","SELECT to_jsonb(p) FROM protocol_mapper p LEFT JOIN client c ON c.id=p.client_id LEFT JOIN client_scope s ON s.id=p.client_scope_id JOIN realm r ON r.id=coalesce(c.realm_id,s.realm_id) WHERE r.name=ANY($1) ORDER BY p.id"),
        ("mapper_configuration","SELECT to_jsonb(a) FROM protocol_mapper_config a JOIN protocol_mapper p ON p.id=a.protocol_mapper_id LEFT JOIN client c ON c.id=p.client_id LEFT JOIN client_scope s ON s.id=p.client_scope_id JOIN realm r ON r.id=coalesce(c.realm_id,s.realm_id) WHERE r.name=ANY($1) ORDER BY a.protocol_mapper_id,a.name"),
        ("roles","SELECT to_jsonb(k) FROM keycloak_role k JOIN realm r ON r.id=k.realm_id WHERE r.name=ANY($1) ORDER BY k.id"),
        ("groups","SELECT to_jsonb(g) FROM keycloak_group g JOIN realm r ON r.id=g.realm_id WHERE r.name=ANY($1) ORDER BY g.id"),
        ("group_roles","SELECT to_jsonb(gm) FROM group_role_mapping gm JOIN keycloak_group g ON g.id=gm.group_id JOIN realm r ON r.id=g.realm_id WHERE r.name=ANY($1) ORDER BY gm.group_id,gm.role_id"),
        ("memberships","SELECT to_jsonb(m) FROM user_group_membership m JOIN user_entity u ON u.id=m.user_id JOIN realm r ON r.id=u.realm_id WHERE r.name=ANY($1) ORDER BY m.user_id,m.group_id"),
        ("user_roles","SELECT to_jsonb(m) FROM user_role_mapping m JOIN user_entity u ON u.id=m.user_id JOIN realm r ON r.id=u.realm_id WHERE r.name=ANY($1) ORDER BY m.user_id,m.role_id"),
        ("composites","SELECT to_jsonb(c) FROM composite_role c JOIN keycloak_role k ON k.id=c.composite JOIN realm r ON r.id=k.realm_id WHERE r.name=ANY($1) ORDER BY c.composite,c.child_role"),
    ] {access.insert(name.into(),json!(rows_digest(&kt,sql,&[&realms]).await?));}
    kt.commit().await?;
    access.insert(
        "hasura_metadata".into(),
        json!(
            rows_digest(
                tx,
                "SELECT metadata FROM hdb_catalog.hdb_metadata ORDER BY id",
                &[]
            )
            .await?
        ),
    );
    access.insert("signing_rules".into(),json!(rows_digest(tx,"SELECT to_jsonb(t)-ARRAY['updated_at','created_at'] FROM sequent_backend.signing_rule t WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid ORDER BY action",&[&tenant,&event]).await?));
    access.insert(
        "tenant_settings".into(),
        json!(
            rows_digest(
                tx,
                "SELECT settings FROM sequent_backend.tenant WHERE id=$1::text::uuid",
                &[&tenant]
            )
            .await?
        ),
    );
    access.insert(
        "tls_endpoints".into(),
        json!(std::env::var("WINDMILL_SELF_TEST_TLS_ENDPOINTS")?),
    );
    if let Ok(path) = std::env::var("WINDMILL_SELF_TEST_TLS_CA_FILE") {
        access.insert(
            "tls_ca".into(),
            json!(hex::encode(Sha256::digest(std::fs::read(path)?))),
        );
    }
    access.insert(
        "self_test_settings".into(),
        json!(super::settings(tx, tenant, event).await?),
    );
    let attack=tx.query_one("SELECT jsonb_build_object('failed_logins_per_minute',COALESCE(s.failed_logins_per_minute,20),'requests_per_minute',COALESCE(s.requests_per_minute,2000)) FROM sequent_backend.tenant t LEFT JOIN sequent_backend.attack_detection_settings s ON s.tenant_id=t.id WHERE t.id=$1::text::uuid", &[&tenant]).await?;
    access.insert(
        "attack_detection_settings".into(),
        attack.try_get::<_, Value>(0)?,
    );
    Ok(
        json!({"election_data":digest(&Value::Object(data)),"access_control":digest(&Value::Object(access))}),
    )
}

fn authenticated_bytes(tenant: &str, event: &str, publication: &str, snapshot: &Value) -> Vec<u8> {
    canonical_text(&json!({"domain":"sequent.self-test-baseline.v1","tenant":tenant,"event":event,"publication":publication,"snapshot":snapshot})).into_bytes()
}
async fn key(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
) -> Result<voting_crypto::signing::SigningKey> {
    let e = crate::postgres::election_event::get_election_event_by_id(tx, tenant, event).await?;
    let board =
        crate::services::election_event_board::get_election_event_board(e.bulletin_board_reference)
            .context("Missing event log")?;
    crate::services::electoral_log_board::get_board_signing_key(tx, tenant, Some(event), &board)
        .await
}
/// This signature authenticates the result of the completed board approval. A
/// test never signs, recaptures or repairs a baseline, even after a failure.
pub async fn approve(
    tx: &Transaction<'_>,
    tenant: &str,
    event: &str,
    publication: &str,
    snapshot: &Value,
) -> Result<()> {
    let signature = key(tx, tenant, event).await?.sign(&authenticated_bytes(
        tenant,
        event,
        publication,
        snapshot,
    ))?;
    tx.execute("INSERT INTO sequent_backend.self_test_baseline (tenant_id,election_event_id,publication_id,snapshot,digest,signature) VALUES($1::text::uuid,$2::text::uuid,$3::text::uuid,$4,$5,$6) ON CONFLICT(tenant_id,election_event_id) DO UPDATE SET publication_id=EXCLUDED.publication_id,snapshot=EXCLUDED.snapshot,digest=EXCLUDED.digest,signature=EXCLUDED.signature,approved_at=now()",&[&tenant,&event,&publication,&snapshot,&digest(snapshot),&serde_json::to_value(signature)?]).await?;
    Ok(())
}
pub async fn approved(tx: &Transaction<'_>, tenant: &str, event: &str) -> Result<Value> {
    let row=tx.query_opt("SELECT publication_id::text,snapshot,digest,signature FROM sequent_backend.self_test_baseline WHERE tenant_id=$1::text::uuid AND election_event_id=$2::text::uuid",&[&tenant,&event]).await?.context("No board-approved self-test baseline exists")?;
    let publication: String = row.try_get(0)?;
    let snapshot: Value = row.try_get(1)?;
    let stored_digest: String = row.try_get(2)?;
    ensure!(
        digest(&snapshot) == stored_digest,
        "Self-test baseline digest is invalid"
    );
    let signature: voting_crypto::signing::Signature = serde_json::from_value(row.try_get(3)?)?;
    voting_crypto::signing::VerificationKey::from_sk(&key(tx, tenant, event).await?)?
        .verify(
            &signature,
            &authenticated_bytes(tenant, event, &publication, &snapshot),
        )
        .context("Self-test approval evidence has been altered")?;
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn baseline_cannot_be_replaced_or_moved_to_another_event() {
        use voting_crypto::signing::{SigningKey, VerificationKey};
        let key = SigningKey::generate().unwrap();
        let public = VerificationKey::from_sk(&key).unwrap();
        let snapshot = json!({"election_data":"approved-data","access_control":"approved-access"});
        let signature = key
            .sign(&authenticated_bytes(
                "tenant",
                "event",
                "publication",
                &snapshot,
            ))
            .unwrap();
        public
            .verify(
                &signature,
                &authenticated_bytes("tenant", "event", "publication", &snapshot),
            )
            .unwrap();
        for bytes in [
            authenticated_bytes("other", "event", "publication", &snapshot),
            authenticated_bytes("tenant", "other", "publication", &snapshot),
            authenticated_bytes("tenant", "event", "other", &snapshot),
            authenticated_bytes(
                "tenant",
                "event",
                "publication",
                &json!({"election_data":"tampered","access_control":"approved-access"}),
            ),
        ] {
            assert!(public.verify(&signature, &bytes).is_err());
        }
    }
}

pub fn canonical_details(value: &Value) -> String {
    canonical_text(value)
}

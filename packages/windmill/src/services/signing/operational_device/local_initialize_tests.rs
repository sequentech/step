// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::services::signing::operational_device::{
    canonical_tests::signed_request, matches_canonical_request,
};
use serde_json::{json, Value};

fn manifest() -> Value {
    let id = Uuid::from_u128;
    json!({
        "tenant": id(1), "event": id(2), "post": id(4), "publication": id(6),
        "installation": id(9), "profile": "softhsm-test"
    })
}

fn parse(value: &Value) -> Result<LocalInitializeInstallation> {
    LocalInitializeInstallation::parse(&serde_json::to_vec(value)?)
}

#[cfg(target_os = "linux")]
fn read_at(run: &std::fs::File) -> Result<Option<LocalInitializeInstallation>> {
    use std::os::unix::fs::MetadataExt;
    descriptor::read_at(run, run.metadata()?.uid())
}

#[test]
fn descriptor_accepts_only_the_six_typed_fields_and_the_software_profile() {
    let valid = manifest();
    let device = parse(&valid).unwrap().device();
    assert!(device.admits(Some("softhsm-test")));
    assert!(!InstalledDevice::YubiHsm.admits(Some(device.as_str())));
    for wrong in [None, Some("yubihsm"), Some("legacy-p12"), Some("software")] {
        assert!(!device.admits(wrong));
    }
    for field in ["tenant", "event", "post", "publication", "installation"] {
        for invalid in [json!(Uuid::nil()), json!("invalid"), json!(12), Value::Null] {
            let mut value = valid.clone();
            value[field] = invalid;
            assert!(parse(&value).is_err(), "{field}");
        }
        let mut missing = valid.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(parse(&missing).is_err(), "{field}");
    }
    for profile in [
        json!("yubihsm"),
        json!("disabled"),
        json!("legacy-p12"),
        json!({"softhsm-test": null}),
    ] {
        let mut value = valid.clone();
        value["profile"] = profile;
        assert!(parse(&value).is_err());
    }
    let mut extra = valid.clone();
    extra["path"] = json!("/tmp/other-installation");
    assert!(parse(&extra).is_err());
    let positional = json!([
        valid["tenant"], valid["event"], valid["post"],
        valid["publication"], valid["installation"], valid["profile"]
    ]);
    assert!(parse(&positional).is_err());
    let duplicate = valid
        .to_string()
        .replacen('{', r#"{"profile":"softhsm-test","#, 1);
    assert!(LocalInitializeInstallation::parse(duplicate.as_bytes()).is_err());
    let mut bytes = serde_json::to_vec(&valid).unwrap();
    bytes.resize(MAX_BYTES, b' ');
    assert!(LocalInitializeInstallation::parse(&bytes).is_ok());
    bytes.push(b' ');
    assert!(LocalInitializeInstallation::parse(&bytes).is_err());
}

#[test]
fn local_receipts_require_the_canonical_initialize_post_and_signed_publication() {
    let row = signed_request(SigningAction::InitializeVoting);
    assert!(matches_canonical_request(&row));
    let local = parse(&manifest()).unwrap();
    assert!(local.matches_request(&row).unwrap());
    for field in ["tenant", "event", "post", "publication"] {
        let mut other = manifest();
        other[field] = json!(Uuid::from_u128(10));
        assert!(
            !parse(&other).unwrap().matches_request(&row).unwrap(),
            "{field}"
        );
    }
    let other_action = signed_request(SigningAction::OpenVoting);
    assert!(matches_canonical_request(&other_action));
    assert!(!local.matches_request(&other_action).unwrap());

    for subject in [
        json!([local.publication]),
        json!({"ballot_publication_id": local.publication}),
        json!({"publication_id": null}),
        json!({"publication_id": 6}),
        json!({"publication_id": Uuid::nil()}),
        json!({"publication_id": local.publication.simple().to_string()}),
    ] {
        let changed = SigningRequestRow {
            subject,
            ..row.clone()
        };
        assert!(local.matches_request(&changed).is_err());
        assert!(!matches_canonical_request(&changed));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn absent_descriptor_and_named_but_disabled_installation_are_distinct() -> Result<()> {
    use std::{fs, os::unix::fs::PermissionsExt};

    let root = tempfile::tempdir()?;
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))?;
    let run = fs::File::open(root.path())?;
    assert!(read_at(&run)?.is_none());
    let directory = root.path().join(DIRECTORY);
    fs::create_dir(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    assert!(read_at(&run)?.is_none());
    let path = directory.join(FILENAME);
    fs::write(&path, manifest().to_string())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444))?;
    let named = read_at(&run)?.unwrap();
    assert!(named.check_id(Some(named.installation)).is_ok());
    for disabled in [None, Some(Uuid::nil()), Some(Uuid::from_u128(10))] {
        assert!(named.check_id(disabled).is_err());
    }
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn unsafe_or_unreadable_installation_is_never_treated_as_absent() -> Result<()> {
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    let root = tempfile::tempdir()?;
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700))?;
    let run = fs::File::open(root.path())?;
    let directory = root.path().join(DIRECTORY);
    symlink("missing-directory", &directory)?;
    assert!(read_at(&run).is_err());
    fs::remove_file(&directory)?;
    fs::create_dir(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    let path = directory.join(FILENAME);
    let target = root.path().join("elsewhere");
    symlink(&target, &path)?;
    assert!(read_at(&run).is_err());
    fs::write(&target, manifest().to_string())?;
    fs::set_permissions(&target, fs::Permissions::from_mode(0o444))?;
    assert!(read_at(&run).is_err());
    fs::remove_file(&path)?;
    fs::create_dir(&path)?;
    assert!(read_at(&run).is_err());
    fs::remove_dir(&path)?;
    fs::write(&path, manifest().to_string())?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644))?;
    assert!(read_at(&run).is_err());
    fs::write(&path, b"{")?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o444))?;
    assert!(read_at(&run).is_err());
    fs::remove_file(&path)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o777))?;
    assert!(read_at(&run).is_err());
    Ok(())
}

/// Run after the normal application has executed the imported Post's Initialize
/// request. Check the committed effect and exercise the same claim a redelivered
/// Celery task takes. All database work in the replay probe is rolled back.
#[tokio::test]
#[ignore = "requires an installed Felix3 scope and a real completed Initialize Voting run"]
async fn executed_local_initialize_has_applied_receipts_and_refuses_replay() -> Result<()> {
    use crate::postgres::signing::list_signing_approvals;
    use crate::services::database::get_hasura_pool;
    use crate::services::signing::{
        guard::effective_rule,
        operational_policy::assert_startup_policy,
        operational_quorum::{checked_approval_device, has_operational_quorum},
        requests::claim_dispatched,
    };
    use sequent_core::signing::SigningRequestStatus;

    let local = installed()?.context("missing local Initialize installation")?;
    let mut client = get_hasura_pool().await.get().await?;
    assert_startup_policy(&client).await?;
    let tx = client.transaction().await?;
    let row = tx
        .query_one(
            "SELECT * FROM sequent_backend.signing_request
             WHERE tenant_id=$1 AND election_event_id=$2 AND election_id=$3
               AND action='initialize-voting' AND subject->>'publication_id'=$4::text
             ORDER BY created_at DESC, id DESC LIMIT 1",
            &[
                &local.tenant,
                &local.event,
                &local.post,
                &local.publication.to_string(),
            ],
        )
        .await
        .context("no Initialize request for the installed publication")?;
    let request = SigningRequestRow::try_from(row)?;
    ensure!(request.status == SigningRequestStatus::Executed && request.executed_at.is_some());
    ensure!(
        checked_approval_device(&tx, &request, &[]).await? == Some(InstalledDevice::DisposableSoftHsm)
    );
    let current = effective_rule(&tx, local.tenant, local.event, request.action).await?;
    let task = request
        .task_execution_id
        .context("missing dispatched task")?;
    let replay = claim_dispatched(&tx, local.tenant, local.event, request.id, task).await?;
    ensure!(!replay, "the executed task acquired another claim");
    let approvals = list_signing_approvals(&tx, local.tenant, local.event, request.id).await?;
    ensure!(
        has_operational_quorum(&tx, &request, &approvals, i64::from(current.required())).await?,
        "the executed request lacks its current applied receipt quorum"
    );
    let tally = request
        .execution_result
        .as_ref()
        .and_then(|result| result.get("tally_session_id"))
        .and_then(Value::as_str)
        .context("missing committed tally session ID")?;
    let tally_id = Uuid::parse_str(tally)?;
    let effect: bool = tx
        .query_one(
            "SELECT EXISTS (SELECT FROM sequent_backend.tally_session
             WHERE tenant_id=$1 AND election_event_id=$2 AND id=$3
               AND election_ids=ARRAY[$4::uuid] AND tally_type='INITIALIZATION_REPORT')",
            &[&local.tenant, &local.event, &tally_id, &local.post],
        )
        .await?
        .try_get(0)?;
    ensure!(
        effect,
        "the result does not identify the Post's initialization tally"
    );
    tx.rollback().await?;
    Ok(())
}

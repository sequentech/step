// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::adapters::memory::results_publication::{
    InMemoryResultsEventPresentation, InMemoryResultsPublicationAudit, InMemoryResultsPublications,
    PublicationCall,
};
use crate::adapters::memory::results_publication_lifecycle::{
    ArtifactCall, InMemoryResultsArtifacts,
};
use crate::domain::results_publication::fixtures::*;
use sequent_core::types::hasura::core::Document;

const PREFIX: &str = "tenant-tenant-1/event-event-1/results";

fn public_publication() -> TallyResultsPublication {
    TallyResultsPublication {
        access: ResultsWebsiteAccess::Public,
        manifest: Some(json!({"schema_version": 1, "title": {"en": "Results"}})),
        error_message: Some("previous finalization failed".into()),
        ..publication()
    }
}

fn presentations(access: &str) -> InMemoryResultsEventPresentation {
    InMemoryResultsEventPresentation::with(
        TENANT_ID,
        ELECTION_EVENT_ID,
        presentation(policy("enabled", access, "full_event")),
    )
}

fn artifacts() -> InMemoryResultsArtifacts {
    let artifacts = InMemoryResultsArtifacts::default();
    artifacts.set_output(
        json!({"full_sqlite": {"document_id": "rendered-sqlite"}}),
        serde_json::from_value(manifest(ResultsManifestArtifacts::default())).unwrap(),
    );
    artifacts
}

async fn publish(
    store: &InMemoryResultsPublications,
    artifacts: &InMemoryResultsArtifacts,
) -> Result<()> {
    publish_results_website_artifacts_with(
        store,
        artifacts,
        TENANT_ID,
        ELECTION_EVENT_ID,
        "publication-1",
    )
    .await
}

async fn refresh(
    store: &InMemoryResultsPublications,
    presentations: &InMemoryResultsEventPresentation,
    artifacts: &InMemoryResultsArtifacts,
) -> Result<()> {
    refresh_public_results_index_with(
        store,
        presentations,
        artifacts,
        TENANT_ID,
        ELECTION_EVENT_ID,
    )
    .await
}

async fn finalize(
    store: &InMemoryResultsPublications,
    presentations: &InMemoryResultsEventPresentation,
    artifacts: &InMemoryResultsArtifacts,
    audit: &InMemoryResultsPublicationAudit,
) -> Result<()> {
    finalize_results_website_publication_with(
        store,
        presentations,
        artifacts,
        audit,
        TENANT_ID,
        ELECTION_EVENT_ID,
        "publication-1",
        "admin-1",
        Some("Admin".into()),
    )
    .await
}

fn document(id: &str, name: Option<&str>) -> Document {
    serde_json::from_value(json!({"id": id, "name": name})).unwrap()
}

#[tokio::test]
async fn published_artifacts_are_not_rendered_again() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    let artifacts = artifacts();
    artifacts.fail_on(ArtifactCall::Prepare, "source unavailable");
    publish(&store, &artifacts).await.unwrap();
    assert!(artifacts.snapshot().prepared_contests.is_none());
    assert_eq!(
        store.stored()[0].error_message.as_deref(),
        Some("previous finalization failed")
    );
}

#[tokio::test]
async fn revoked_and_superseded_publications_cannot_be_republished() {
    for (status, message) in [
        (
            ResultsPublicationStatus::Revoked,
            "Cannot publish a Revoked results publication",
        ),
        (
            ResultsPublicationStatus::Superseded,
            "Cannot publish a Superseded results publication",
        ),
    ] {
        let store = InMemoryResultsPublications::with([TallyResultsPublication {
            publication_status: status,
            ..public_publication()
        }]);
        let artifacts = artifacts();
        assert_eq!(
            publish(&store, &artifacts).await.unwrap_err().to_string(),
            message
        );
        assert!(artifacts.snapshot().prepared_contests.is_none());
        assert_eq!(store.stored()[0].publication_status, status);
    }
}

#[tokio::test]
async fn publishing_and_failed_publications_store_the_rendered_artifacts() {
    for status in [
        ResultsPublicationStatus::Publishing,
        ResultsPublicationStatus::Failed,
    ] {
        for access in [
            ResultsWebsiteAccess::Public,
            ResultsWebsiteAccess::Authenticated,
        ] {
            let store = InMemoryResultsPublications::with([TallyResultsPublication {
                publication_status: status,
                access,
                published_contest_ids: vec!["contest-2".into(), "contest-1".into()],
                ..public_publication()
            }]);
            let artifacts = artifacts();
            publish(&store, &artifacts).await.unwrap();
            let stored = store.stored().remove(0);
            assert_eq!(
                stored.publication_status,
                ResultsPublicationStatus::Published
            );
            assert_eq!(
                stored.documents,
                json!({"full_sqlite": {"document_id": "rendered-sqlite"}})
            );
            assert_eq!(
                stored.manifest,
                Some(manifest(ResultsManifestArtifacts::default()))
            );
            assert_eq!(stored.error_message, None);
            let state = artifacts.snapshot();
            assert_eq!(
                state.prepared_contests,
                Some(vec!["contest-2".into(), "contest-1".into()])
            );
            if access == ResultsWebsiteAccess::Public {
                assert_eq!(
                    state.public_contests,
                    Some(vec!["contest-2".into(), "contest-1".into()])
                );
                assert!(state.private_contests.is_none());
            } else {
                assert_eq!(
                    state.private_contests,
                    Some(vec!["contest-2".into(), "contest-1".into()])
                );
                assert!(state.public_contests.is_none());
            }
        }
    }
}

#[tokio::test]
async fn rendering_failures_leave_the_publication_unpublished() {
    for (call, access) in [
        (ArtifactCall::Prepare, ResultsWebsiteAccess::Public),
        (ArtifactCall::PublishPublic, ResultsWebsiteAccess::Public),
        (
            ArtifactCall::PublishPrivate,
            ResultsWebsiteAccess::Authenticated,
        ),
    ] {
        let store = InMemoryResultsPublications::with([TallyResultsPublication {
            publication_status: ResultsPublicationStatus::Publishing,
            access,
            ..publication()
        }]);
        let artifacts = artifacts();
        artifacts.fail_on(call, "artifact failure");
        assert_eq!(
            publish(&store, &artifacts).await.unwrap_err().to_string(),
            "artifact failure"
        );
        assert_eq!(
            store.stored()[0].publication_status,
            ResultsPublicationStatus::Publishing
        );
        assert_eq!(store.stored()[0].documents, json!({}));
    }
}

#[tokio::test]
async fn storing_a_rendered_publication_propagates_database_errors() {
    let store = InMemoryResultsPublications::with([TallyResultsPublication {
        publication_status: ResultsPublicationStatus::Failed,
        ..publication()
    }]);
    store.fail_on(PublicationCall::MarkPublished, "write failed");
    let artifacts = artifacts();
    assert_eq!(
        publish(&store, &artifacts).await.unwrap_err().to_string(),
        "write failed"
    );
    assert_eq!(
        artifacts.snapshot().private_contests,
        Some(vec!["contest-1".into()])
    );
    assert_eq!(
        store.stored()[0].publication_status,
        ResultsPublicationStatus::Failed
    );
}

#[tokio::test]
async fn a_missing_publication_does_not_prepare_artifacts() {
    let artifacts = artifacts();
    assert_eq!(
        publish(&InMemoryResultsPublications::default(), &artifacts)
            .await
            .unwrap_err()
            .to_string(),
        "Publication not found"
    );
    assert!(artifacts.snapshot().prepared_contests.is_none());
}

#[tokio::test]
async fn public_cleanup_removes_nested_stored_and_expected_paths_once() {
    let publication = TallyResultsPublication {
        documents: json!({
            "one": {"document_id": "doc-1", "public_path": "stored.sqlite"},
            "nested": [{"document_id": "doc-1", "latest_public_path": "stored-latest.json"}, {"document_id": 12, "public_path": false}]
        }),
        ..public_publication()
    };
    let artifacts = artifacts();
    artifacts.fail_on(
        ArtifactCall::Document,
        "public cleanup must not read private documents",
    );
    delete_publication_artifacts_with(&artifacts, &publication, true)
        .await
        .unwrap();
    let state = artifacts.snapshot();
    assert_eq!(
        state.removed_public_paths,
        HashSet::from([
            "stored.sqlite".into(),
            "stored-latest.json".into(),
            format!("{PREFIX}/full-v1.sqlite"),
            format!("{PREFIX}/manifest-v1.json"),
            format!("{PREFIX}/manifest-latest.json")
        ])
    );
    assert_eq!(
        state.removed_documents,
        HashSet::from([(TENANT_ID.into(), ELECTION_EVENT_ID.into(), "doc-1".into())])
    );
    assert!(state.removed_private_paths.is_empty());
}

#[tokio::test]
async fn superseded_election_cleanup_keeps_latest_aliases() {
    let publication = TallyResultsPublication {
        route_scope: ResultsRouteScope::Election,
        route_election_id: Some("election-2".into()),
        documents: json!({"public_path": "stored-v1.json", "latest_public_path": "stored-latest.json"}),
        ..public_publication()
    };
    let artifacts = artifacts();
    delete_publication_artifacts_with(&artifacts, &publication, false)
        .await
        .unwrap();
    assert_eq!(
        artifacts.snapshot().removed_public_paths,
        HashSet::from([
            "stored-v1.json".into(),
            format!("{PREFIX}/elections/election-2/full-v1.sqlite"),
            format!("{PREFIX}/elections/election-2/manifest-v1.json")
        ])
    );
}

#[tokio::test]
async fn private_cleanup_skips_missing_documents_and_uses_an_empty_missing_name() {
    let publication = TallyResultsPublication {
        documents: json!([{"document_id": "present"}, {"document_id": "missing"}, {"document_id": "nameless"}]),
        ..publication()
    };
    let artifacts = artifacts();
    artifacts.insert_document(
        TENANT_ID,
        ELECTION_EVENT_ID,
        document("present", Some("results.sqlite")),
    );
    artifacts.insert_document(TENANT_ID, ELECTION_EVENT_ID, document("nameless", None));
    delete_publication_artifacts_with(&artifacts, &publication, true)
        .await
        .unwrap();
    let state = artifacts.snapshot();
    assert_eq!(
        state.removed_private_paths,
        HashSet::from([
            "tenant-tenant-1/event-event-1/document-present/results.sqlite".into(),
            "tenant-tenant-1/event-event-1/document-nameless/".into()
        ])
    );
    assert_eq!(state.removed_documents.len(), 3);
    assert!(state.documents.is_empty());
    assert!(state.removed_public_paths.is_empty());
}

#[tokio::test]
async fn object_storage_and_document_lookup_failures_preserve_document_rows() {
    for call in [
        ArtifactCall::Document,
        ArtifactCall::DeletePrivate,
        ArtifactCall::DeletePublic,
    ] {
        let publication = TallyResultsPublication {
            access: if call == ArtifactCall::DeletePublic {
                ResultsWebsiteAccess::Public
            } else {
                ResultsWebsiteAccess::Authenticated
            },
            documents: json!({"document_id": "doc-1"}),
            ..publication()
        };
        let artifacts = artifacts();
        artifacts.insert_document(
            TENANT_ID,
            ELECTION_EVENT_ID,
            document("doc-1", Some("results.sqlite")),
        );
        artifacts.fail_on(call, "cleanup failed");
        assert_eq!(
            delete_publication_artifacts_with(&artifacts, &publication, true)
                .await
                .unwrap_err()
                .to_string(),
            "cleanup failed"
        );
        assert!(artifacts.snapshot().removed_documents.is_empty());
        assert_eq!(artifacts.snapshot().documents.len(), 1);
    }
}

#[tokio::test]
async fn document_deletion_errors_follow_successful_object_removal() {
    let artifacts = artifacts();
    artifacts.fail_on(ArtifactCall::DeleteDocuments, "delete rows failed");
    assert_eq!(
        delete_publication_artifacts_with(&artifacts, &public_publication(), true)
            .await
            .unwrap_err()
            .to_string(),
        "delete rows failed"
    );
    assert_eq!(artifacts.snapshot().removed_public_paths.len(), 3);
}

#[tokio::test]
async fn an_enabled_public_index_contains_only_matching_event_and_election_routes() {
    let election = TallyResultsPublication {
        id: "election-publication".into(),
        route_scope: ResultsRouteScope::Election,
        route_election_id: Some("election-1".into()),
        election_ids: vec!["election-1".into()],
        ..public_publication()
    };
    let mismatch = TallyResultsPublication {
        id: "private-publication".into(),
        documents: json!({"document_id": "private-doc"}),
        ..publication()
    };
    let store = InMemoryResultsPublications::with([public_publication(), election, mismatch]);
    let artifacts = artifacts();
    refresh(&store, &presentations("public"), &artifacts)
        .await
        .unwrap();
    assert_eq!(
        artifacts.snapshot().indexes["results-index/event-1.json"],
        json!({
            "schema_version": 1, "tenant_id": "tenant-1", "election_event_id": "event-1", "publications": [
                {"publication_id": "election-publication", "route_scope": "election", "route": "/event-1/elections/election-1", "route_election_id": "election-1", "election_ids": ["election-1"], "access": "public", "visibility_scope": "full_event", "manifest_public_path": "tenant-tenant-1/event-event-1/results/elections/election-1/manifest-latest.json"},
                {"publication_id": "publication-1", "route_scope": "event", "route": "/event-1", "route_election_id": null, "election_ids": ["election-1", "election-2"], "access": "public", "visibility_scope": "full_event", "manifest_public_path": "tenant-tenant-1/event-event-1/results/manifest-latest.json"}
            ]
        })
    );
    assert_eq!(
        store.stored()[2].publication_status,
        ResultsPublicationStatus::Superseded
    );
    assert!(artifacts.snapshot().removed_documents.contains(&(
        TENANT_ID.into(),
        ELECTION_EVENT_ID.into(),
        "private-doc".into()
    )));
}

#[tokio::test]
async fn an_authenticated_index_never_exposes_a_public_manifest_path() {
    let store = InMemoryResultsPublications::with([publication()]);
    let artifacts = artifacts();
    refresh(&store, &presentations("authenticated"), &artifacts)
        .await
        .unwrap();
    let index = &artifacts.snapshot().indexes["results-index/event-1.json"];
    assert_eq!(index["publications"][0]["publication_id"], "publication-1");
    assert!(index["publications"][0]["manifest_public_path"].is_null());
}

#[tokio::test]
async fn a_disabled_website_removes_its_public_prefix_and_supersedes_active_publications() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    let artifacts = artifacts();
    let disabled =
        InMemoryResultsEventPresentation::with(TENANT_ID, ELECTION_EVENT_ID, presentation(None));
    refresh(&store, &disabled, &artifacts).await.unwrap();
    assert_eq!(
        store.stored()[0].publication_status,
        ResultsPublicationStatus::Superseded
    );
    assert!(artifacts.snapshot().removed_public_paths.contains(PREFIX));
    assert_eq!(
        artifacts.snapshot().indexes["results-index/event-1.json"]["publications"],
        json!([])
    );
}

#[tokio::test]
async fn active_publication_read_errors_precede_invalid_policy_errors() {
    let store = InMemoryResultsPublications::default();
    store.fail_on(PublicationCall::Active, "query failed");
    let invalid = InMemoryResultsEventPresentation::with(
        TENANT_ID,
        ELECTION_EVENT_ID,
        presentation(Some(json!("invalid"))),
    );
    let artifacts = artifacts();
    assert_eq!(
        refresh(&store, &invalid, &artifacts)
            .await
            .unwrap_err()
            .to_string(),
        "query failed"
    );
    assert!(artifacts.snapshot().indexes.is_empty());
}

#[tokio::test]
async fn cleanup_failure_does_not_supersede_a_publication_or_replace_the_index() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    let artifacts = artifacts();
    artifacts.fail_on(ArtifactCall::DeletePublic, "S3 unavailable");
    assert_eq!(
        refresh(&store, &presentations("authenticated"), &artifacts)
            .await
            .unwrap_err()
            .to_string(),
        "S3 unavailable"
    );
    assert_eq!(
        store.stored()[0].publication_status,
        ResultsPublicationStatus::Published
    );
    assert!(artifacts.snapshot().indexes.is_empty());
}

#[tokio::test]
async fn superseding_failure_leaves_the_index_untouched_after_cleanup() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    store.fail_on(PublicationCall::MarkSuperseded, "supersede failed");
    let artifacts = artifacts();
    assert_eq!(
        refresh(&store, &presentations("authenticated"), &artifacts)
            .await
            .unwrap_err()
            .to_string(),
        "supersede failed"
    );
    assert_eq!(artifacts.snapshot().removed_public_paths.len(), 3);
    assert!(artifacts.snapshot().indexes.is_empty());
}

#[tokio::test]
async fn only_published_publications_can_be_finalized() {
    for status in [
        ResultsPublicationStatus::Publishing,
        ResultsPublicationStatus::Failed,
        ResultsPublicationStatus::Revoked,
        ResultsPublicationStatus::Superseded,
    ] {
        let store = InMemoryResultsPublications::with([TallyResultsPublication {
            publication_status: status,
            ..public_publication()
        }]);
        let artifacts = artifacts();
        let audit = InMemoryResultsPublicationAudit::default();
        assert_eq!(
            finalize(&store, &presentations("public"), &artifacts, &audit)
                .await
                .unwrap_err()
                .to_string(),
            "Results publication is not published"
        );
        assert!(artifacts.snapshot().latest_manifests.is_empty());
        assert!(artifacts.snapshot().indexes.is_empty());
        assert!(audit.entries().is_empty());
    }
}

#[tokio::test]
async fn a_public_manifest_is_required_before_any_finalization_effects() {
    let store = InMemoryResultsPublications::with([TallyResultsPublication {
        manifest: None,
        ..public_publication()
    }]);
    let artifacts = artifacts();
    let audit = InMemoryResultsPublicationAudit::default();
    assert_eq!(
        finalize(&store, &presentations("public"), &artifacts, &audit)
            .await
            .unwrap_err()
            .to_string(),
        "Published results publication has no manifest"
    );
    assert!(artifacts.snapshot().indexes.is_empty());
    assert!(audit.entries().is_empty());
}

#[tokio::test]
async fn public_finalization_uploads_latest_cleans_old_versions_and_records_the_actor() {
    let superseded = TallyResultsPublication {
        id: "old-publication".into(),
        version: 2,
        publication_status: ResultsPublicationStatus::Superseded,
        documents: json!({"document_id": "old-document", "latest_public_path": "retained-latest.json"}),
        ..public_publication()
    };
    let store = InMemoryResultsPublications::with([public_publication(), superseded]);
    let artifacts = artifacts();
    let audit = InMemoryResultsPublicationAudit::default();
    finalize(&store, &presentations("public"), &artifacts, &audit)
        .await
        .unwrap();
    let state = artifacts.snapshot();
    assert_eq!(
        state.latest_manifests["publication-1"],
        json!({"schema_version": 1, "title": {"en": "Results"}})
    );
    assert_eq!(
        state.removed_public_paths,
        HashSet::from([
            format!("{PREFIX}/full-v2.sqlite"),
            format!("{PREFIX}/manifest-v2.json")
        ])
    );
    assert!(state.removed_documents.contains(&(
        TENANT_ID.into(),
        ELECTION_EVENT_ID.into(),
        "old-document".into()
    )));
    assert_eq!(store.stored()[0].error_message, None);
    let entries = audit.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].publication_id, "publication-1");
    assert_eq!(entries[0].action, ResultsPublicationAction::Publish);
    assert_eq!(entries[0].user_id, "admin-1");
    assert_eq!(entries[0].username.as_deref(), Some("Admin"));
}

#[tokio::test]
async fn private_finalization_removes_the_public_route_without_requiring_a_manifest() {
    let store = InMemoryResultsPublications::with([publication()]);
    let artifacts = artifacts();
    artifacts.fail_on(
        ArtifactCall::UploadLatest,
        "private publication cannot upload a public manifest",
    );
    let audit = InMemoryResultsPublicationAudit::default();
    finalize(&store, &presentations("authenticated"), &artifacts, &audit)
        .await
        .unwrap();
    assert_eq!(
        artifacts.snapshot().removed_public_paths,
        HashSet::from([
            format!("{PREFIX}/full-v1.sqlite"),
            format!("{PREFIX}/manifest-v1.json"),
            format!("{PREFIX}/manifest-latest.json")
        ])
    );
    assert!(artifacts.snapshot().removed_documents.is_empty());
    assert_eq!(audit.entries().len(), 1);
}

#[tokio::test]
async fn failures_before_audit_preserve_the_finalization_error() {
    for call in [
        ArtifactCall::UploadLatest,
        ArtifactCall::UploadIndex,
        ArtifactCall::DeletePublic,
    ] {
        let old = TallyResultsPublication {
            id: "old".into(),
            publication_status: ResultsPublicationStatus::Superseded,
            ..public_publication()
        };
        let store = InMemoryResultsPublications::with([public_publication(), old]);
        let artifacts = artifacts();
        artifacts.fail_on(call, "finalization failed");
        let audit = InMemoryResultsPublicationAudit::default();
        assert_eq!(
            finalize(&store, &presentations("public"), &artifacts, &audit)
                .await
                .unwrap_err()
                .to_string(),
            "finalization failed"
        );
        assert!(audit.entries().is_empty());
        assert_eq!(
            store.stored()[0].error_message.as_deref(),
            Some("previous finalization failed")
        );
    }
}

#[tokio::test]
async fn superseded_query_failure_occurs_after_the_latest_manifest_and_index_are_uploaded() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    store.fail_on(PublicationCall::Superseded, "superseded query failed");
    let artifacts = artifacts();
    let audit = InMemoryResultsPublicationAudit::default();
    assert_eq!(
        finalize(&store, &presentations("public"), &artifacts, &audit)
            .await
            .unwrap_err()
            .to_string(),
        "superseded query failed"
    );
    assert_eq!(artifacts.snapshot().latest_manifests.len(), 1);
    assert_eq!(artifacts.snapshot().indexes.len(), 1);
    assert!(audit.entries().is_empty());
}

#[tokio::test]
async fn audit_failure_does_not_clear_the_finalization_error() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    let artifacts = artifacts();
    let audit = InMemoryResultsPublicationAudit::default();
    audit.fail_with("audit unavailable");
    assert_eq!(
        finalize(&store, &presentations("public"), &artifacts, &audit)
            .await
            .unwrap_err()
            .to_string(),
        "audit unavailable"
    );
    assert_eq!(artifacts.snapshot().indexes.len(), 1);
    assert_eq!(
        store.stored()[0].error_message.as_deref(),
        Some("previous finalization failed")
    );
}

#[tokio::test]
async fn failure_to_clear_the_finalization_error_is_reported_after_audit() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    store.fail_on(PublicationCall::ClearFinalizationError, "clear failed");
    let artifacts = artifacts();
    let audit = InMemoryResultsPublicationAudit::default();
    assert_eq!(
        finalize(&store, &presentations("public"), &artifacts, &audit)
            .await
            .unwrap_err()
            .to_string(),
        "clear failed"
    );
    assert_eq!(audit.entries().len(), 1);
    assert_eq!(
        store.stored()[0].error_message.as_deref(),
        Some("previous finalization failed")
    );
}

#[tokio::test]
async fn revocation_cleanup_rebuilds_the_index_before_removing_revoked_artifacts() {
    let revoked = TallyResultsPublication {
        publication_status: ResultsPublicationStatus::Revoked,
        ..public_publication()
    };
    let store = InMemoryResultsPublications::with([revoked.clone()]);
    let artifacts = artifacts();
    artifacts.fail_on(ArtifactCall::UploadIndex, "index upload failed");
    assert_eq!(
        cleanup_revoked_results_publication(
            &store,
            &presentations("public"),
            &artifacts,
            TENANT_ID,
            ELECTION_EVENT_ID,
            &revoked
        )
        .await
        .unwrap_err()
        .to_string(),
        "index upload failed"
    );
    assert!(artifacts.snapshot().removed_public_paths.is_empty());
    assert_eq!(
        store.stored()[0].publication_status,
        ResultsPublicationStatus::Revoked
    );
}

#[tokio::test]
async fn an_already_revoked_publication_still_has_its_artifacts_cleaned_up() {
    let revoked = TallyResultsPublication {
        publication_status: ResultsPublicationStatus::Revoked,
        documents: json!({"document_id": "revoked-doc"}),
        ..public_publication()
    };
    let store = InMemoryResultsPublications::with([revoked.clone()]);
    let artifacts = artifacts();
    cleanup_revoked_results_publication(
        &store,
        &presentations("public"),
        &artifacts,
        TENANT_ID,
        ELECTION_EVENT_ID,
        &revoked,
    )
    .await
    .unwrap();
    assert_eq!(
        artifacts.snapshot().indexes["results-index/event-1.json"]["publications"],
        json!([])
    );
    assert_eq!(artifacts.snapshot().removed_public_paths.len(), 3);
    assert!(artifacts.snapshot().removed_documents.contains(&(
        TENANT_ID.into(),
        ELECTION_EVENT_ID.into(),
        "revoked-doc".into()
    )));
}

#[tokio::test]
async fn presentation_and_policy_errors_do_not_modify_artifacts_or_publication_state() {
    for broken_presentation in [false, true] {
        let store = InMemoryResultsPublications::with([public_publication()]);
        let presentations = InMemoryResultsEventPresentation::with(
            TENANT_ID,
            ELECTION_EVENT_ID,
            presentation(Some(json!("invalid"))),
        );
        if broken_presentation {
            presentations.fail_with("presentation unavailable");
        }
        let artifacts = artifacts();
        let error = refresh(&store, &presentations, &artifacts)
            .await
            .unwrap_err();
        assert_eq!(
            error.to_string(),
            if broken_presentation {
                "presentation unavailable"
            } else {
                "Invalid results website policy"
            }
        );
        assert!(artifacts.snapshot().removed_public_paths.is_empty());
        assert!(artifacts.snapshot().indexes.is_empty());
        assert_eq!(
            store.stored()[0].publication_status,
            ResultsPublicationStatus::Published
        );
    }
}

#[tokio::test]
async fn a_disabled_website_does_not_supersede_publications_until_its_public_prefix_is_removed() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    let artifacts = artifacts();
    artifacts.fail_on(ArtifactCall::DeletePublic, "prefix deletion failed");
    let disabled =
        InMemoryResultsEventPresentation::with(TENANT_ID, ELECTION_EVENT_ID, presentation(None));
    assert_eq!(
        refresh(&store, &disabled, &artifacts)
            .await
            .unwrap_err()
            .to_string(),
        "prefix deletion failed"
    );
    assert_eq!(
        store.stored()[0].publication_status,
        ResultsPublicationStatus::Published
    );
    assert!(artifacts.snapshot().removed_documents.is_empty());
    assert!(artifacts.snapshot().indexes.is_empty());
}

#[tokio::test]
async fn private_route_cleanup_failure_prevents_index_refresh_and_audit() {
    let store = InMemoryResultsPublications::with([publication()]);
    let artifacts = artifacts();
    artifacts.fail_on(ArtifactCall::DeletePublic, "stale route deletion failed");
    let audit = InMemoryResultsPublicationAudit::default();
    assert_eq!(
        finalize(&store, &presentations("authenticated"), &artifacts, &audit)
            .await
            .unwrap_err()
            .to_string(),
        "stale route deletion failed"
    );
    assert!(artifacts.snapshot().indexes.is_empty());
    assert!(audit.entries().is_empty());
}

#[tokio::test]
async fn a_policy_change_during_finalization_keeps_the_existing_superseded_state_error() {
    let store = InMemoryResultsPublications::with([public_publication()]);
    let artifacts = artifacts();
    let audit = InMemoryResultsPublicationAudit::default();
    let disabled =
        InMemoryResultsEventPresentation::with(TENANT_ID, ELECTION_EVENT_ID, presentation(None));
    let error = finalize(&store, &disabled, &artifacts, &audit)
        .await
        .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Published publication was not available to update its finalization error"
    );
    assert_eq!(
        store.stored()[0].publication_status,
        ResultsPublicationStatus::Superseded
    );
    assert_eq!(
        store.stored()[0].error_message.as_deref(),
        Some("previous finalization failed")
    );
    assert_eq!(audit.entries().len(), 1);
    assert_eq!(
        artifacts.snapshot().indexes["results-index/event-1.json"]["publications"],
        json!([])
    );
}

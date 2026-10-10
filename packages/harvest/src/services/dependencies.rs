// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::cast_votes::WindmillCastVotes;
use crate::adapters::database::WindmillDatabasePools;
use crate::adapters::documents::S3DocumentStorage;
use crate::adapters::electoral_log::BoardElectoralLogs;
use crate::adapters::identity::KeycloakIdentityAdmin;
use crate::adapters::monitoring_renderer::HttpMonitoringRenderer;
use crate::adapters::monitoring_snapshots::WindmillMonitoringSnapshots;
use crate::adapters::task_ledger::WindmillTaskLedger;
use crate::adapters::task_queue::CeleryTaskQueue;
use crate::adapters::vault::WindmillVault;
use crate::ports::cast_votes::CastVotes;
use crate::ports::database::DatabasePools;
use crate::ports::documents::DocumentStorage;
use crate::ports::electoral_log::ElectoralLogs;
use crate::ports::identity::IdentityAdmin;
use crate::ports::monitoring_renderer::MonitoringRenderer;
use crate::ports::monitoring_snapshots::MonitoringSnapshots;
use crate::ports::task_ledger::TaskLedger;
use crate::ports::task_queue::TaskQueue;
use crate::ports::vault::SecretVault;
use crate::services::monitoring::{DrawFailure, DrawnChart};
use crate::services::monitoring_cache::RenderCache;
use crate::services::monitoring_config_cache::MonitoringConfigs;
use sequent_core::monitoring::cadence::Cadence;
use std::sync::Arc;
use windmill::services::consolidation::signed_transmission_package::AnnotatedSbeis;
use windmill::services::monitoring::audit::ElectoralLogConfigAudit;
use windmill::services::monitoring::cadence;
use windmill::services::monitoring::config_store::MonitoringConfigAudit;
use windmill::services::signing::actions::eml::{
    DocumentSigners, EmlDocumentSigner,
};
use windmill::services::signing::approve::SigningServices;
use windmill::services::signing::certificates::OpensslCertificateVerifier;
use windmill::services::signing::executors::default_registry;
use windmill::services::signing::pdf::{PdfDocumentSigner, S3RevisionStore};
use windmill::services::signing::requests::DocumentExportStore;
use windmill::services::signing::rules::{
    KeycloakSigningRoleAdmin, SigningRoleAdmin,
};

/// The charts monitoring widgets were drawn as.
pub type MonitoringCache = RenderCache<DrawnChart, DrawFailure>;

/// What route handlers reach outside Harvest through. Rocket manages one
/// instance: the production adapters in the service, fakes or a test
/// database in route tests.
pub struct HarvestServices {
    pub cast_votes: Arc<dyn CastVotes>,
    pub databases: Arc<dyn DatabasePools>,
    pub documents: Arc<dyn DocumentStorage>,
    pub electoral_log: Arc<dyn ElectoralLogs>,
    pub identity: Arc<dyn IdentityAdmin>,
    pub monitoring_audit: Arc<dyn MonitoringConfigAudit>,
    pub monitoring_cache: Arc<MonitoringCache>,
    pub monitoring_configs: Arc<MonitoringConfigs>,
    /// How often figures are counted: what the dashboards poll at.
    pub monitoring_cadence: Cadence,
    pub monitoring_renderer: Arc<dyn MonitoringRenderer>,
    pub monitoring_snapshots: Arc<dyn MonitoringSnapshots>,
    pub ledger: Arc<dyn TaskLedger>,
    pub tasks: Arc<dyn TaskQueue>,
    pub vault: Arc<dyn SecretVault>,
    /// What an approval checks certificates with and runs actions with.
    pub signing: SigningServices,
    /// Changes which groups can sign an action.
    pub signing_roles: Arc<dyn SigningRoleAdmin>,
}

impl HarvestServices {
    pub fn production() -> Self {
        let databases: Arc<dyn DatabasePools> = Arc::new(WindmillDatabasePools);
        Self {
            cast_votes: Arc::new(WindmillCastVotes),
            databases: databases.clone(),
            documents: Arc::new(S3DocumentStorage),
            electoral_log: Arc::new(BoardElectoralLogs),
            identity: Arc::new(KeycloakIdentityAdmin),
            monitoring_audit: Arc::new(ElectoralLogConfigAudit),
            monitoring_cache: Arc::new(MonitoringCache::from_env()),
            monitoring_configs: Arc::new(MonitoringConfigs::from_env()),
            monitoring_cadence: cadence::from_env(),
            monitoring_renderer: Arc::new(HttpMonitoringRenderer::from_env()),
            monitoring_snapshots: Arc::new(WindmillMonitoringSnapshots {
                databases,
            }),
            ledger: Arc::new(WindmillTaskLedger),
            tasks: Arc::new(CeleryTaskQueue),
            vault: Arc::new(WindmillVault),
            signing: SigningServices {
                verifier: Arc::new(OpensslCertificateVerifier::default()),
                executors: default_registry(),
                documents: Arc::new(DocumentSigners::new(
                    Arc::new(EmlDocumentSigner::new(
                        Arc::new(S3RevisionStore),
                        Arc::new(AnnotatedSbeis),
                    )),
                    Arc::new(PdfDocumentSigner::new(Arc::new(S3RevisionStore))),
                )),
                exports: Arc::new(DocumentExportStore),
            },
            signing_roles: Arc::new(KeycloakSigningRoleAdmin),
        }
    }
}

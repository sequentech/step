// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::cast_votes::WindmillCastVotes;
use crate::adapters::database::WindmillDatabasePools;
use crate::adapters::documents::S3DocumentStorage;
use crate::adapters::electoral_log::BoardElectoralLogs;
use crate::adapters::identity::KeycloakIdentityAdmin;
use crate::adapters::task_ledger::WindmillTaskLedger;
use crate::adapters::task_queue::CeleryTaskQueue;
use crate::adapters::vault::WindmillVault;
use crate::ports::cast_votes::CastVotes;
use crate::ports::database::DatabasePools;
use crate::ports::documents::DocumentStorage;
use crate::ports::electoral_log::ElectoralLogs;
use crate::ports::identity::IdentityAdmin;
use crate::ports::task_ledger::TaskLedger;
use crate::ports::task_queue::TaskQueue;
use crate::ports::vault::SecretVault;
use std::sync::Arc;

/// What route handlers reach outside Harvest through. Rocket manages one
/// instance: the production adapters in the service, fakes or a test
/// database in route tests.
pub struct HarvestServices {
    pub cast_votes: Arc<dyn CastVotes>,
    pub databases: Arc<dyn DatabasePools>,
    pub documents: Arc<dyn DocumentStorage>,
    pub electoral_log: Arc<dyn ElectoralLogs>,
    pub identity: Arc<dyn IdentityAdmin>,
    pub ledger: Arc<dyn TaskLedger>,
    pub tasks: Arc<dyn TaskQueue>,
    pub vault: Arc<dyn SecretVault>,
}

impl HarvestServices {
    pub fn production() -> Self {
        Self {
            cast_votes: Arc::new(WindmillCastVotes),
            databases: Arc::new(WindmillDatabasePools),
            documents: Arc::new(S3DocumentStorage),
            electoral_log: Arc::new(BoardElectoralLogs),
            identity: Arc::new(KeycloakIdentityAdmin),
            ledger: Arc::new(WindmillTaskLedger),
            tasks: Arc::new(CeleryTaskQueue),
            vault: Arc::new(WindmillVault),
        }
    }
}

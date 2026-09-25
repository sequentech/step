// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::adapters::database::WindmillDatabasePools;
use crate::adapters::documents::S3DocumentStorage;
use crate::adapters::identity::KeycloakIdentityAdmin;
use crate::adapters::task_ledger::WindmillTaskLedger;
use crate::adapters::task_queue::CeleryTaskQueue;
use crate::ports::database::DatabasePools;
use crate::ports::documents::DocumentStorage;
use crate::ports::identity::IdentityAdmin;
use crate::ports::task_ledger::TaskLedger;
use crate::ports::task_queue::TaskQueue;
use std::sync::Arc;

/// What route handlers reach outside Harvest through. Rocket manages one
/// instance: the production adapters in the service, fakes or a test
/// database in route tests.
pub struct HarvestServices {
    pub databases: Arc<dyn DatabasePools>,
    pub documents: Arc<dyn DocumentStorage>,
    pub identity: Arc<dyn IdentityAdmin>,
    pub ledger: Arc<dyn TaskLedger>,
    pub tasks: Arc<dyn TaskQueue>,
}

impl HarvestServices {
    pub fn production() -> Self {
        Self {
            databases: Arc::new(WindmillDatabasePools),
            documents: Arc::new(S3DocumentStorage),
            identity: Arc::new(KeycloakIdentityAdmin),
            ledger: Arc::new(WindmillTaskLedger),
            tasks: Arc::new(CeleryTaskQueue),
        }
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

use crate::ports::service_keys::ServiceKeySource;
use anyhow::anyhow;
use jsonwebtoken::jwk::JwkSet;
use std::collections::HashMap;
use std::sync::Mutex;

/// Key sets per realm, set by the test. A realm without one fails like an
/// unreachable Keycloak.
#[derive(Default)]
pub struct StaticServiceKeys {
    sets: Mutex<HashMap<String, JwkSet>>,
    /// Every realm whose keys were asked for, in order.
    pub fetches: Mutex<Vec<String>>,
}

impl StaticServiceKeys {
    pub fn publish(&self, realm: &str, keys: JwkSet) {
        self.sets.lock().unwrap().insert(realm.to_string(), keys);
    }

    /// The realm's keys can no longer be fetched, as when Keycloak is down.
    pub fn withdraw(&self, realm: &str) {
        self.sets.lock().unwrap().remove(realm);
    }

    pub fn fetch_count(&self) -> usize {
        self.fetches.lock().unwrap().len()
    }
}

#[rocket::async_trait]
impl ServiceKeySource for StaticServiceKeys {
    async fn realm_keys(&self, realm: &str) -> anyhow::Result<JwkSet> {
        self.fetches.lock().unwrap().push(realm.to_string());
        self.sets
            .lock()
            .unwrap()
            .get(realm)
            .cloned()
            .ok_or_else(|| anyhow!("no keys published for {realm}"))
    }
}

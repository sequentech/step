// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

//! A software stand-in for an HSM that holds an election event's cast-vote
//! key, for tests only: a P-256 key generated in memory whose ECDH runs in
//! OpenSSL, opening `hsm-seal-v1` values as a device would. Include it with
//! `#[path = "support/cast_vote_opener.rs"] mod cast_vote_opener;`.

use anyhow::{ensure, Result};
use openssl::bn::BigNumContext;
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey, EcPoint, PointConversionForm};
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use std::sync::atomic::{AtomicUsize, Ordering};
use windmill::services::cast_vote_seal::{
    format, CastVoteOpener, OpenFuture, OpenItem, OpenResults, SealKey, MAX_OPEN_BATCH,
};

pub struct SoftwareOpener {
    key: PKey<Private>,
    key_id: String,
    /// Batches opened.
    pub batches: AtomicUsize,
}

impl SoftwareOpener {
    /// A new device key for an event, and its published form.
    pub fn generate(tenant_id: &str, election_event_id: &str) -> (Self, SealKey) {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let key = EcKey::generate(&group).unwrap();
        let mut ctx = BigNumContext::new().unwrap();
        let point = key
            .public_key()
            .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut ctx)
            .unwrap();
        let key_id = electoral_log::cast_vote_seal::key_id(tenant_id, election_event_id).unwrap();
        (
            SoftwareOpener {
                key: PKey::from_ec_key(key).unwrap(),
                key_id: key_id.clone(),
                batches: AtomicUsize::new(0),
            },
            SealKey::new(key_id, 1, point),
        )
    }

    fn shared(&self, point: &[u8]) -> Result<Vec<u8>> {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1)?;
        let mut ctx = BigNumContext::new()?;
        let point = EcPoint::from_bytes(&group, point, &mut ctx)?;
        let peer = PKey::from_ec_key(EcKey::from_public_key(&group, &point)?)?;
        let mut deriver = Deriver::new(&self.key)?;
        deriver.set_peer(&peer)?;
        Ok(deriver.derive_to_vec()?)
    }

    fn open_now(
        &self,
        key_id: &str,
        key_version: u32,
        items: Vec<OpenItem>,
    ) -> Result<OpenResults> {
        ensure!(key_id == self.key_id && key_version == 1, "no such key");
        ensure!(
            !items.is_empty() && items.len() <= MAX_OPEN_BATCH,
            "batch size"
        );
        self.batches.fetch_add(1, Ordering::SeqCst);
        Ok(items
            .iter()
            .map(|item| {
                format::ephemeral_point(&item.sealed)
                    .and_then(|point| self.shared(point))
                    .and_then(|shared| {
                        format::open_with_shared(
                            &shared,
                            key_id,
                            key_version,
                            &item.sealed,
                            &item.aad,
                        )
                    })
                    .map_err(|error| error.to_string())
            })
            .collect())
    }
}

impl CastVoteOpener for SoftwareOpener {
    fn open_batch<'a>(
        &'a self,
        key_id: &'a str,
        key_version: u32,
        items: Vec<OpenItem>,
    ) -> OpenFuture<'a> {
        Box::pin(async move { self.open_now(key_id, key_version, items) })
    }
}

/// Every event's opener, by key id: one installed opener per test binary.
#[derive(Default)]
pub struct Openers {
    openers: std::sync::Mutex<Vec<std::sync::Arc<SoftwareOpener>>>,
}

impl Openers {
    pub fn add(&self, opener: std::sync::Arc<SoftwareOpener>) {
        self.openers.lock().unwrap().push(opener);
    }
}

impl CastVoteOpener for Openers {
    fn open_batch<'a>(
        &'a self,
        key_id: &'a str,
        key_version: u32,
        items: Vec<OpenItem>,
    ) -> OpenFuture<'a> {
        Box::pin(async move {
            let opener = self
                .openers
                .lock()
                .unwrap()
                .iter()
                .find(|opener| opener.key_id == key_id)
                .cloned();
            match opener {
                Some(opener) => opener.open_now(key_id, key_version, items),
                None => Err(anyhow::anyhow!("no such key")),
            }
        })
    }
}

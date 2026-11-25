// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Test support: a software stand-in for the HSM, installed once for the
//! whole test binary, holding every key a test makes with [`event_key`].

use super::*;
use openssl::bn::BigNumContext;
use openssl::derive::Deriver;
use openssl::ec::{EcGroup, EcKey, EcPoint, PointConversionForm};
use openssl::nid::Nid;
use openssl::pkey::{PKey, Private};
use sequent_core::ballot::{SignedHashableBallot, TYPES_VERSION};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// A software stand-in for the HSM: the device key's ECDH in OpenSSL.
pub(crate) struct SoftwareOpener {
    key: PKey<Private>,
    pub(crate) calls: AtomicUsize,
}

impl SoftwareOpener {
    pub(crate) fn generate() -> (Self, Vec<u8>) {
        let group = EcGroup::from_curve_name(Nid::X9_62_PRIME256V1).unwrap();
        let key = EcKey::generate(&group).unwrap();
        let mut ctx = BigNumContext::new().unwrap();
        let point = key
            .public_key()
            .to_bytes(&group, PointConversionForm::UNCOMPRESSED, &mut ctx)
            .unwrap();
        (
            SoftwareOpener {
                key: PKey::from_ec_key(key).unwrap(),
                calls: AtomicUsize::new(0),
            },
            point,
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

impl SoftwareOpener {
    fn open_now(
        &self,
        key_id: &str,
        key_version: u32,
        items: Vec<OpenItem>,
    ) -> Result<OpenResults> {
        ensure!(items.len() <= MAX_OPEN_BATCH, "batch too large");
        self.calls.fetch_add(1, Ordering::SeqCst);
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

/// A voter's ballot (no contests, which hashing allows), its Ballot ID and
/// its cast-vote hash.
pub(crate) fn ballot(seed: &str) -> (String, String, Vec<u8>) {
    let content = serde_json::to_string(&SignedHashableBallot {
        version: TYPES_VERSION,
        issue_date: seed.to_string(),
        contests: vec![],
        config: seed.to_string(),
        ballot_style_hash: seed.to_string(),
        voter_signing_pk: None,
        voter_ballot_signature: None,
    })
    .unwrap();
    let hashes = computed_hashes(&content, "hashable-ballot").unwrap();
    (content, hashes.ballot_id, hashes.cast_vote_hash.to_vec())
}

/// The test binary's installed opener: every key [`event_key`] made.
#[derive(Default)]
struct Keys {
    keys: Mutex<Vec<(String, Arc<SoftwareOpener>)>>,
}

impl CastVoteOpener for Keys {
    fn open_batch<'a>(
        &'a self,
        key_id: &'a str,
        key_version: u32,
        items: Vec<OpenItem>,
    ) -> OpenFuture<'a> {
        let opener = self
            .keys
            .lock()
            .unwrap()
            .iter()
            .find(|(id, _)| id == key_id)
            .map(|(_, opener)| opener.clone());
        Box::pin(async move {
            match opener {
                Some(opener) => opener.open_now(key_id, key_version, items),
                None => Err(anyhow!("no such key")),
            }
        })
    }
}

static KEYS: once_cell::sync::Lazy<Arc<Keys>> = once_cell::sync::Lazy::new(|| {
    let keys = Arc::new(Keys::default());
    install(keys.clone()).expect("the test opener is the only one installed");
    keys
});

/// A new key of the event in the installed test opener.
pub(crate) fn event_key(tenant_id: &str, election_event_id: &str) -> SealKey {
    let (opener, point) = SoftwareOpener::generate();
    let key = SealKey::new(
        event_key_id(tenant_id, election_event_id).unwrap(),
        1,
        point,
    );
    KEYS.keys
        .lock()
        .unwrap()
        .push((key.key_id.clone(), Arc::new(opener)));
    key
}

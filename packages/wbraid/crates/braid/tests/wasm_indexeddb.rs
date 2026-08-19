// SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

//! Headless-browser test for the wasm IndexedDB persistence backend (M3-B).
//!
//! Run with a headless browser (Chrome), e.g. `wasm-pack test --headless
//! --chrome` (see the wasm build's atomics toolchain). It is compiled only for
//! `wasm32` under the `wasm` feature, so native `cargo test` skips it entirely.
//!
//! The board client's anti-rewrite `collides()` logic is platform-agnostic and
//! already covered natively (`board::tests`); what needs a real browser is the
//! IndexedDB I/O itself, so this focuses on the round-trip + idempotency of
//! [`IndexedDbPersistence`].

#![cfg(all(target_arch = "wasm32", feature = "wasm-core"))]

use wasm_bindgen_test::*;

use braid::board::persistence::Persistence;
use braid::wasm::persistence::IndexedDbPersistence;

use braid::messages::newtypes::{
    zero_hash, CiphertextsHash, ConfigurationHash, PublicKeyHash, SharesHash,
};
use braid::messages::predicate::{Mix, Predicate, Shares};

wasm_bindgen_test_configure!(run_in_browser);

/// Two distinct predicates (dummy zero hashes — persistence never inspects
/// bodies, only serializes the predicate tuple).
fn sample_predicates() -> Vec<Predicate> {
    let configuration = ConfigurationHash(zero_hash());
    vec![
        Predicate::Shares(Shares {
            configuration,
            shares: SharesHash(zero_hash()),
            sender: 1,
        }),
        Predicate::Mix(Mix {
            configuration,
            public_key: PublicKeyHash(zero_hash()),
            input: CiphertextsHash(zero_hash()),
            output: CiphertextsHash(zero_hash()),
            sender: 2,
        }),
    ]
}

#[wasm_bindgen_test]
async fn indexeddb_round_trips_predicates() {
    // Unique DB name per run so a shared browser profile never leaks state.
    let db_name = format!("braid_test_{}", js_sys::Date::now() as u64);

    let mut persistence = IndexedDbPersistence::open(&db_name)
        .await
        .expect("open IndexedDB");
    let predicates = sample_predicates();
    for predicate in &predicates {
        persistence.persist(predicate).await.expect("persist");
    }
    // Re-persisting an identical predicate is idempotent (same key overwrites).
    persistence
        .persist(&predicates[0])
        .await
        .expect("persist dup");
    drop(persistence);

    // Reopen (simulating a restart) and load the committed set back.
    let reopened = IndexedDbPersistence::open(&db_name)
        .await
        .expect("reopen IndexedDB");
    let loaded = reopened.load().await.expect("load");

    assert_eq!(
        loaded.len(),
        predicates.len(),
        "round-trip preserves the set (and the duplicate did not add an entry)"
    );
    for predicate in &predicates {
        assert!(
            loaded.contains(predicate),
            "predicate must survive persist + reopen + load"
        );
    }
}

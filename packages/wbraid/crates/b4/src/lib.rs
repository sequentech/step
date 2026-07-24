// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

pub mod api_types;

// Native-only modules
#[cfg(feature = "native")]
pub mod db;
#[cfg(feature = "native")]
pub mod handlers;
#[cfg(feature = "native")]
pub mod s3;
#[cfg(feature = "native")]
pub mod state;

/// Seconds elapsed since `std::time::UNIX_EPOCH`.
pub type Timestamp = u64;
use cryptography::utils::hash::Hasher as HasherTrait;

#[cfg(feature = "native")]
use std::time::{SystemTime, UNIX_EPOCH};

/// The Hasher instance as defined by the cryptography library.
pub type Hasher = cryptography::context::CryptographicHasher;

/// The Hash output type as defined by the cryptography library.
pub type CryptographicHash = sha3::digest::Output<Hasher>;

#[cfg(feature = "native")]
pub fn timestamp() -> Timestamp {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Impossible with respect to UNIX_EPOCH");

    since_the_epoch.as_secs()
}

#[cfg(target_arch = "wasm32")]
pub fn timestamp() -> Timestamp {
    // Use JavaScript Date.now() for WASM (returns milliseconds since epoch)
    (js_sys::Date::now() / 1000.0) as u64
}

pub fn get_schema_version() -> String {
    "1".to_string()
}

/// Hash bytes to produce a [`CryptographicHash`].
///
/// This deliberately reaches for the library's global default hasher
/// ([`Hasher`]) rather than threading through a [`Context`]'s hasher
/// (`C::get_hasher()`). The output type is `CryptographicHash`, a fixed
/// wire/storage/datalog format that must NOT vary per context instantiation, so
/// pinning it to the global default is intentional. Sourcing the hasher from a
/// `Context` would only typecheck under the bound `C::Hasher == CryptographicHasher`
/// — i.e. we would have to relate the context back to this independently-defined
/// type — which adds noise without changing behavior (every context uses the same
/// 512-bit hasher). Threading through `Context` pays off for operations whose
/// output type is genuinely parameterized by the context (group elements, scalars,
/// signatures), not for a format-pinned hash.
///
/// [`Context`]: cryptography::context::Context
pub fn hash_bytes(bytes: &[u8]) -> CryptographicHash {
    use sha3::Digest;
    let mut hasher = Hasher::hasher();
    hasher.update(bytes);
    hasher.finalize()
}

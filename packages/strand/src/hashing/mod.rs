// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(any(feature = "openssl_core", feature = "openssl_full"))]
pub mod openssl;
#[cfg(not(any(feature = "openssl_core", feature = "openssl_full")))]
pub mod rustcrypto;

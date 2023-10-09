// SPDX-FileCopyrightText: 2021 David Ruescas <david@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
#[cfg(feature = "openssl")]
pub mod openssl;
#[cfg(not(feature = "openssl"))]
pub mod rand;

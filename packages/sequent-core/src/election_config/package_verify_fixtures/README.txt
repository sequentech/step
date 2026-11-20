SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only

TEST ONLY throwaway keys for package_verify_v2_tests.rs (format-2 RSA-PSS
approvals and final signatures). Generated with:
  openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048
They protect nothing and are trusted by nothing outside these tests.
rsa-1024.spki.der and rsa-6144.spki.der are public keys only, for the
2048-4096-bit modulus bounds.

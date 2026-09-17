---
id: ecies-encryption
title: ECIES command-line tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

From `packages/ECIESEncryption`, run `mvn test` with Java 17 and Maven 3.9.9.
The test dependencies and Surefire runner are pinned in the POM. An offline
worker needs their Maven artifacts fetched before `mvn -o test`.

Tests invoke the real CLI in bounded child JVMs because its command dispatcher
calls `System.exit`. Each test owns a temporary directory and synthetic keys;
child output is redirected to files to avoid blocked pipes. No service or
production certificate is needed.

Generated PEM keys must load into the independent JDK EC provider. JDK-generated
EC and RSA signatures supply verification controls; changed content must fail
verification. ECIES tests use the provider directly at the cryptographic boundary
to check literal plaintext bytes and reject modified authentication tags.
Bulk-signing tests verify binary-file signatures and preserve existing signature
files and directories. Usage checks assert both exit status and diagnostic text.

These are command-line integration contracts, not a cryptographic primitive
certification. PKCS12 certificate-chain and expiry handling need additional
certificate fixtures. No source coverage percentage is published: subprocess JVM
instrumentation and a comparable baseline require a separate coverage profile.

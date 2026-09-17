---
id: e2e-helpers
title: E2E helper tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

From `packages/`, with the pinned Rust toolchain:

```sh
cargo test --locked -p e2e -p mock_server
cargo clippy --locked -p e2e -p mock_server --bins
```

These are local integration tests for the load-test HTTP client and synthetic
identity mock server. They do not run paid Loadero scenarios. The HTTP fixture
owns its loopback port, bounds accepts and reads, inspects request paths and
payloads, and returns scripted status/body pairs. Tests restore serialized
environment changes. HTTP polling failures must propagate to the caller instead
of returning success.

Mock-server fixtures own a temporary working directory because the helper opens
`voters.db` relative to the process directory. The suite serializes and restores
that directory, maps literal CSV columns and dates through real SQLite, checks
country filtering with bound parameters, and invokes Rocket's local client.
Malformed database rows must remain errors, not disappear into an empty result.

The JavaScript scenario scripts, deployed browser sessions and remote identity
services require separate environments. No source percentage is published for
these test-free baseline packages; a passing local suite is not a measured
coverage comparison or proof of remote scenario success.

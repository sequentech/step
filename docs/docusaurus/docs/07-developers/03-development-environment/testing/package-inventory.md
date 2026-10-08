---
id: package-test-inventory
title: Package test inventory
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

A directory under `packages/` is not necessarily a member of the root Rust or
JavaScript workspace. Check `packages/Cargo.toml`, `packages/package.json`, nested
Cargo workspaces and Maven reactors before interpreting a package test run.

| Area | Test entry point | Separate boundary |
| --- | --- | --- |
| Bulletin board and trustee protocol | [B3](./b3.md), [Braid](./braid.md) | Browser storage and remote transport |
| ImmuDB client and board mapping | [Client](./immudb-rs.md), [row mapping](./immu-board.md) | Server/storage compatibility |
| Frontends | [Results Portal](./results-portal.md), [Ballot Verifier](./ballot-verifier.md), [Admin Portal](./admin-portal.md) | Browser flows and live services |
| Java utilities and extensions | [ECIES](./ecies-encryption.md), [Keycloak](./keycloak-extensions.md) | Certificate chains and deployed authentication |
| Load-test helpers | [E2E helpers](./e2e-helpers.md) | Remote browser/load scenarios |
| Runtime adapters and rendering | [Orare](./orare.md) | Deployed HTTP and S3 transfer |
| Plugin component | [Miru](./miru.md) | Host JWT and transaction implementations |

The `wbraid` directory, when present, is a separate Cargo workspace with existing
protocol, model/property, persistence and cross-implementation tests. Run from
`packages/wbraid`:

```sh
cargo test --locked --release --features sqlite,postgres
```

Read that workspace's feature and integration-test prerequisites before enabling
additional backends. Ignored external-service/JVM cases and the live B4/LocalStack
fixtures remain separate obligations; enabling a feature alone does not prove
its external boundary was exercised.

`board-messages/src/electoral_log/mod.rs` is a dormant source fragment: the
directory has no Cargo manifest or workspace member, and its declared artifact,
message, newtypes and statement submodules are absent. Its ignored historical
ImmuDB tests therefore are not executable tests. Do not claim consumer coverage
or count this fragment as compiled source. Restoring a manifest, those modules,
or a consumer invalidates this inventory exception and requires a fresh test
and source-scope review.

Use the coverage profiles only for their named configurations. A passing package
suite without an instrumented source inventory has no implied percentage.
Unimported frontend source remains counted; test-only setup/declarations and
separate browser/WASM/runtime profiles must be accounted for explicitly.

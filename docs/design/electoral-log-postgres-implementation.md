<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# OVCS PostgreSQL electoral log

Parent issue: https://github.com/sequentech/meta/issues/13698

This ports the PostgreSQL replacement and ImmuDB retirement from Step PR 3420 onto OVCS. It is a breaking storage replacement for fresh installations, without historical data migration. The dedicated electoral-log database uses the existing PostgreSQL server.

OVCS-specific integration preserves signed-message encoding, the durable RabbitMQ dispatcher, legacy batch decoding, event timestamps, monitoring counters and acknowledgement only after confirmed persistence. Per-board writes are atomic. Stored input fingerprints reject conflicting delivery identities, while confirmed retries retain their original signed bytes. The OVCS coverage/CI and offline-build structure remain in place with PostgreSQL replacing ImmuDB fixtures and artifacts.

The runtime Beyond submodule pin is unchanged. No deployment or cloud apply is part of this PR. Provisioning requirements are documented in the Docusaurus PostgreSQL electoral-log guide.

## Validation

Validation of this OVCS revision is recorded below; results from the main-targeted PR are not treated as validation of this port.

- `cargo test --locked -p electoral-log --features postgres-tests -- --test-threads=1`: 71 tests passed, including five contracts against disposable PostgreSQL 17, signed messages, OVCS monitoring/wire compatibility and CLI validation.
- `cargo clippy --locked -p electoral-log --all-targets --features postgres-tests`: completed with warnings; no lint errors.
- `cargo fmt -p electoral-log -- --check`: passed. Changed caller Rust files were formatted separately.
- Focused CI/e2e tooling: 37 tests passed. Coverage reporter/tooling: 100 tests passed (synthetic Git fixtures use unsigned commits and a canonical temporary path on macOS).
- Development Compose, CI overlay and coverage overlay configuration checks passed. Changed YAML, JSON, TOML, GraphQL schemas, TypeScript syntax and shell syntax checks passed.
- REUSE lint passed. Signed-message source and the Beyond gitlink are unchanged from OVCS.

Local validation uses an isolated checkout and a task-owned PostgreSQL instance; existing development services are untouched. Full Docker/RabbitMQ integration, portal builds, cloud rollout and coverage percentage comparisons are separate CI/deployment checks, not claimed as local passes.

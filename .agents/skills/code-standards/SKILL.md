---
name: code-standards
description: Apply the product design rules and Rust/TypeScript quality rules reviewers enforce, and the new-feature checklist. Use when designing a feature, writing or reviewing code, or preparing a PR.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Code standards

The canonical list is the Coding Standards section of the
[contributing guide](../../../docs/docusaurus/docs/07-developers/00-contributing.md);
production lint rules are in the
[assurance lint policy](../../../docs/docusaurus/docs/07-developers/03-development-environment/production-assurance.md).

## Product rules

- No client-specific features: design the general solution for every client.
- Features compose; unsupported combinations are rejected or explained, never
  silently broken.
- Policies and configuration modes are enums, not booleans (Rust and TS).
- Stored and exported data stays backwards compatible: new fields are optional
  or defaulted so older election events still import.

## Code rules

- TDD: write the test, see it fail, implement, see it pass.
- Rust: propagate `Option`/`Result` instead of unwrapping in production code;
  reuse `sequent_core::types`; named constants and enums instead of string
  literals.
- TypeScript: no `any`; await promises and handle their errors; service-prefixed
  environment variables, also added to the Docker Compose files.
- No dead code, commented-out code, generated boilerplate comments or unlinked
  TODOs. Keep existing comments that explain non-obvious logic.

## New feature checklist

- Docs under `docs/docusaurus/`.
- New Keycloak permissions in `IPermissions`
  (`packages/admin-portal/src/types/keycloak.ts`) and the default tenant realm
  under `.devcontainer/keycloak/import/`.
- Unit tests including rejected input and edge cases
  ([implement-unit-tests](../implement-unit-tests/SKILL.md)).
- Frontend checked at several screen sizes.
- REUSE headers on every new file.

## Developer docs

Put guides in the matching section of `docs/docusaurus/docs/07-developers/`;
package READMEs link there. Keep durable content (setup, commands, contracts,
interpretation); issue numbers, targets and progress belong in issues and PRs.
Check moved links and the sidebar `_category_.yml` files.

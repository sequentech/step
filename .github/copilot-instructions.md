<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# Copilot Code Review Instructions

Review this repository as election software, not a generic web app. Prioritize findings that can change counted ballots, voter eligibility, auditability, tenant isolation, permissions, privacy, or backwards compatibility. Prefer correctness and security issues over style comments.

Detailed review rules are intentionally split into files under .github/instructions so GitHub.com Copilot code review can apply more specific guidance while staying within the 4000-character per-file limit.

## System Mindset

- Treat changes in packages/windmill, packages/sequent-core, packages/harvest, packages/immu-board, packages/keycloak-extensions, packages/voting-portal, packages/admin-portal, and beyond/packages/ballot-audit as high risk.
- Review for election-system invariants: outcome correctness, deterministic behavior, reproducible audits, tenant isolation, voter privacy, and safe failure modes.
- Flag bugs that can silently drop ballots, count stale or duplicate ballots, mis-handle area rules, widen eligibility, expose voter-linked data, or make two code paths disagree about the same election state.
- Be suspicious of best-effort parsing, skipped records, silent fallbacks, lossy transforms, non-deterministic ordering, retry logic, and special cases for one client. In this system those are often integrity bugs, not harmless implementation details.

## High-Priority Review Areas

- Ballot selection and counting: verify that latest valid ballot semantics, deduplication rules, and area scoping stay consistent across tally, statistics, exports, receipts, and audits.
- Authorization and tenancy: verify server-side enforcement, correct Keycloak and Hasura scoping, and no cross-tenant or cross-event data exposure.
- Election state transitions: review open or closed voting logic, eligibility checks, revote rules, ceremony steps, and admin-only actions as security-sensitive.
- Cryptography and auditability: flag any weakening of proof, signature, hash, encryption, randomness, immutable-log, or canonical serialization checks.
- Compatibility and data flow: review imports, exports, migrations, generated GraphQL types, and persisted payloads for backwards compatibility and deterministic semantics.
- Frontend correctness matters when it changes security or election behavior: contest state, permission-gated actions, public documents, and HTML or rich-text rendering must remain correct and sanitized.

## Product and Tests

- Flag client-specific behavior that is not generalized. Prefer composable policies and enums over booleans when more modes are likely.
- Expect targeted tests for changes to tally, cast_vote, revotes, permissions, state transitions, migrations, import or export, or cryptographic validation.
- Strong tests cover revotes, multi-area behavior, deterministic ballot selection, authorization failures, wrong-tenant access, invalid input, and backwards compatibility with existing election data.
- If no material integrity, security, privacy, or compatibility issue is present, prefer no finding over low-value style commentary.

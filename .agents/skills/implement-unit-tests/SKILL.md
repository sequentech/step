---
name: implement-unit-tests
description: Add or improve meaningful unit tests and investigate test coverage gaps in an existing package. Use for regression tests and focused coverage work.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Implement unit tests

Read repository guidance, package test documentation and relevant source. Reuse
test conventions, pinned tools and coverage profiles. Check an existing PR’s
latest CI/review and actual base. Preserve scope, protected behavior and unrelated
working-tree changes.

When an issue tracks PRs or stacks for multiple target versions, find the matching
PR in each stack and carry equivalent tests and fixes across where practical.
Adapt to each version's APIs, validate against its own base, and record justified
differences in the issue; do not import unrelated changes from another version.

## Test contracts

- Exercise observable results, boundaries, rejected inputs and failure propagation
  through public entry points. Verify the intended error, not merely any failure.
- Use independent expectations: literal wire bytes, hand-calculated values or
  documented rules. Neither copied implementation logic nor round trips alone
  provide an independent oracle.
- Start rejection cases from a valid control and change one relevant property.
  Check absent, partial and complete combinations of related optional fields;
  malformed input must not silently become a default or an unsigned/empty result.
- Name tests for behavior; comment on the invariant or failure being isolated.
  Prefer simple fixtures and direct assertions. Fail-fast setup, including Rust
  `unwrap`/`expect`/indexing, is appropriate where test policy permits it.
- Match production compiler/target settings when emitted behavior matters. Observe
  the boundary where failure occurs; later cleanup or state updates can hide it.
- For a bug, demonstrate the failing regression before the smallest production
  fix. Do not refactor or alter runtime settings merely for coverage.

## Fixtures and generated code

- Use disposable isolated environments, synthetic data, no production credentials
  and existing/free tooling. Bound waits, clean up resources, and isolate or
  restore global state, caches and environment variables.
- Keep fixtures deterministic: retain ownership of allocated ports, synchronize
  on readiness rather than sleeps, and avoid assertions that require wall-clock
  time to advance. Gate feature-specific test files with their source modules.
- Mock external boundaries, not the behavior under test. When real protocol or
  database mapping matters, use bounded local service fixtures and label those
  checks as integration tests. Include success controls for injected failures.
- Use parameterized tests/macros with explicit cases and expectations. Test
  serialization with known bytes, truncation, trailing data and controlled writer
  failures; include a successful full-capacity control.

## Coverage and completion

- Inspect uncovered source and functions. Investigate suspicious generated-code
  counters with a minimal reproducer. Keep generated serialization counted;
  avoid exercising debug/clone/printing code solely for a score.
- Enable supported features and run their tests with separate coverage profiles.
  A disabled feature is not a justification for leaving its code untested.
- Document concrete residual gaps: location, why the path is unreachable or
  costly to test, and what would invalidate that reasoning. Unmeasured files,
  optional features, platforms and real branches are separate obligations;
  LLVM regions are not branch coverage. Keep mixed inline test code visible.
- Apply justified exclusions consistently to covered and total counts and every
  export, including raw function records and artifacts from failed runs. For a
  no-decrease gate, compare each metric separately against the actual PR base
  under the same profile; an improvement target is not a replacement gate.
- Run focused tests, the affected suite and required lint/format checks; preserve
  production lint rules. Report revision, tools/profile, counts and gaps. Distinguish
  local/hosted results, explain observed variance and identify pending reviews.
- Keep developer guides durable: setup, commands, contracts, fixture design and
  interpretation. Put issue links, targets, counters, commit checkpoints and
  progress/history in the tracker or PR, not in development documentation.
  Preserve runnable commands, working directories and fixture prerequisites when
  removing progress prose. Follow the repository's issue/documentation format.
- Update relevant documentation. Commit, push or update trackers only within the
  user's authorization; preserve existing issue/PR/documentation links. Separate
  unrelated CI failures from failures caused by this change.

## Review follow-through

- Review all requested reviewers' inline threads and review summaries, including
  collapsed findings. Verify claims against the owning PR's current source and
  pinned dependencies; a fix only in a later stacked PR does not settle the earlier
  one. A bot's “addressed” marker is not evidence that the behavior is correct.
- When authorized to respond on GitHub, reply in each finding's actual thread with
  the fix and commit, or a concrete reason no change is needed, plus relevant
  validation. Reply to summary-only findings on the PR. A local audit is not a
  posted response; resolve only settled threads after replying.
- Read back replies and resolution state. After an uncertain write, check for an
  existing reply before retrying to avoid duplicates. Refresh reviews and hosted
  checks at the final head; report pending findings/checks honestly.

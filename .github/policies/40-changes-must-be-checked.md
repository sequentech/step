<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# New code must be covered by this repository's checks

A check that does not run is worse than no check, because it looks like
protection. The same is true of a test nobody executes and a linter nobody
invokes: they read as evidence of care while providing none.

This repository runs tests, linting, formatting and licence checks in CI. Code
added here has to be reachable by them.

## The rule

**Code added to this repository must be executed, linted and licensed by an
existing CI job — or the change must add the job that does it.**

## What breaks this rule

Report a violation when a change:

- **Adds tests that nothing runs.** A new test file under a path that no
  workflow's `paths` filter matches, or in a language or framework no job
  invokes, is dead weight. Trace the new file to a job that would actually
  execute it; if you cannot, that is the finding.
- **Introduces a language or toolchain with no corresponding check.** Adding the
  first files of a new language, or a new top-level directory of code, without a
  workflow that builds, tests and lints them.
- **Adds runnable code with no test at all**, where the surrounding code is
  tested. A new module with real branching logic and no accompanying test, in a
  directory where every other module has one.
- **Adds a file with no SPDX header** and no covering entry in `REUSE.toml` —
  the licence check will fail, and saying so here is faster than a red build.
- **Weakens a check's reach.** Narrowing a workflow's `paths`, widening a
  `paths-ignore`, adding an `exclude` to a linter's configuration, or marking a
  test skipped, without saying why in the pull request description.

Report at **`warning`** by default: these are gaps rather than breaches, and the
fix is usually small. Report at **`blocker`** when the change weakens the reach
of an existing check, because that silently reduces coverage for everything
afterwards, not just for the code in this pull request.

## What does not break it

- Configuration, documentation, fixtures and static assets — files with no
  behaviour to test.
- Generated code, where the generator is what is tested.
- A change that adds both the code and its check in the same pull request. That
  is the shape this policy is asking for.
- Trivial code with nothing meaningful to assert: a constant, a re-export, a
  one-line accessor.
- A deliberately narrow tool configuration that is explained in a comment or in
  the pull request description. Scoping a new linter to the directory it was
  introduced for is reasonable; doing it silently is what this catches.

## What to do instead

Add the code and the check that covers it in the same pull request. If a new
kind of code needs a new workflow, that workflow is part of the change, not a
follow-up — a follow-up that never happens is how a repository ends up with
tests that have not run in a year.

When a check genuinely has to be narrowed or skipped, say so in the pull request
description and in a comment next to the change, so the next reader knows it was
a decision rather than an oversight.

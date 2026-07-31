<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# New code and documentation must be covered by this repository's checks

A check that does not run is worse than no check, because it looks like
protection. The same is true of a test nobody executes, a linter nobody invokes,
a package nobody builds, a scan that silently skipped, and a documentation page
nothing renders: they read as evidence of care while providing none.

This repository runs tests, linting, builds, static analysis and licence checks
in CI. Anything added here has to be reachable by them.

## The rule

**Code and documentation added to this repository must be built, tested, linted,
statically analysed and licensed by an existing CI job — or the change must add
the job that covers them.**

## Where the checks reach today

The reviewer needs this to tell "covered" from "looks covered". Each check has a
reach, and the gap between that reach and a new file is where coverage is lost.

| Check | What it covers | Where it stops |
|---|---|---|
| `sonarqube.yml` | Static analysis for three projects: `step-backend-rust`, `step-frontend`, `step-keycloak-extensions` | Triggers on `packages/**` only, and its language filters match only `.rs`, `Cargo.toml`, `.java`, `pom.xml`, `.js`, `.ts`, `.jsx`, `.tsx`, `package.json` |
| `documentation.yml` | Docusaurus site and the GraphQL API reference | Triggers on `docs/docusaurus/**`, `docs/api/graphql/**` and `packages/graphql.schema.json` only |
| `pr_build.yml` → `reusable_build_push.yml` | Docker images for deployable services | Only the images named in the build definition |
| `tests.yml`, `java_test.yml` | Rust, frontend, Windmill and Java test suites | Only the packages named in their matrices |
| `lint_prettify.yml` | Frontend and Hasura formatting | Not Python, not Go, not anything else |
| `license_reuse.yml` | Licence headers, whole repository | Nothing — this one has no blind spot |

The pattern worth internalising: **most of these are path- or matrix-scoped, so
new code lands outside them by default rather than inside them.**

## What breaks this rule

Report a violation when a change:

### Tests

- **Adds tests that nothing runs.** A new test file under a path no workflow's
  `paths` filter matches, or in a framework no job invokes. Trace the file to a
  job that would execute it; if you cannot, that is the finding.
- **Adds runnable code with no test at all**, where the surrounding code is
  tested — a new module with real branching logic in a directory where every
  other module has one.

### Builds

- **Adds a deployable service, binary or image that no build job produces.** A
  new service directory with a Dockerfile that is not in the build definition is
  never built on a pull request, so it breaks in the release build instead.
- **Adds a package to a workspace without adding it to the jobs that build and
  test that workspace**, where those jobs enumerate packages rather than
  discovering them.

### Static analysis

- **Adds code that SonarQube will not see.** Two distinct ways this happens, and
  both are silent — the workflow still reports green:
  - **Outside the trigger.** Source added outside `packages/**` does not start
    the scan workflow at all.
  - **Outside the language filters.** Source added *inside* `packages/**` in a
    language the `detect-changes` filters do not list — Python, Go, Kotlin, C#,
    Ruby — matches nothing, so every scan job is skipped.
- **Widens `sonar.exclusions`,** or adds a path to an existing exclusion list,
  removing code from analysis.
- **Adds a new project to the repository without a `sonar.projectKey`** covering
  it, where the existing three do not.

### Documentation

- **Adds documentation the site does not build.** Pages outside
  `docs/docusaurus/**` are not rendered by `documentation.yml`, so broken links,
  bad MDX and invalid front matter are never caught.
- **Adds a Docusaurus page that no sidebar references.** It builds, and then
  nobody can reach it. Check `sidebars.js` when a page is added.
- **Changes the GraphQL schema without the API reference following.** The
  reference is generated from `packages/graphql.schema.json`; a schema change
  that leaves it stale ships documentation that describes an API that no longer
  exists.
- **Documents behaviour that the change does not implement**, or leaves
  documentation describing behaviour the change removes.

### Licensing

- **Adds a file with no SPDX header** and no covering entry in `REUSE.toml`. The
  licence check runs on every file in the repository and will fail; saying so
  here is faster than a red build.

### Weakening a check's reach

- Narrowing a workflow's `paths`, widening a `paths-ignore`, adding an
  `exclude` to a linter or scanner configuration, removing a package from a test
  matrix, or marking a test skipped — without saying why in the pull request
  description.

## Severity

Report at **`warning`** by default: these are gaps rather than breaches, and the
fix is usually small.

Report at **`blocker`** when a change **weakens the reach of an existing check**.
That silently reduces coverage for everything afterwards, not just for the code
in this pull request, and it is the failure mode that compounds.

## What does not break it

- Configuration, fixtures and static assets — files with no behaviour to test.
- Generated code, where the generator is what is tested.
- A change that adds both the code and its check in the same pull request. That
  is the shape this policy is asking for.
- Trivial code with nothing meaningful to assert: a constant, a re-export, a
  one-line accessor.
- Documentation changes to existing pages already inside the built site.
- A deliberately narrow tool configuration that is explained in a comment or in
  the pull request description. Scoping a new linter to the directory it was
  introduced for is reasonable; doing it silently is what this catches.
- Release notes and changelog entries, which are records rather than references.

## What to do instead

Add the code and the check that covers it in the same pull request. If a new
kind of code needs a new workflow — or a new entry in an existing matrix, filter
or exclusion list — that is part of the change, not a follow-up. A follow-up
that never happens is how a repository ends up with a test suite that has not
run in a year and a scanner that quietly stopped seeing half the codebase.

For documentation, put it where the site builds it and reference it from a
sidebar, so it is rendered, checked and reachable.

When a check genuinely has to be narrowed or skipped, say so in the pull request
description and in a comment next to the change, so the next reader knows it was
a decision rather than an oversight.

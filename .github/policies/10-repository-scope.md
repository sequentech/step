<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Repository scope and self-containment

> **Label:** `policy:architecture`
> **Reviewers:** `@sequentech/Architects`
> **If breached:** an Architect must approve before this can merge.

This repository is the public, open-source home of the Sequent voting platform.
It holds the platform's own source code, its tests, and its documentation.

## The rule

**Everything added here must be publicly usable and publicly buildable.** A
person who clones this repository and nothing else must be able to read,
understand, build and test what they find, using only public resources.

## What breaks this rule

Report a violation when a change:

- **Adds a dependency on a resource that is not public** — a workflow that
  checks out a repository a member of the public cannot clone, an action
  referenced from a private repository, a submodule pointing somewhere
  unreachable, a build step that pulls from a registry requiring internal
  credentials, or a script whose success depends on an internal-only host.
- **Names internal infrastructure that the public has no way to reach** — a
  hostname, cluster, bucket, database or queue that only exists inside the
  organisation's network, where the reference serves no purpose to an outside
  reader.
- **Documents an internal-only procedure as if it were a public one** — runbooks
  or setup guides whose steps cannot be followed without internal access, placed
  in documentation aimed at outside contributors.

## What does not break it

- Existing references that the change merely moves, reformats or leaves alone.
  Only new or newly-worsened dependencies are in scope.
- Public third-party dependencies: package registries, public container
  registries, published GitHub Actions, public APIs.
- Example or placeholder values in documentation and tests, where they are
  clearly illustrative and not required to reach anything real.
- Deployment metadata that is genuinely public, such as the project's own
  published documentation site.

## What to do instead

Keep the public-facing artefact self-contained. Where a build or deployment step
genuinely needs something internal, it belongs in the repository that owns that
concern, invoked from there — not embedded here.

If a change truly requires a new external dependency for this repository, say so
in the pull request description and have a maintainer confirm it is public and
intended, rather than introducing it silently.

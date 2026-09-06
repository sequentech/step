<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Licensing and attribution

> **Label:** `policy:coverage`
> **Reviewers:** `@sequentech/DevOps`
> **If breached:** the `reuse` check will fail anyway — this is the
> faster feedback.

This repository is distributed under AGPL-3.0-only and follows the
[REUSE](https://reuse.software/) specification: every file carries its copyright
and licence, either inline or through an entry in `REUSE.toml`.

## The rule

**Every new file must be licensed, and third-party code must keep its own
licence and attribution.**

## What breaks this rule

Report a violation when a change:

- **Adds a source file with no SPDX header** and no covering entry in
  `REUSE.toml`. Headers look like this, using the comment syntax of the language:

  ```text
  # SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
  #
  # SPDX-License-Identifier: AGPL-3.0-only
  ```

- **Vendors third-party code without preserving its licence** — a copied file
  with its original header stripped, a bundled library with no licence text, or
  a vendored dependency relabelled under this project's licence.
- **Introduces a licence incompatible with AGPL-3.0-only** — most visibly a
  proprietary or no-licence dependency added to a manifest.

## What does not break it

- Files matched by an existing `REUSE.toml` annotation; that is the intended way
  to cover generated files, lockfiles and asset directories.
- Formats with no comment syntax, such as JSON, which are covered through
  `REUSE.toml` rather than inline.
- Third-party files that correctly keep their original licence, including a
  permissive licence different from this project's, with a matching `REUSE.toml`
  entry recording it.

## What to do instead

Add the SPDX header when you create the file. For a file that cannot carry one,
add an annotation to `REUSE.toml` covering it. When vendoring third-party code,
keep its licence text and record the real copyright holder — never replace it
with this project's header.

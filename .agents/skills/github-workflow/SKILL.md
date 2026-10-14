---
name: github-workflow
description: Name branches, maintain PR stacks, write PR and issue bodies, and reply to and resolve review threads for sequentech repositories. Use when creating a branch, opening or updating a PR, editing a tracking issue or handling review feedback.
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# GitHub workflow

The [contributing guide](../../../docs/docusaurus/docs/07-developers/00-contributing.md)
covers commits, the CLA and release branches.

## Branches and stacks

- Branch from the target: `feat/meta-<issue>-<description>/<target>` or
  `fix/meta-<issue>/release/X.Y`, using the tracking issue in
  `sequentech/meta`. Create the issue first if none exists.
- Conventional commit subjects (`feat:`, `fix:`, `docs:`, ...).
- Never force-push or rebase a shared branch. Update with normal merges.
- Base each stacked PR on its immediate predecessor; propagate shared fixes
  down the stack by merging. Keep existing PRs and their discussions.
- Merging, closing issues and replacing branches need the owner's approval.

## PR bodies

Start with `Parent issue: https://github.com/sequentech/meta/issues/<n>`. Then
keep it short: the problem, the resulting behaviour, the validation and the
documentation. No history or "supersedes" narratives. Release-branch PRs do
not edit release notes.

## Reviews

For every finding, including summary-only and collapsed ones: verify it against
the current source; fix it or decide no change; reply with the outcome, commit
and validation in the finding's thread, or on the PR for summary-only findings;
resolve only settled threads; read the result back. Check for an existing
reply before retrying a failed post.

## Issues

Keep the template and relationships. Tracking issues use, in order:
`### Suggestion`, `### Affected Versions`, `### Acceptance criteria` (verifiable
conditions), the PR list (one full URL per bullet), `### Verification` (a
before/after table and **Work performed** bullets) and `### Documentation`
(`- **Topic**: [Docusaurus](<rendered>) · [GitHub Markdown](<source>)`). Record
current state, not edit history.

## Links

Use full URLs across repositories: a bare `#123` resolves in the current one.

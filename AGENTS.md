<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Repository guidance

For unit tests and coverage work, use
[implement-unit-tests](.agents/skills/implement-unit-tests/SKILL.md), together with
the package's testing guide and configured tool versions. Invoke it with
`$implement-unit-tests add tests for packages/<package>`.

## Issues

Preserve the issue template and existing relationships when editing a ticket.
For programme trackers, retain this section order:

- `### Suggestion`: concise objective, Parent issue and relevant related issues.
- `### Affected Versions`: affected versions or branches.
- `### Acceptance criteria`: verifiable completion conditions; distinguish an
  improvement target from the actual CI gate.
- `### Main PRs`: one plain, full PR URL per bullet; preserve every active PR link.
- `## stable PRs`, when present: matching PRs for stable target versions, in stack
  order, with the target branch identified; preserve this section too.
- `### Verification`: a compact before/after table and **Work performed** bullets,
  followed by concrete remaining gaps or validation limits where needed.
- `### Documentation`: topic bullets with paired rendered and source links.

Keep the current state, not a running history of ticket consolidation or previous
edits. Completed-work bullets belong under Verification, not Acceptance criteria.
Label measurement profiles and incomparable baselines; do not conflate LLVM
regions with branches or local checks with hosted CI.

Use full GitHub URLs for cross-repository issues and PRs; a bare `#123` resolves
in the current repository. Preserve parent/related links and documentation links
when shortening a body. Read back remote edits to verify formatting and links.

## Pull requests and stacks

Start PR bodies with `Parent issue: <full issue URL>`. Keep the remainder short:
concrete problem, resulting behavior, relevant validation and documentation.
Do not add `Supersedes`/`Superseeds` sections or historical replacement narratives.

Use the correct parent ticket in new branch names, following the existing branch
convention. Keep each stacked PR based on its immediate predecessor. Preserve
existing PR identities and review discussions; formatting edits do not require
recreating PRs. Branch replacement, ticket closure and publication require the
user's authorization. For an authorized stack update, propagate shared fixes
through the affected descendants without force-pushing or merging PRs.

Address review feedback in GitHub as well as in code when that work is authorized:
reply with the outcome and evidence in the relevant thread, including a reasoned
no-change outcome. Resolve only settled findings, then verify the posted result.

## Developer documentation

Place developer guides in the appropriate existing section under
`docs/docusaurus/docs/07-developers/`; package READMEs may forward to the canonical
guide. Keep reusable setup, commands, prerequisites, contracts and interpretation
there. Programme targets, coverage snapshots, issue IDs and progress history
belong in issues or PRs. Do not remove useful commands while removing status prose.

End an issue's Documentation section with one bullet per topic in this form:

```markdown
- **Topic**: [Docusaurus](<rendered guide URL>) · [GitHub Markdown](<source URL>)
```

Use real corresponding URLs, preserving both links. For repository-only material
such as skills, retain the source link without inventing a rendered guide. Check
moved links and navigation, and build Docusaurus when guide changes warrant it.

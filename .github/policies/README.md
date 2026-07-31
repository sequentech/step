<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Policies for this repository

Every Markdown file in this directory is a policy. The
[policy review workflow](../workflows/policy-check.yml) loads all of them and
checks each pull request against them, then comments with what it finds.

This file is documentation, not a rule — the loader skips `README.md`.

## Adding a policy

Create a Markdown file here. There is no workflow change and no code change: the
next pull request picks it up.

```
NN-short-name.md
```

The numeric prefix orders the files; the first `# Heading` becomes the policy's
title in the review comment. Give each rule:

- **What the rule is**, stated plainly.
- **What breaks it**, ideally with a concrete example.
- **What to do instead**, so a report is actionable.
- **Any exceptions**, listed explicitly. The reviewer honours documented
  exceptions and reports anything else.

Write for a competent engineer who has not read the rest of this directory.
Vague rules produce vague findings, and a check people learn to ignore is worse
than no check.

## What belongs here

This repository is public, and so is everything in this directory. Keep policies
here general: the structure of this repository, its licensing, its hygiene.

A rule that would only make sense alongside knowledge of non-public systems does
not belong here. Put it in the repository it applies to — every repository has
its own policies directory, and the shared workflow reads whichever one belongs
to the repository being reviewed. See
[`../policy-review/README.md`](../policy-review/README.md).

## Changing a policy

Policies are read from the **target branch**, never from the pull request's own
head. A pull request that edits a policy is still judged against the rule as it
stands on the branch it is merging into, and the edit is called out in the
review comment so a human considers it deliberately. The new wording takes
effect once it is merged.

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Changes to the policy system itself

> **Label:** `policy:governance`
> **Reviewers:** `@sequentech/Architects`
> **If breached:** nothing here is forbidden — but it cannot merge without an
> Architect, and it is announced in Slack.

Every other policy asks *"is this change allowed?"*. This one asks a different
question: **"does this change alter the thing that decides what is allowed?"**

Deleting a policy file is one line. Widening a path filter so a check stops
running is one line. Changing which team a violation routes to is one word. All
three arrive in diffs that look like routine maintenance, and none of them
breaches any other policy — so nothing else would report them.

This is rarely malice. It is far more likely to be someone tidying a workflow or
trimming a rule they believe is redundant, which is exactly why it has to be
automatic rather than a habit.

## The rule

**The policy system may be changed — it has to be maintainable — but never
quietly.**

These files are the system:

```
.coderabbit.yaml                     labels, reviewer routing, guideline wiring
.github/policies/**                  the rules themselves
.github/CODEOWNERS                   who must approve a change to the above
.github/workflows/policy-alert.yml   the Slack alert
```

The architecture document —
`docs/docusaurus/docs/engineering/architecture.md` — is also Architect-owned in
`CODEOWNERS`, but it is not part of this policy. It is the baseline the review
compares against, not part of the machinery that decides; a change to it is
[`60-architectural-changes.md`](60-architectural-changes.md)'s business.

## What to do when this label appears

It is not a finding to be argued away. It is a request for a different kind of
review than usual:

- **Say what the change does to the system's ability to do its job.** Does it
  narrow a rule, relax a trigger, reroute a reviewer, change what is reported —
  or is it routine maintenance that leaves enforcement intact?
- **Do not report it as a violation on its own.** Maintaining the system is
  allowed. Report a violation only if the change *also* breaches another policy.
- **Check the reasoning is recorded.** A deliberate narrowing is fine; a silent
  one is what this exists to catch. It belongs in the pull request description.

## How it is enforced

Three independent mechanisms, and each covers a gap in the others:

| Mechanism | What it does |
|---|---|
| `.github/CODEOWNERS` | An Architect must approve. This is the one that actually blocks the merge. |
| This label | Pulls an Architect in automatically and announces the change in Slack. |
| The review itself | Describes what the change means for enforcement. |

CodeRabbit reads `.coderabbit.yaml` from the pull request's own branch, so a
change to that file affects the very review judging it. CODEOWNERS is what makes
that safe, and it is the reason this policy leans on a GitHub control rather than
on the review.

## What does not break this rule

- Adding a policy, or making an existing one stricter or clearer.
- Fixing a typo, a broken link, or formatting in a policy file.
- Changes elsewhere in `.github/` that are not part of the system listed above —
  an unrelated workflow is an ordinary change.

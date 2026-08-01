<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Policies for this repository

Every Markdown file here is a policy. [CodeRabbit](https://coderabbit.ai) reads
them as review guidelines and checks each pull request against them.

This file is documentation, not a rule.

## How a policy becomes an action

Each policy owns one label, and the label decides what happens:

| Policy | Label | Reviewers | Slack |
|---|---|---|---|
| `10-repository-scope` | `policy:repo-scope` | Architects | 🔴 |
| `20-secrets-and-environment-config` | `policy:secrets` | Architects | 🔴 |
| `30-licensing`, `40-changes-must-be-checked` | `policy:coverage` | DevOps | 🔵 |
| `50-governance` | `policy:governance` | Architects | 🟠 |
| `60-architectural-changes` | `policy:architecture-change` | Architects | 🏗️ |

`policy:repo-scope` and `policy:architecture-change` ask different questions
and are not interchangeable. The first asks whether code is in the **right
repository**; the second asks whether the change alters **what the system is**.
A change can trip both, one, or neither.

CodeRabbit applies the label and requests the review; both are ordinary GitHub
events, which is how [`policy-alert.yml`](../workflows/policy-alert.yml) can post
a different Slack message per policy without any integration of its own.

Severity is expressed by what a label routes to, not by a severity word. A policy
that pulls in an Architect and pages Slack is more serious than one that only
labels the pull request — and saying it that way is honest, because it describes
what will actually happen.

## Adding a policy

Three declarative edits, no code:

1. **Write the Markdown file here.** `NN-short-name.md`. Open with the label
   header block the other files use, so the policy states its own routing.
2. **Add its label** to `labeling_instructions` in
   [`.coderabbit.yaml`](../../.coderabbit.yaml), and its reviewers to
   `suggested_reviewers_instructions` if it should pull someone in.
3. **Add a case** to [`policy-alert.yml`](../workflows/policy-alert.yml) if it
   should reach Slack. Leave it out and the policy is labelled but silent.

Then create the label in the repository — CodeRabbit applies labels, it does not
create them.

Write four things in every policy: **what the rule is**, **what breaks it** with
concrete examples, **what does not** break it, and **what to do instead**. The
"what does not" section matters as much as the rule: an unwritten exception
becomes a false positive, and false positives are what teach a team to ignore a
check.

## What belongs here

This repository is public, and so is everything in this directory. Keep policies
general — the structure of this repository, its licensing, its hygiene.

A rule that would only make sense alongside knowledge of non-public systems does
not belong here. Put it in the repository it applies to; every repository has its
own policies directory and its own `.coderabbit.yaml`, and nothing is shared
between them.

## Changing a policy

CodeRabbit reads configuration from the pull request's own branch, so a change
here affects the review judging it. That is why
[`CODEOWNERS`](../CODEOWNERS) requires an Architect's approval for this
directory, and why [`50-governance.md`](50-governance.md) exists.

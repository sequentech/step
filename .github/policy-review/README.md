<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Policy review

An automated reviewer that checks each pull request against a set of written
policies and reports what it finds: a comment on the pull request, a formal
"changes requested" review once the pull request is ready, and a Slack alert
when something real is wrong.

It is a policy *engine*, not a policy. It ships no rules of its own — every rule
it enforces is a Markdown file in the repository being reviewed.

## How it fits together

```
This repository
└── .github/workflows/policy-review.yml     the one reusable workflow
    └── .github/policy-review/              the engine it runs

Each repository that uses it
├── .github/workflows/policy-check.yml      thin caller: triggers + policies_path
└── .github/policies/                       that repository's own rules
```

One implementation, many callers. A repository opts in with a short caller
workflow and a directory of Markdown files.

## Why the reusable workflow lives here

GitHub lets a private repository call a reusable workflow from a public one. The
reverse needs the private repository to grant access, which would make the
public repository depend on something a member of the public cannot see.

Putting the shared implementation in the public repository means every
repository can call it, while this repository needs no knowledge of — and holds
no reference to — any of its callers. The dependency runs one way only.

That property is not left to good intentions.
[`tests/test_public_safety.py`](tests/test_public_safety.py) asserts that these
files reference no repository other than this one, and it runs on every pull
request that touches the engine. It is written without naming what it is
guarding against, so the guard itself discloses nothing.

## How `policies_path` works

The workflow reads policies from a path **inside the repository being
reviewed**, not from here:

```yaml
with:
  policies_path: .github/policies   # the default
```

Every `.md`, `.markdown` or `.txt` file below that path is loaded, except
`README.md`. The first `# Heading` becomes the policy's title; the filename stem
becomes its id, which is what findings cite.

Because each repository carries its own directory, each publishes only the rules
appropriate to it. A rule that would disclose something by existing lives in the
repository it applies to, never here.

**Policies are read from the target branch.** A pull request cannot weaken the
rules it is about to be judged against by editing them in the same change; the
edit is reported in the review comment instead, so a human considers it
deliberately. A new rule takes effect once merged.

## Using it in a repository

Add `.github/workflows/policy-check.yml`:

```yaml
name: Policy check

on:
  pull_request:
    types: [opened, synchronize, reopened, ready_for_review]
  workflow_dispatch:
    inputs:
      pr_number:
        description: Pull request number to review.
        type: number
        required: true

permissions:
  contents: read
  pull-requests: write

jobs:
  policy-review:
    uses: sequentech/step/.github/workflows/policy-review.yml@main
    with:
      policies_path: .github/policies
      repo_type: One or two sentences on what this repository is for.
      pr_number: ${{ inputs.pr_number || 0 }}
    secrets:
      anthropic_api_key: ${{ secrets.ANTHROPIC_API_KEY }}
      slack_bot_token: ${{ secrets.SLACK_BOT_TOKEN }}
```

Then write at least one policy under `policies_path`. With no policies the check
passes and says so — it never fails a repository for not having opted in yet.

Keep callers thin. Triggers, path filters and the values above are the caller's
business; everything else belongs in the shared workflow.

> [!WARNING]
> **A `paths-ignore` filter must never be able to match a guarded path.**
> Policy files are Markdown, so a blanket `paths-ignore: "**/*.md"` skips the
> check entirely for a pull request that guts a policy — the one change that
> most needs reviewing. GitHub does not support negation in `paths-ignore`, so
> keep any filter narrow and specific (`docs/**`) rather than broad. If in
> doubt, use no filter: a review costs one model call, and a hole in the guard
> costs the whole check.

### Inputs

| Input | Default | What it does |
|---|---|---|
| `policies_path` | `.github/policies` | Where to read policies from, in the calling repository |
| `repo_type` | `""` | Free-form description of the repository's role, given to the reviewer as context |
| `model` | `claude-opus-5` | Model used for the review |
| `effort` | `high` | `low`, `medium`, `high`, `xhigh` or `max` |
| `fail_on_severity` | `blocker` | Lowest severity that fails the check |
| `guarded_paths` | built-in list | Files belonging to the policy review system itself — see [Self-protection](#self-protection) |
| `max_diff_bytes` | `400000` | Larger diffs are truncated, and the comment says so |
| `slack_channel` | `""` | Channel ID for alerts; empty disables Slack |
| `slack_message_prompt` | built-in | House style for the generated alert |
| `pr_number` | `0` | Only needed for `workflow_dispatch` |
| `engine_ref` | `main` | Ref to take the engine from |
| `python_version` | `3.12` | Python used to run the engine |
| `runs_on` | `ubuntu-24.04` | Runner label |

`repo_type` is deliberately a free string rather than a fixed set of choices. An
enum would mean this public workflow carried a list of the repositories that
call it.

Pin `engine_ref` to whatever ref the `uses:` line points at, so the workflow
definition and the engine it runs always match.

### Secrets

| Secret | Required | Purpose |
|---|---|---|
| `ANTHROPIC_API_KEY` | yes | Model access. Without it the review **skips** rather than fails — which is what happens for a pull request from a fork, since forks receive no secrets. |
| `SLACK_BOT_TOKEN` | no | Posting alerts. Needs `chat:write`, and the bot must be in the channel. |

`GITHUB_TOKEN` is the built-in workflow token. The workflow requests
`contents: read` and `pull-requests: write` and nothing else.

## Adding a policy

Add a Markdown file to the right repository's policies directory. No workflow
change, no code change, no release.

Good policies say four things: **what the rule is**, **what breaks it** (with an
example), **what to do instead**, and **what the exceptions are**. The reviewer
honours documented exceptions and reports anything else, so an unwritten
exception becomes a false positive — and false positives are what teach a team
to ignore a check.

Findings cite policies by id, so filenames are part of the interface. Renaming
one changes what future comments cite.

## Self-protection

Every policy asks "is this change allowed?". One check asks a different
question: **"does this change alter the thing that decides what is allowed?"**

A pull request that edits the policies, the caller workflow, the reusable
workflow or the engine can weaken or disable enforcement for every pull request
that follows — quietly, in a diff that looks like routine maintenance. Deleting
a policy file is the clearest example, and it is a one-line change.

So when a pull request touches any of these:

```
.github/policies/**                        (whatever policies_path is set to)
.github/workflows/policy-check.yml
.github/workflows/policy-review.yml
.github/workflows/policy-review-tests.yml
.github/policy-review/**
```

…three things happen regardless of the verdict:

1. **A warning banner is placed above the verdict** in the pull request comment,
   naming each touched file and what it is. It sits above deliberately: a change
   to the machinery outranks the verdict that machinery just produced.
2. **A Slack alert is sent**, even when the review passes with no violations,
   and even when the review could not run at all. A failed review of a change to
   the machinery is the worst case to stay silent about.
3. **The reviewer is told**, and asked to describe in its summary what the
   change does to the system's ability to do its job.

It is a **notice, not a block**. The system has to be maintainable, so this is
not treated as a violation on its own — only as something that must never pass
unremarked. A real breach is still reported as a violation in the normal way.

`guarded_paths` overrides the list above (newline- or comma-separated).
`policies_path` is always guarded whether or not it appears there.

## What it does with what it finds

- **Always** — updates a single comment on the pull request. Repeated pushes
  edit that comment rather than adding more.
- **Blocking violations, once the pull request is marked ready for review** —
  submits a formal "changes requested" review. If GitHub declines (nobody may
  review their own pull request), the comment stands on its own.
- **Any violation, or any change to the system itself, if Slack is configured** —
  posts one alert to the channel, with the text generated from
  `slack_message_prompt`.
- **Clean** — a short "all policies passed" note.
- **Could not run** — an explicit "this is not a pass" comment. A review that
  failed must never look like one that succeeded.

Exit codes: `0` pass or skipped, `1` blocking violations, `2` engine error.

## Design notes

**Untrusted input.** A pull request author controls the title, the description
and every line of the diff. All of it is wrapped in `<untrusted_*>` tags, any
closing tag inside the payload is defanged so content cannot escape its own
fence, and the system prompt states that the fenced region is evidence rather
than instruction. Content that tries to steer the review is itself reported.

**No shell interpolation.** Every value reaches the engine through `env:`.
Nothing attacker-controlled is interpolated into a `run:` block, so a crafted
branch name or title cannot become a command.

**The violations list is the source of truth.** A response that lists breaches
while declaring itself a pass still blocks, and a severity the engine does not
recognise is escalated rather than ignored. A malformed response fails the
check; it never degrades into a quiet pass.

**One dependency.** The engine uses the official `anthropic` SDK and reaches
GitHub and Slack through the standard library, keeping the supply chain of a
credential-handling workflow small.

## Working on the engine

```bash
cd .github/policy-review
python -m venv .venv && . .venv/bin/activate
pip install -r requirements.txt
python -m unittest discover -s tests -t . -v
```

The tests stub every outbound call, so the suite needs no credentials and no
network.

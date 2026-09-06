<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Architectural changes

> **Label:** `policy:architecture-change`
> **Reviewers:** `@sequentech/Architects`
> **If breached:** an Architect must approve, and the change must update the
> [architecture document](../../docs/docusaurus/docs/engineering/architecture.md).

Some changes decide what the system *is*. Which identity provider issues our
tokens, which database holds the votes, which queue carries the work, whether
the ballot is encrypted in the browser or on the server — these are not
implementation details that can be revisited cheaply next quarter. They are
load-bearing, and everything built afterwards assumes them.

They also do not announce themselves. Replacing an authentication library is a
few hundred lines. Adding a datastore is one dependency and one connection
string. Splitting a service in two is a directory move. None of these looks like
a decision in a diff, and none breaches any other policy — so without this one,
nothing reports them.

This is rarely anyone acting out of turn. It is far more often someone solving
the problem in front of them well, without knowing that the shape of the
solution was settled two years ago for reasons that are not visible from where
they are standing.

## The rule

**An architectural change is allowed. It must pull in an Architect, and it must
update the [architecture document](../../docs/docusaurus/docs/engineering/architecture.md) in the same pull request.**

Two obligations, and the second is the one people forget. A decision that is
made but not recorded is a decision the next person will unknowingly contradict.

## What counts as architectural

Report this when a change does any of the following.

### Replaces or adds a foundational technology

- **Identity and access** — the identity provider, the token format, how
  sessions are established, how permissions are represented or carried.
- **The API layer** — the engine serving the API, the protocol it speaks, or how
  requests are authorised.
- **A datastore** — adding one, removing one, or changing which one holds a
  given kind of data. The tamper-evident log is the most sensitive of these: its
  records are append-only and effectively permanent.
- **The message broker or task protocol** — what carries asynchronous work.
- **Object storage** — the storage API a component talks to.
- **A cryptographic primitive, curve, hash or protocol.** Always. There is no
  such thing as a routine change here.
- **A language or runtime** — introducing one the repository does not already
  build, or removing one it does.

### Changes how components relate

- **Adding, removing, splitting or merging a service.**
- **Moving a responsibility between components** — logic that ran synchronously
  becoming a background task, or the reverse.
- **Changing an interface between components** — the shape of a task message,
  the contract of an API action, the serialisation shared between the backend
  and code compiled for the browser.
- **Changing where code runs** — moving cryptographic work from the client to
  the server, or the reverse. This changes the trust model, which is the whole
  point of the system.

### Changes a pinned or forked dependency

Some dependencies are pinned or forked for reasons that are not obvious from the
manifest. Changing which fork is used, or unpinning something the repository
deliberately pinned, is architectural regardless of the version delta.

### Contradicts the architecture document

If the change makes any statement in the
[architecture document](../../docs/docusaurus/docs/engineering/architecture.md) untrue and does not update it, **that is the finding** — even when the change is
otherwise fine and well reviewed. Say which section is now wrong.

This half of the rule matters more than it looks. The first half is caught by
the review; the second is what stops the document decaying into a description of
a system that no longer exists, at which point everyone stops reading it and the
first half stops working too.

## What does not break this rule

Being explicit here matters — a policy that fires on ordinary work is a policy
people learn to dismiss.

- **Ordinary feature work** inside the existing structure: new endpoints,
  new components, new tables, new UI, new tests.
- **Dependency updates** that keep the same technology — a minor or patch bump,
  or a major bump of a library that is not one of the decisions recorded in the
  architecture document.
- **Refactoring, renaming and reorganising** that leaves the component
  boundaries and the interfaces between them where they were.
- **Performance work** that does not change what a component is or what it talks
  to.
- **Adding a document, a diagram or a comment** describing the architecture as
  it already is. Improving the record is the opposite of what this catches.
- **Reverting** a change that was itself flagged under this policy, provided the
  architecture document is reverted with it.

## What to do

When the label appears:

1. **Say what the decision is**, in the pull request description — not what the
   code does, but what is being chosen and what it replaces.
2. **Say why**, including what was considered and rejected. The next person to
   ask this question deserves the reasoning, not just the outcome.
3. **Update the [architecture document](../../docs/docusaurus/docs/engineering/architecture.md)** — the affected row, the diagram, and anything elsewhere in it that the change contradicts.
4. **Link the `meta` issue** where the decision was taken.

If you conclude the change is not architectural, say so and why. A short
explanation resolves a mislabel; silence leaves the reviewer guessing.

## How it is enforced

| Mechanism | What it does |
|---|---|
| This label | Pulls in an Architect automatically and announces the change in Slack. |
| `.github/CODEOWNERS` | An Architect must approve a change to the architecture document. This is what stops the record being quietly rewritten to match the code. |
| The review itself | Names which part of the architecture document the change contradicts. |

Note what CODEOWNERS covers and what it does not: it gates *edits to the
document*, not *changes that should have edited it*. Only the review catches the
second, which is why the omission is a reportable finding in its own right.

## Not the same as `policy:repo-scope`

They are easy to confuse and they catch different things.

| | Question it asks |
|---|---|
| `policy:repo-scope` ([10-repository-scope](10-repository-scope.md)) | Is this code in the **right repository**? |
| `policy:architecture-change` (this policy) | Does this change **what the system is**? |

A change can trip both, one, or neither. Moving a service to a different
repository is the first. Replacing the identity provider in place is the second.

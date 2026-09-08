---
id: policy-review-configuration
title: Configuring the policy review
sidebar_position: 2
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Configuring the policy review

This repository checks every pull request against the policies in
[`.github/policies/`](https://github.com/sequentech/step/tree/main/.github/policies)
using [CodeRabbit](https://coderabbit.ai), labels what it finds, and can announce
it in Slack.

The review itself needs no configuration — it runs from files in the repository.
This page covers the parts that do: the optional Slack notification, and the
repository settings that turn the review from advice into a merge gate.

It is written to be followed start to finish. If you only came for one thing,
**[creating the Slack webhook](#2-create-a-slack-webhook)** is section 2.

:::info Who can do this
Creating a Slack app needs permission to install apps in your workspace. Adding
a repository secret needs **admin** on the repository.
:::

---

## 1. What there is to configure

| # | What | Required | What happens without it |
|---|---|---|---|
| 1 | `POLICY_SLACK_WEBHOOK_URL` | No | Falls back to `SLACK_WEBHOOK_URL` |
| 2 | `SLACK_WEBHOOK_URL` | No | No Slack message; the check still passes |
| 3 | The CodeRabbit app | Yes | Nothing is reviewed, and nothing says so |
| 4 | The `policy:*` labels | Yes, for alerts | A label that does not exist cannot be applied, so no alert fires |
| 5 | Branch protection and `CODEOWNERS` | For enforcement | The review is advice; nothing is gated |

**Neither Slack secret is required.** The policy review works without them — the
labels still land on the pull request and the review still blocks the merge. The
secrets only decide whether a message also appears in chat.

### Two webhook names, and why

`POLICY_SLACK_WEBHOOK_URL` takes precedence over `SLACK_WEBHOOK_URL`:

```yaml
SLACK_WEBHOOK_URL: ${{ secrets.POLICY_SLACK_WEBHOOK_URL || secrets.SLACK_WEBHOOK_URL }}
```

`SLACK_WEBHOOK_URL` is usually a general-purpose webhook shared with other
workflows, which means it points at a channel already carrying build and deploy
traffic. A policy alert arriving among routine notifications is one nobody
reads. The dedicated name lets these go somewhere quiet without disturbing
anything else.

:::tip Prefer the dedicated name
`POLICY_SLACK_WEBHOOK_URL` with its own channel is the intended setup. The
fallback exists so nothing breaks before it is configured, not as an equally
good alternative.
:::

---

## 2. Create a Slack webhook

A Slack incoming webhook posts to **exactly one channel**. To change the channel
later you create a new webhook; you cannot repoint an existing one.

### 2.1 Pick a channel

Somewhere the people who own the architecture actually watch, and quiet enough
that a 🔴 message stands out. Avoid a build or deployment channel — that is the
problem this is solving.

### 2.2 Create the app and the webhook

1. Go to **https://api.slack.com/apps** and sign in.
2. **Create New App** → **From scratch**.
3. Give it a recognisable name — `Policy review` — and pick your workspace.
   **Create App**.
4. In the left sidebar, choose **Incoming Webhooks**.
5. Turn **Activate Incoming Webhooks** to **On**.
6. Click **Add New Webhook to Workspace**.
7. Choose the channel from step 2.1 and click **Allow**.
8. Copy the **Webhook URL**:

   ```text
   https://hooks.slack.com/services/<workspace-id>/<webhook-id>/<token>
   ```

   (Deliberately written with placeholders rather than a realistic-looking
   example: GitHub push protection rejects a commit containing anything shaped
   like a real Slack webhook, including in documentation.)

:::warning Treat it as a credential
Anyone with this URL can post to that channel. It goes straight from your
clipboard into the GitHub secret field — never into a pull request, an issue, a
commit or a chat message. If it leaks, delete the webhook in the Slack app and
create a new one; that is the only way to revoke it.
:::

---

## 3. Add the webhook to GitHub

1. Open the repository → **Settings**.
2. Sidebar: **Secrets and variables** → **Actions**.
3. **New repository secret**.
4. **Name:** `POLICY_SLACK_WEBHOOK_URL` — exactly, case-sensitive.
5. **Secret:** paste the webhook URL.
6. **Add secret**.

:::danger If you use an organisation secret instead
An organisation-level secret is invisible to any repository outside its
**Repository access** selection, and **nothing warns you** — the workflow simply
behaves as though the secret does not exist. If you go that route, check the
selection list, then verify with [section 4](#4-verify-it-works) rather than
assuming it worked.

A repository secret always beats an organisation secret of the same name.
:::

---

## 4. Verify it works

Do not wait for a real policy breach to find out.

`policy-alert.yml` has a manual trigger that sends one message:

1. Repository → **Actions**.
2. Choose **Policy alert** in the sidebar.
3. **Run workflow**.
4. Leave the branch as `main`; in **Policy label to simulate**, keep
   `policy:governance` or choose another `policy:*` label.
5. **Run workflow**.

One message should appear in your channel within a few seconds:

```text
🔒 Policy review — sequentech/step
   The policy system itself was changed
   Pull request: manual test    Author: <you>    Policy: .github/policies/50-governance.md
```

`Pull request: manual test` is expected — a manual run has no pull request, so
the message links to the workflow run instead.

:::note The button only appears once the workflow is on the default branch
GitHub shows **Run workflow** for a `workflow_dispatch` trigger only after the
workflow file has been merged to the default branch.
:::

### If no message arrives

Open the run and look at the **Notify Slack** job.

| What you see | What it means |
|---|---|
| The job did not run | The `if:` gate did not match. On a real pull request the label must start with `policy:` |
| `Compose the alert` → `skip=true` | That label is deliberately silent — it labels the pull request and pages nobody |
| Warning: `SLACK_WEBHOOK_URL is not set` | No webhook resolved. Check the secret name, and the repository selection if it is an organisation secret |
| `Slack notification` failed | The URL is wrong or the Slack app was deleted. Create a new webhook |
| All green, no message | You are watching a different channel — a webhook is bound to the channel it was created for |

---

## 5. What the other pieces do

### CodeRabbit

Installed as a GitHub app. Configuration lives in
[`.coderabbit.yaml`](https://github.com/sequentech/step/blob/main/.coderabbit.yaml),
read **from the pull request's own branch** — which is why `CODEOWNERS` guards
that file. Nothing is configured in the CodeRabbit web UI.

If the app is uninstalled or the plan lapses, no review happens and no error
appears. The weekly `policy-heartbeat.yml` job exists to catch that: it fails if
CodeRabbit has not responded to any non-draft pull request in a fortnight.

### The `policy:*` labels

CodeRabbit *applies* labels; it never creates them. A label that does not exist
is silently skipped — and because the alert workflow triggers on the label
event, no label means no alert. Create them in **Issues → Labels**, matching the
names in
[`.coderabbit.yaml`](https://github.com/sequentech/step/blob/main/.coderabbit.yaml).

Adding a policy therefore means three edits, not one: the Markdown file, the
`labeling_instructions` entry, and the label itself. The
`Policy consistency` check in CI verifies the first two agree.

### Branch protection and CODEOWNERS

The review reports; these enforce. In **Settings → Branches**, on the default
branch:

- **Require a pull request before merging**, with at least one approval.
- **Require review from Code Owners** — without this,
  [`CODEOWNERS`](https://github.com/sequentech/step/blob/main/.github/CODEOWNERS)
  is a comment and a pull request can weaken the very rules judging it.
- **Require status checks to pass** — `reuse` is a safe choice here, as it runs
  on every pull request with no path filter. A check gated behind a `paths`
  filter will hang pending forever on a pull request that does not touch those
  paths, and block the merge.

:::warning A CODEOWNERS team needs write access
GitHub silently ignores a `CODEOWNERS` entry naming a team that lacks **write**
on the repository — the rule simply does not apply, with no error. Enabling
"Require review from Code Owners" in that state gives you a gate that looks
enabled and enforces nothing. Check the owners resolve before relying on it.
:::

### Nothing to configure

`policy-consistency.yml` and `policy-heartbeat.yml` take no secrets; the
heartbeat uses the automatic `GITHUB_TOKEN`.

---

## Summary

| Secret | Required | Effect if absent |
|---|---|---|
| `POLICY_SLACK_WEBHOOK_URL` | No | Falls back to `SLACK_WEBHOOK_URL` |
| `SLACK_WEBHOOK_URL` | No | No Slack message; the check still passes |

No part of the merge gate depends on Slack. With neither secret set, policies are
still enforced by the review and by `CODEOWNERS` — you simply do not hear about
it in chat.

See [Architecture](architecture.md) for what the policies are defending.

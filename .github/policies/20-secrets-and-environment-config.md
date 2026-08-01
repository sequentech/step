<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# No credentials, no per-deployment configuration

> **Label:** `policy:secrets`
> **Reviewers:** `@sequentech/Architects`
> **If breached:** an Architect must approve, and a committed credential
> must be rotated before it is removed.

Source code and deployment configuration are different things with different
lifecycles, and secrets belong to neither. This repository holds the platform's
code; it is not where a particular deployment is described, and it is never
where a credential lives.

## The rule

**No credential may be committed, and no configuration specific to one
deployment or one customer may be added here.**

Anything committed to a public repository is public permanently. Rotation is the
only remedy for a leaked credential, and rewriting history does not undo the
disclosure.

## What breaks this rule

Report a violation when a change adds:

- **A credential of any kind** — passwords, API keys, bearer tokens, private
  keys, TLS keys, signing keys, database connection strings containing
  passwords, cloud access keys, webhook URLs carrying an embedded secret, or a
  `.env` file with real values.
- **Configuration belonging to one specific deployment or customer** — hostnames,
  cluster or account identifiers, per-customer resource names, per-environment
  replica counts, image tags pinned for one deployment, or values that would have
  to change for every installation of the platform.

Treat a high-entropy string assigned to a name suggesting a secret as a
violation and say so, even if it might be a placeholder: a false positive here
is cheap and a miss is not.

## What does not break it

- Obvious placeholders — `CHANGE_ME`, `your-api-key-here`, `xxx`, an empty
  string — and values in a `.example` or `.sample` file.
- Fixtures and mock values in tests, where the value is clearly synthetic and
  grants access to nothing.
- References to a secret rather than the secret itself: an environment variable
  name, a `secrets.*` reference in a workflow, or a key path in a secret store.
- Defaults that ship with the platform and apply to every installation, rather
  than to one.

## What to do instead

- For a credential: put it in the organisation's secret store and reference it
  by name. If one has already been committed, treat it as compromised — rotate
  it first, then remove it.
- For deployment configuration: it belongs with the deployment, not with the
  code. Ship a sensible default or a documented example here, and let each
  deployment supply its own values.

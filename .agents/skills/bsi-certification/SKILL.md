---
name: bsi-certification
description: Work in step that affects the BSI Common Criteria certification of the uniWAHL TOE (BSI-DSZ-CC-1291), such as TOE source and production definitions, the vulnerability disclosure policy, security.txt or consumer guidance exports.
---
<!--
SPDX-FileCopyrightText: 2026 Sequent Tech <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# BSI certification (step)

The evidence, plans and procedures live in beyond, in `docs/docusaurus/docs/bsi-certification/` on `feat/meta-10548/main` (sequentech/beyond#615). Its `2_alc/9_Internal_To_Do.md` is the authority for open work and decisions; read it first. Tracking issue: https://github.com/sequentech/meta/issues/10548.

## What in step is evidence

- The whole repository is part of the evaluated `temenos` tree, so every file enters the configuration list.
- `docs/docusaurus/docs/06-technology/06-vulnerability-disclosure-policy.md` and `docs/docusaurus/static/.well-known/security.txt` are the public ALC_FLR.2 guidance (ALC-07). A change to them changes delivered evidence: update the policy version and date, and the beyond to-do.
- TOE production definitions (Dockerfiles, lock files, build flags) are recorded in ALC-06 and the configuration list. A change needs a to-do entry and a regenerated list.

## Rules

- Branch from `feat/meta-10548/main` as `feat/meta-10548-<desc>/main` and open draft PRs into it with the `Related:` line.
- Every lint expectation needs an item-specific justification (SDLC §3.2); add it to the table in PR #2335.
- Run `reuse lint` before pushing; CI enforces REUSE.
- Do not merge, publish or contact the ITSEF without Eduardo's authorization.

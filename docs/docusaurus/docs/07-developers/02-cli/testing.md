---
id: testing
title: Step CLI boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/step-cli/tests/`](https://github.com/sequentech/step/blob/feat/meta-13302-ui-essentials-coverage/main/packages/step-cli/tests). Commands state their working directory.

The credential-export slice covers independent PBKDF2 vectors, Unicode and
quoted CSV fields, preservation of previous files on malformed input, duplicate
password headers, reserved credential headers and actual subprocess exit codes.
All credentials are synthetic; tests use private temporary directories and low
iteration counts for fast feedback. Production defaults remain 600,000 iterations.

The pre-existing election end-to-end test uses a complete election environment
and external Loadero automation. It remains ignored; this work invokes no paid
test service. A local browser replacement is tracked in [Meta #13298](https://github.com/sequentech/meta/issues/13298). These file and subprocess tests do not certify live election workflows
or establish 95% package coverage. The measured scope and remaining gap belong
in [Meta #13302](https://github.com/sequentech/meta/issues/13302); do not exclude untested commands to make the percentage pass.

Credential conversion validates the complete CSV before publishing it. Duplicate
password headers and reserved credential headers are rejected. A successful
export atomically replaces the destination with a private file; a failed one
leaves the prior export intact. The CLI reports failures with a nonzero exit code.
Hashing processes at most 256 rows per batch, preserving source order without
retaining the entire plaintext census in memory. A failed-writer control counts
how many records were read; separate 600-row cases check batch order and cleanup
after a malformed final row.

Import tests compare file hashes with independent vectors, reject contradictory
sources before upload and refuse partial GraphQL results containing errors.

From the repository root:

```bash
python3 scripts/coverage/run.py step-cli --baseline --offline
```

This emits HTML, JSON and LCOV reports. Remove `--baseline` to enforce 95% and
require every source file to have a measurement or reviewed scope explanation.

Empty document references now fail before network access; their regression fails
on the previous code and a valid reference remains accepted. Additional cases
pin the standard empty-file SHA-256, preserve I/O error types and reject trailing
JSON documents and invalid UTF-8 while retaining valid null and zero counts.
The suite has 34 passing tests and one pre-existing ignored service journey.

Native coverage at `c4f9dc3` is 482/3,851 lines (12.52%), 42/245 functions
(17.14%) and 738/6,148 LLVM regions (12.00%). The actual Step CLI PR base has
two passing tests and measures 45/3,787 lines, 4/237 functions and 58/6,061
regions. Every fraction increases; CI compares all three independently.
Authenticated election workflows beyond the owned import peer remain open.

Review regressions run the shipped import command against an owned loopback
HTTP peer with a private executable/config directory. Partial GraphQL data is
rejected before upload and after import, errors reach shell callers through a
nonzero exit, and empty error lists remain valid. Existing and dangling output
symlinks are rejected without changing either the link or its target. These
regressions fail before the small fixes and pass with the valid controls.

Hard-link aliases are rejected by Unix device/inode identity before output is
created. An independent regular-file replacement remains valid. Both tally import
commands return failure status for preparation or backend errors; actual HTTP
success and error subprocesses exercise those paths. Windows file identity and
authenticated end-to-end election journeys remain separate scopes.

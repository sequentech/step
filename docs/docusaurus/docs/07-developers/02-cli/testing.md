---
id: testing
title: Step CLI boundary tests
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

Test file paths in this guide are relative to [`packages/step-cli/tests/`](https://github.com/sequentech/step/blob/main/packages/step-cli/tests). Commands state their working directory.

The credential-export slice covers independent PBKDF2 vectors, Unicode and
quoted CSV fields, preservation of previous files on malformed input, duplicate
password headers, reserved credential headers and actual subprocess exit codes.
All credentials are synthetic; tests use private temporary directories and low
iteration counts for fast feedback. Production defaults remain 600,000 iterations.

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

Empty document references fail before network access; a valid reference remains
accepted. Input tests
pin the standard empty-file SHA-256, preserve I/O error types and reject trailing
JSON documents and invalid UTF-8 while retaining valid null and zero counts.
Command tests run the shipped import command against an owned loopback
HTTP peer with a private executable/config directory. Partial GraphQL data is
rejected before upload and after import, errors reach shell callers through a
nonzero exit, and empty error lists remain valid. Existing and dangling output
symlinks are rejected without changing either the link or its target. Each rejected input has a successful control.

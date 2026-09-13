<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Step CLI boundary tests

The credential-export slice covers independent PBKDF2 vectors, Unicode and
quoted CSV fields, preservation of previous files on malformed input, duplicate
password headers, reserved credential headers and actual subprocess exit codes.
All credentials are synthetic; tests use private temporary directories and low
iteration counts for fast feedback. Production defaults remain 600,000 iterations.

The pre-existing election end-to-end test uses a complete election environment
and external Loadero automation. It remains ignored; this work invokes no paid
test service. A local browser replacement is tracked in Meta #13298. These file and subprocess tests do not certify live election workflows
or establish 95% package coverage. The measured scope and remaining gap belong
in Meta #13297; do not exclude untested commands to make the percentage pass.

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

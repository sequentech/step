<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Voting-load runtime

The public interface is `step-cli load`. The CLI bundles this directory at build time and extracts a versioned runtime into the user's cache; no checkout is needed to execute an installed binary.

| Module | Responsibility |
| --- | --- |
| `driver.py` | Preparation lifecycle, executor selection, result collection |
| `provision.py` | Synthetic election, automatic ceremony, census import |
| `runner.py` | Shared protocol, streaming census, finite shard ownership |
| `scale.k6.js`, `replay.k6.js` | Authenticated HTTP journeys |
| `aggregate.py`, `presentation.py` | Disk-backed aggregation and standalone reports |
| `capture.py`, `capture_report.py` | Optional development HAR/SQL diagnostics |
| `Dockerfile`, `image.sh` | Source-only worker packaging |

Chromium uses the shared flow in `packages/voting-portal/test/load/flow.ts`. Native encryption lives in `packages/step-cli/src/load/encryption.rs`. Configuration defaults and rustdoc live in `packages/step-cli/src/load/config.rs`; `step-cli load reference` generates their public reference.

The bundled fixture retains the deployment authentication flows but contains one contest, no users and no exported client or CAPTCHA secrets. Custom election exports are supplied through `preparation.template`.

Run module tests from the repository root inside devenv:

```bash
python3 -m unittest discover \
  -s packages/voting-load
```

The tests cover voter ownership, shared hashing, duplicate receipts, partial reports, and failure-time collection. Full validation also needs fresh k6 and Chromium journeys against an open synthetic election; unit tests do not establish deployment capacity.

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Voting load engines

Use `step-cli load`. Coordination, census generation, native encryption, scheduling
and SQLite-backed reporting live in `packages/step-cli/src/load`. Installed CLI
binaries embed their engine adapters and need no repository checkout or Python.

| Source | Responsibility |
| --- | --- |
| `scale.k6.js`, `replay.k6.js` | Fresh-session authenticated HTTP journeys |
| `worker.rs`, `worker.Cargo.toml` | Small standalone Rust worker; shares the CLI's ownership and engine code |
| `Dockerfile` | Build the Rust worker and package the selected engine |
| `report.html` | Portable report layout, populated by Rust with inline SVG and fonts |
| `capture.py`, `capture_report.py` | Optional developer HAR/SQL diagnostics; never used by the CLI |

Chromium uses `packages/voting-portal/test/load/flow.ts`. Configuration defaults and
Rustdoc live in `packages/step-cli/src/load/config.rs`; `step-cli load reference`
generates the operator reference. Image builds accept configurable base images and
Playwright versions through `step-cli load image --help`.

The fixture retains deployment authentication flows but contains one contest,
no users and no exported client or CAPTCHA secrets. Supply custom exports through
`preparation.template`.

Run native behavioral tests from the repository root:

```bash
cargo test \
  --manifest-path packages/step-cli/Cargo.toml \
  load::
```

Optional capture diagnostics have separate Python tests:

```bash
python3 -m unittest discover \
  -s packages/voting-load
```

Native tests cover unique voter allocation, shared password hashing, immutable
claims, duplicate receipts, exact quantiles and partial reports. Full validation
also needs fresh k6 and Chromium journeys; unit tests do not establish capacity.

The k6 adapters use the locally bundled url-1.0.0.js from
[Grafana's URL library](https://jslib.k6.io/url/1.0.0/index.js) to compare canonical
HTTP(S) origins, including IPv6 and internationalized hostnames. The vendored
file records its upstream checksum and licenses; keep it unformatted when updating.
It is included in the CLI runtime and worker images, with no runtime download.

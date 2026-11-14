<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# voting-load threat model

voting-load holds the engines that `step-cli load` runs: k6 scripts that replay the voter HTTP journey (Keycloak login with PKCE, `GetVoterStatus`, publication downloads, `InsertCastVote` with pre-encrypted ballots), the `step-load-worker` entry point and its container images, the default synthetic election fixture, the HTML report template and optional Python capture tools for developers. It runs on operator machines, in the devcontainer and in Docker or Kubernetes worker pods, never on a production voting path, but it logs in synthetic voters and casts real ballots against whatever deployment it targets. Coordination, census, encryption and provisioning belong to the [step-cli threat model](../step-cli/THREAT_MODEL.md); see also the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Synthetic voter secrets**: the shared password in `LOAD_PASSWORD`, authorization codes, PKCE verifiers and access tokens. Confidentiality: they let anyone vote as every synthetic voter.
- **Run inputs**: ballot shards (`LOAD_BALLOTS`), run configuration (`LOAD_CONFIG`) and the default fixture `fixtures/election.json`. Integrity: they decide what is cast and against which event.
- **Results and diagnostics**: `RESULT` samples, k6 summaries, worker and bootstrap logs, capture HAR and SQL files. Confidentiality (signed URLs, tokens, logged SQL values) and integrity (capacity conclusions).

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `worker.rs` `run` (`step-load-worker`, image `ENTRYPOINT`), `scale.k6.js` default and `handleSummary`, `bootstrap.k6.js` default | step-cli coordinator, Docker or Kubernetes indexed Job | internal service | `--index` or `JOB_COMPLETION_INDEX` parsed as an integer; run directory canonicalized before step-cli `src/load/worker.rs` `node`; shard length must match its allocation (`Invalid shard`); bootstrap throws if asked to cast |
| Target responses parsed by `replay.k6.js` `replayJourney` (login form, `Location`, token JSON, voter status, publication objects) | Keycloak, Hasura, harvest and object storage of the target | untrusted | `approvedUrl` origin allowlist on every request, `redirects: 0`, `state`, `nonce` and publication scope checks |
| `fixtures/election.json` (default `preparation.template`), `report.html` (filled by step-cli `src/load/presentation.rs`) | Repository maintainers | operator | Fixture has no users, client secrets or key material; report values pass through step-cli `escape` |
| `capture.py`, `capture_report.py`, `resources.py`, `replay_profile.py`; listeners `proxy.py` and `serve_portal.py` | Developer; local processes | operator (CLIs), untrusted (listeners) | DSNs come from named environment variables; capture output directory must be new; proxy serves one path and needs a credential-free HTTP upstream origin |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| voting-load-T1 | Information disclosure | The synthetic password, tokens or signed publication URLs leak through engine output, result samples or reports | Password read only from `LOAD_PASSWORD` (`scale.k6.js`, `bootstrap.k6.js`); `scale.k6.js` default reports only reasons in its `known` set or `HTTP failure at <phase>: <status>`; `replay.k6.js` `send` traces URLs without query or fragment, off in scale runs unless `trace_http` is true; `RESULT` holds index, reason, timings and receipt id. Raw engine logs are private files and can hold server responses | Partial |
| voting-load-T2 | Spoofing | A target response steers credentials or the bearer token to another host, or a journey accepts another session's code or tokens | `replay.k6.js` `approvedUrl` and `origin` (HTTP(S) only, no userinfo, canonical origin in `allowed_origins`) on every request, including the login form action, publication URLs and the cast; `redirects: 0`; fresh `http.CookieJar`, random `state`, S256 verifier and `nonce` per journey, checked on the callback and ID token; `tests/origins.k6.js` | Mitigated |
| voting-load-T3 | Tampering | A failed or mismatched cast counts as accepted, a voter casts twice, or shards and configuration change between prepare and run | `scale.k6.js` default requires HTTP 200, no GraphQL errors and a receipt whose `ballot_id`, `election_id` and `election_event_id` match; `accepted_casts` and `journey_failures` thresholds; `shared-iterations` without retry; `replayJourney` rejects a changed style, publication version or scope; step-cli `src/load/worker.rs` `shard` checks `config_sha256` and shard `.sha256` files that sit beside the data (step-cli-T12) | Partial |
| voting-load-T4 | Elevation of privilege | A run adds synthetic voters and their ballots to a real election event | Event provisioning belongs to step-cli (step-cli-T10); `fixtures/election.json` has one contest and no users | Partial |
| voting-load-T5 | Spoofing | Someone other than the run's synthetic voters authenticates to a load event | Fixture holds no users, client secrets or keys (key providers are generated on import); operators can supply their own export through `preparation.template` | Partial |
| voting-load-T6 | Information disclosure | Developer diagnostics or local listeners expose tokens, credentials, ballots or database contents | `capture.py` `main` sets umask `0o077` and needs a new output directory, `save` writes 0600, `preflight` keeps only exception types; `resources.py` `extract`, `traffic.py` `inventory` and `capture_report.py` `request_key` drop headers, bodies and query values; `proxy.py` forwards one path and logs no headers or bodies. Capture relies on full database statement logging configured outside the tool | Partial |
| voting-load-T7 | Denial of service | The generator exhausts the target or shared infrastructure | Finite iterations, `maxDuration`, `request_timeout` and `cast_timeout` in `scale.k6.js` and `replay.k6.js` | Accepted (generating load is the purpose; the operator chooses target and concurrency) |
| voting-load-T8 | Tampering | Worker images or vendored code are substituted during the build | `Dockerfile` builds with `cargo build --locked` from `worker.Cargo.lock` and runs as UID 1000; the k6 image has no Python, Node or admin clients; `url-1.0.0.js` is vendored with its upstream SHA-256 recorded in a comment and is never downloaded at runtime | Partial |

## Assumptions

- **step-cli** keeps run directories private, starts engines with a cleared environment holding only `LOAD_CONFIG`, `LOAD_PASSWORD`, shard paths and proxy/CA settings (`src/load/worker.rs` `environment`), and guards provisioning.
- **Operators** target disposable events or non-production deployments, configure HTTPS outside a local stack, run capture tools only against a local stack, and remove run directories, Kubernetes Secrets and synthetic events afterwards.
- **voting-portal** `test/load/flow.ts` and `test/load/capture.spec.ts` handle credentials in the Chromium journeys; the voting-portal model covers them.

## Review focus

1. Event targeting with step-cli: how a run selects and is bound to its event.
2. Authentication settings of load events.
3. Secret and token flow: transport, engine environment, engine output, logs, samples and reports.
4. Developer diagnostics and the image supply chain: what capture tools write, local listeners, base images and installed packages.

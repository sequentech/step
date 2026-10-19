---
id: vote-privacy-validation
title: Ballot privacy validation
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

This procedure checks a disposable election containing synthetic voters. It is an operator acceptance procedure, not evidence that a deployment has already passed. Keep its private identity manifest and resolver output outside public evidence bundles.

## Collect and scan ballot artifacts

Create a local `forbidden.json` listing the synthetic values assigned to the test voter. Supported categories are `voter_id`, `username`, `ip`, `country`, `device`, and `credential`. Values are nonempty arrays of exact strings. Use unique synthetic sentinels; do not supply production credentials.

```json
{
  "voter_id": ["synthetic-private-voter"],
  "username": ["Synthetic Privacy Voter"],
  "ip": ["198.51.100.73"],
  "country": ["AQ"],
  "device": ["synthetic-private-device"]
}
```

Collect explicit files from the ballot table export, ballot-only signed-log statements, public Logs response, downloadable proof, receipt-task output, ballot CSV exports, and receipt PDF. Extract the PDF with a local PDF text tool, such as `pdftotext receipt.pdf receipt.pdf.txt`. Scan only the selected ballot artifacts:

```sh
python3 scripts/vote_privacy_audit.py --forbidden forbidden.json \
  ballot-table.json ballot-statements.json logs-response.json \
  proof.json receipt-task.json ballots.csv receipt.pdf.txt
```

The scanner prints only each basename and finding categories. Exit status 0 means no supplied value was found, 1 means a finding, and 2 means an unreadable or unsupported artifact. Missing files and unextracted PDFs fail the check. JSON strings, nested JSON envelopes, and canonical message base64 in `message`, `received_entry`, and `cast_entry` fields are decoded before checking. Country codes match complete tokens; other values match exact substrings. Traversal is bounded to 32 decoded JSON levels; deeper artifacts fail the check.

Do not scan a complete mixed authentication log: its signed authentication records legitimately identify the signing-in user. Export only the ballot Received/Cast entries for this check. Do not include the private mapping, forensic store, identity manifest, resolver response, or an approved confirmation email in the public-artifact collection. Voter reports may contain voter identities but must not contain a ballot identifier or ballot timestamp beside them; inspect that separate invariant rather than treating the whole voter report as a ballot artifact.

This is a known-value regression detector. It does not prove the absence of arbitrary personal data, covert channels, image-only PDF content, or unrecognized encodings. Retain the exact artifact inventory and scan result with the acceptance evidence.

## Logs-to-binding-to-resolution demonstration

1. Create a new pseudonym event with signed voter ballots and signed ballot-box receipts. Publish its ballot and synthetic census. Enable the voter Logs tab and open ballot checks. Record the event's independently published ballot-box key and log public key.
2. Receive and cast one synthetic ballot. Record its Ballot ID privately. Confirm its voter receipt task has no executor identity exposed in the task list.
3. In the voter Logs tab, locate that Ballot ID. Record the displayed pseudonym. Confirm another authorized voter in the same area sees the same public row, and a voter outside the area or election cannot fetch it. The public row must not contain the synthetic identity or network values.
4. Open the proof. While sequencing is pending, absent evidence must say Pending. After sequencing, all four checks must pass: authority binding, received ballot/voter signature, derived Ballot ID, and cast signatures. The Received entry must bind the same pseudonym hash and voter public key, event, and election. Download the exact proof bytes and include them in the artifact scan. Use the automated verifier regressions for altered signatures, keys, channels, and pseudonyms; a manually altered display label is not sufficient evidence of a cryptographic failure.
5. With an audit operator's configured CLI, validate the event log through its existing audit task:

   ```sh
   step-cli step audit-electoral-log --election-event-id EVENT_UUID
   ```

6. Confirm an account without `voter-pseudonym-resolve` is refused by the resolver. Only the explicitly authorized investigation operator performs the following step, with a reason containing no voter or network data:

   ```sh
   step-cli step resolve-voter-pseudonym \
     --election-event-id EVENT_UUID \
     --pseudonym XXXX-XXXX \
     --reason "Authorized synthetic privacy acceptance check"
   ```

   Keep the returned voter ID in the private acceptance record. Confirm it matches the synthetic census account. Ordinary Logs and proof actions must never return this resolution.

7. Wait for the signed `PseudonymResolved` audit entry. Verify its authority signature through the electoral-log audit and inspect its actor, pseudonym, and reason. Confirm the resolved voter ID is absent from the public statement. Repeat the ballot artifact scan and retain its output.
8. Repeat cast/storage checks with **Do not store** and **Keep for forensic review**. With forensic retention enabled, use the dedicated permission to view attribution and confirm every successful read queues a signed `ForensicRead` statement. Without that permission, the request must fail before private storage is read. For an already published pseudonym event, repeat a successful voter sign-in and confirm a private `sign-in` record exists only with forensic retention enabled. Confirm it contains the pseudonym, not the voter ID, and ordinary sign-in logs/queues contain no network metadata. Disable the policy and repeat immediately; no new attribution should be sent or retained. Authentication rollback, failed login and tenant administrator login must not create private records. During an unavailable Harvest or token service, completed login remains valid and attribution may be missing; this path has no durable retry queue.

Record actual results for every step. The procedure itself does not resolve a voter, change a running election, or establish approval to investigate production voters.

## Reproducible local demonstration

Run the synthetic demonstration inside `devenv` with `ELECTORAL_LOG_PG_*` pointing to a disposable event catalog and `HASURA_DB__*` pointing to a test PostgreSQL server whose user can create databases. The driver uses devenv's `CHROMIUM_EXECUTABLE_PATH` for the generated PDF check; `CHROME` can select another local Chromium executable. The fixtures create random event databases and a separately migrated Harvest database; never select a production catalog. The driver uses a synthetic local vault key only for secrets it creates.

```sh
scripts/dev/step-dev vote_privacy_proof
```

The first stage sequences a real signed encrypted ballot, verifies all four public proof checks and Merkle inclusion, scans generated JSON, CSV and PDF text, exercises the real User's Logs filter/export, and repeats verification after mapping purge. The second stage follows a ballot through the public log and proof APIs, then exercises refused and authorized resolution of that same pseudonym, the signed resolution audit and its deduplication, and refusal after purge. Each stage must execute exactly one passing integration test; an empty test selection fails the driver.

This is a local API and storage demonstration. Browser stories separately check the rendered proof and responsive lists. Record the deployed browser/CLI scenario above, custom report templates and operational acceptance separately; this driver does not certify their execution or peak throughput.

## Sign-in capture configuration

The custom Keycloak event listener uses the deployment's internal `HARVEST_DOMAIN` origin (a host and port, or an explicit HTTP/HTTPS origin) and the existing `KEYCLOAK_URL`, `KEYCLOAK_CLIENT_ID` and `KEYCLOAK_CLIENT_SECRET` service-account configuration. Use HTTPS when the deployment network requires transport encryption. Redirects are never followed. The tenant service-account token scopes both requests to the tenant and event.

Event realm templates and imports remove Keycloak's built-in `jboss-logging` listener, which otherwise writes raw request addresses and event details independently of the event's network policy. Beat schedules the same idempotent reconciliation for existing event realms at startup. The custom signed-log listener remains enabled; unrelated listeners and configured event/admin audit stores retain their settings. Check the reconciliation task completes when upgrading an existing deployment.

After a successful event-realm `LOGIN` transaction, the listener requests `GET /privacy/sign-in-network/policy?tenant_id=...&election_event_id=...` without voter or network metadata. Only an explicit `forensic-store` response allows `POST /privacy/sign-in-network`. The policy is not cached; the receiving POST checks it again. Each Harvest request has a two-second timeout; acquiring an uncached service token retains the shared token client's existing timeouts. The listener copies only bounded immutable fields while the request context is live, and submits them after commit to a process-local worker. No worker retains a Keycloak session, and token or HTTP calls do not run on the login response thread. A semaphore bounds active, queued and delayed deliveries together to 1,024 per process, using four worker threads. Transient transport failures, HTTP 429 and HTTP 5xx responses receive at most six attempts, with delays of 1, 2, 4, 8 and 16 seconds. Every attempt repeats the policy preflight and uses the same delivery identifier so storage deduplicates an uncertain previous acceptance. Other authorization/input refusals are terminal. Failures produce generic warnings without payloads or exceptions. Shutdown cancels volatile pending work and interrupts workers; no payload is copied to a durable queue or logs.

The IP is Keycloak's request address; configure trusted proxy handling correctly. Optional country uses `CF-IPCountry`, which the deployment's trusted proxy must replace or strip, and browser data uses `User-Agent`. These are attribution hints, not proof of a person's location or device. Invalid country codes and user agents over 1,024 UTF-8 bytes are omitted. Missing or purged pseudonym mappings are never reconstructed by authentication. Smart-link registration allocates pseudonyms through its existing durable registration task. If a published event has an authoritative eligible voter whose mapping is still pending, sign-in ingestion returns a retryable response instead of silently accepting and discarding the record. Unpublished events, nonvoters and permanently purged mappings terminate capture. The retry test must exercise pending registration, run the normal allocator, then verify one retained record even after duplicate delivery. A registration outage longer than the bounded retry horizon can still lose attribution; durable registration retries do not make the volatile forensic transport lossless.

The CLI cast-vote CSV keeps its existing `hash_voter_id` header for compatibility. For new pseudonym events that field contains the signed pseudonym hash, and `ballot_id` contains the receipt's display code. Export does not look up the private voter mapping. The older identity-based voting acceptance report explicitly refuses pseudonym events; use the public-proof demonstration above for that mode.

## Automated checks and their limits

```sh
python3 -m unittest discover -s scripts -p test_vote_privacy_audit.py
```

Rust validation uses the repository's `devenv` shell and checkout-local Cargo target. The native core privacy verifier tests exercise real signatures and unchanged receipt vectors. Electoral-log privacy integration tests require their isolated database. Harvest's `do_not_trace_private_attribution_at_any_level` and `do_not_trace_attribution_at_any_level` tests capture TRACE events and complete span lifecycles for successful and refused receive/cast requests with synthetic attribution. Those tests use scripted ballot storage: they prove the HTTP/request-guard/route logging boundary, not every background worker or storage implementation. Additional regressions capture worker read/refusal spans and bridged SQL-driver events after a runtime TRACE reload. The normal logger suppresses PostgreSQL driver DEBUG/TRACE parameter output while preserving INFO, WARN and ERROR diagnostics. The actual deployment acceptance run must collect its ballot-path logs and artifacts separately.

The listener tests mock all HTTP calls. They cover preflight refusal, policy changes between retries, authenticated stable payloads, transaction commit/rollback, request-context independence, bounded metadata, nonblocking dispatch, capacity, finite retries, shutdown and sanitized failure logs. Harvest route tests cover service authorization and policy/storage boundaries. These tests do not send real sign-in data or establish deployed availability.

## Audit lifecycle and forensic transactions

The authenticated `audit_ballot(election_id, ballot_id)` mutation marks only the requesting voter's received ballot in the authorized event, election and area. Audit and cast serialize on that ballot: one wins, the other is refused. Repeating an audit is safe. The portal must wait for acknowledgment before exposing audit data, invalidate the old receipt/key, and discard audit material after cast. Exercise Audit → Back → Cast and Cast → Audit through the connected portal, and check the queued, pseudonymous `BallotAudited` record.

Synthetic receipt previews remain private ordinary administrative reports. Their filenames start with `preview_ballot_receipt_`, and their annotations omit the `BallotReceipt` purpose (`ballot-receipt` on the wire). Administrators download these samples through the normal document route. Real receipts retain the protected purpose and the legacy `ballot_receipt_` filename classification; their download requires authenticated ballot ownership. Check both preview download success and refusal of real receipts through ordinary administrative document access.

Forensic receive metadata is inserted with the new receipt, and forensic cast metadata with ballot acceptance, in their respective database transactions. A failed attribution insert rolls back acceptance. An idempotent receive replay preserves the original attribution rather than combining the replay's address with an earlier receipt time. The address-summary preview/count queries have an event/address index; check their actual plans at large address cardinality.

The ignored `matched_receipt_and_cast_storage_cost` test in electoral-log's `privacy_receipts` target compares the same receive/cast pair in legacy signed-receipt, pseudonym do-not-store and pseudonym forensic-store modes. It rotates mode order, warms each mode twice, then measures twelve batches of eight pairs. Run it alone against the disposable storage-test database. The reported p50/p95 values are quantiles of each batch's mean per pair, excluding fixture setup and cryptographic signing; they are not per-request tail latency or production throughput. On the local 19 October 2026 run those p50/p95 values were 2.143/2.317 ms, 2.461/2.972 ms and 2.637/2.989 ms respectively. Record new raw batches on the deployment acceptance hardware rather than extrapolating these values to capacity.

## Shared ballot-page proxy

The local, remote and air-gap preparation Compose configurations include the internal `harvest-edge` service; backend and full development modes start it with Harvest. Hasura routes only `ballot_box_log` through `HARVEST_BALLOT_LOG_DOMAIN` (default `harvest-edge:8080`); other actions continue using `HARVEST_DOMAIN`. The proxy forwards to that existing Harvest origin. Deployments that apply this metadata outside these Compose configurations must also provide the internal proxy and `HARVEST_BALLOT_LOG_DOMAIN` in the Hasura environment. This setting is required when applying the updated action metadata.

The internal Nginx template is `.devcontainer/harvest-edge/default.conf.template`. Its cache contains only successful newest public ballot pages for three seconds. Every request, including a hit, first calls Harvest's `/ballot-box-log-authorize` with the bearer token. Harvest verifies the token, event, area, authorized elections, logs setting and checks period, then returns an opaque population-scope digest. The cache key combines that scope, HTTP method and request body. Empty or file-backed bodies bypass caching. Own-ballot, search, cursor, investigation and mutation responses are never shared. Downstream responses remain `private, no-store`; only this internal proxy overrides that header for its explicitly marked cacheable route. Access logging is disabled.

Run `python3 scripts/dev/test_ballot_log_edge.py` with Docker and the local `nginx:latest` image to exercise the shipped template using temporary upstreams and a disposable proxy. It checks cross-user sharing within one authorized population, isolation across populations and request bodies, reauthorization on hits, denied access, method isolation and private-path bypass. It does not restart the development stack.

Search limits are shared in each event database, keyed by a one-way digest of the authenticated session and scope. Refreshed access tokens retain their session limit; different sessions are independent. Expired limiter rows are removed in bounded batches, without retaining raw tokens, user IDs or network metadata.

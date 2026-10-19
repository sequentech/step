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

The scanner prints only each basename and finding categories. Exit status 0 means no supplied value was found, 1 means a finding, and 2 means an unreadable or unsupported artifact. Missing files and unextracted PDFs fail the check. JSON strings, nested JSON envelopes, and canonical message base64 in `message`, `received_entry`, and `cast_entry` fields are decoded before checking. Country codes match complete tokens; other values match exact substrings.

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

## Sign-in capture configuration

The custom Keycloak event listener uses the deployment's internal `HARVEST_DOMAIN` origin (a host and port, or an explicit HTTP/HTTPS origin) and the existing `KEYCLOAK_URL`, `KEYCLOAK_CLIENT_ID` and `KEYCLOAK_CLIENT_SECRET` service-account configuration. Use HTTPS when the deployment network requires transport encryption. Redirects are never followed. The tenant service-account token scopes both requests to the tenant and event.

After a successful event-realm `LOGIN` transaction, the listener requests `GET /privacy/sign-in-network/policy?tenant_id=...&election_event_id=...` without voter or network metadata. Only an explicit `forensic-store` response allows `POST /privacy/sign-in-network`. The policy is not cached; the receiving POST checks it again. Each Harvest request has a two-second timeout; acquiring an uncached service token retains the shared token client's existing timeouts. Capture runs synchronously after commit, so token acquisition and the two Harvest calls can delay the login response by approximately 14 seconds in total when they reach their timeouts. Failures produce generic warnings without payloads or exceptions, and never enqueue a retry payload.

The IP is Keycloak's request address; configure trusted proxy handling correctly. Optional country uses `CF-IPCountry`, which the deployment's trusted proxy must replace or strip, and browser data uses `User-Agent`. These are attribution hints, not proof of a person's location or device. Invalid country codes and user agents over 1,024 UTF-8 bytes are omitted. Missing or purged pseudonym mappings are never reconstructed by authentication. Smart-link registration allocates pseudonyms asynchronously; its first login can arrive before allocation, in which case that sign-in attribution is omitted. Use an existing published registry for the successful-capture acceptance control, and test the registration timing separately.

## Automated checks and their limits

```sh
python3 -m unittest discover -s scripts -p test_vote_privacy_audit.py
```

Rust validation uses the repository's `devenv` shell and checkout-local Cargo target. The native core privacy verifier tests exercise real signatures and unchanged receipt vectors. Electoral-log privacy integration tests require their isolated database. Harvest's `do_not_trace_private_attribution_at_any_level` and `do_not_trace_attribution_at_any_level` tests capture TRACE events and complete span lifecycles for successful and refused receive/cast requests with synthetic attribution. Those tests use scripted ballot storage: they prove the HTTP/request-guard/route logging boundary, not every background worker or storage implementation. The actual deployment acceptance run must collect its ballot-path logs and artifacts separately.

The listener tests mock all HTTP calls. They cover preflight refusal, immediate policy changes, authenticated payloads, transaction commit/rollback, bounded metadata and sanitized failure logs. Harvest route tests cover service authorization and policy/storage boundaries. These tests do not send real sign-in data or establish deployed availability.

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
8. Repeat cast/storage checks with **Do not store** and **Keep for forensic review**. With forensic retention enabled, use the dedicated permission to view attribution and confirm every successful read queues a signed `ForensicRead` statement. Without that permission, the request must fail before private storage is read. Fresh sign-in forensic capture is currently unavailable: the private Keycloak sender is not implemented or connected. The authenticated ingress and audit redaction do not establish capture. Do not declare that requirement passed until the sender is integrated and tested.

Record actual results for every step. The procedure itself does not resolve a voter, change a running election, or establish approval to investigate production voters.

## Automated checks and their limits

```sh
python3 -m unittest discover -s scripts -p test_vote_privacy_audit.py
```

Rust validation uses the repository's `devenv` shell and checkout-local Cargo target. The native core privacy verifier tests exercise real signatures and unchanged receipt vectors. Electoral-log privacy integration tests require their isolated database. Harvest's `do_not_trace_private_attribution_at_any_level` and `do_not_trace_attribution_at_any_level` tests capture TRACE events and complete span lifecycles for successful and refused receive/cast requests with synthetic attribution. Those tests use scripted ballot storage: they prove the HTTP/request-guard/route logging boundary, not every background worker or storage implementation. The actual deployment acceptance run must collect its ballot-path logs and artifacts separately.

---
id: auditor_transparency
title: Verify transparency receipts and evidence
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

The ballot verifier offers **Transparency receipt** beside the existing ballot audit. Open a receipt-chain file, paste a receipt reference, or follow the confirmation page's **Check independently** action. Check the trust policy's ID and six words against the official notice before pinning it. A policy carried by a receipt link is a candidate, not an independently established trust anchor.

Provide the expected ballot ID, event and election when the receipt file does not already carry the entry the voter expects. Inspect the three facts and **Proof steps**. Pending is distinct from a contradiction, and an invalid object alone is not cryptographic evidence of misconduct. Missing profile meaning metadata keeps overall claims pending while retaining the cryptographic tier results.

The first check fetches only from policy logs. The full check can contact mirrors, hubs and monitors, normally after a random delay. Private-event policies and upgrades are not public: obtain the policy and receipt through the authorised channel, check the included material offline, and ask the organiser for any missing opening or auditor attestation. Do not send another voter's private material to a public service.

With the Vitrine CLI installed, the equivalent offline starting points are:

```sh
vitrine policy check policy.txt
vitrine policy words policy.txt
vitrine chain verify receipt.vrc --policy policy.txt --offline \
  --expect event_id=EVENT_ID --expect election_id=ELECTION_ID --expect ballot_id=BALLOT_ID
vitrine evidence verify evidence.txt --pin policy.txt
```

Replace `EVENT_ID`, `ELECTION_ID` and `BALLOT_ID` with the values you expect independently. An offline check cannot establish fresh successor discovery, so some claims remain pending.

Use `vitrine --help` and each subcommand's `--help` for the installed version's input and output options. Evidence verification requires a separately pinned policy; a policy embedded in an evidence file does not establish trust. Preserve the original bytes of receipts, signed notes, policies and evidence when reporting a contradiction. The receipt establishes logged inclusion and consistency properties, not the correctness of a tally or the secrecy of a voter's choices.

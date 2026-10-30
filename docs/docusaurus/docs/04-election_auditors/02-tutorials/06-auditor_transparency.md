---
id: auditor_transparency
title: Verify transparency receipts and evidence
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

The ballot verifier offers **Transparency receipt** beside the existing ballot audit, with a **Check a receipt** button. On **Check a transparency receipt**, open a receipt file (the `.vitrine-receipt` file that **Save receipt** saves in the voting portal), paste a receipt, its link or its reference, or follow the ballot tracker's **Check it independently** action. Check the trust policy's ID and six words against the official notice before pinning it. A policy carried by a receipt link is a candidate, not an independently established trust anchor.

**Your ballot** shows the ballot ID the page looks for, as the short ID and the full ID in groups of four. Provide the expected full ballot ID, event and election when the receipt file does not already carry the entry the voter expects. The field refuses a short ID, and **Change the ballot ID** looks for another one. **Result** shows a headline and three steps: **In the public record**, **Confirmed by witnesses** and **Anchored publicly**. Each step shows what its checks found, such as the entry number, how many witnesses signed and how many the policy needs, and how many hubs hold the record.

Pending is distinct from a contradiction. A step that is still being checked is not a failure, and **Taking longer than expected** is not evidence of tampering. **The record does not match** means that validly signed objects contradict each other: **Save evidence** saves the receipt file that holds them, from which an auditor can build the evidence file with the Vitrine verifier. **Couldn't be checked** means that some data looks damaged, and an invalid object alone is not cryptographic evidence of misconduct. Missing profile meaning metadata keeps the overall result pending, so the status does not reach **Verified**, while each step keeps its own result.

The first check fetches only from policy logs. The full check can contact mirrors, hubs and monitors. It runs after a random delay of 1 to 30 minutes, at once when an anchor hub or the browser layer is late, or when you select **Check everything now**; the button then reads **Check again**. Private-event policies and upgrades are not public: obtain the policy and receipt through the authorised channel, check the included material offline, and ask the organiser for any missing opening or auditor attestation. Do not send another voter's private material to a public service.

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

---
id: auditor_transparency
title: Verify transparency receipts and evidence
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

The ballot verifier offers **Transparency receipt** beside the existing ballot audit, with a **Check a receipt** button. On **Check a transparency receipt**, open a receipt file (the `.vitrine-receipt` file that **Save receipt** saves in the voting portal), paste a receipt, its link or its reference, or follow the ballot tracker's **Check it independently** action. Check the trust policy's ID against the official notice before pinning it: the ID is 13 groups of four characters, the format of a Tracking ID, and the first four groups (80 bits) are the ones to compare. Tools compare all 13. A policy carried by a receipt link is a candidate, not an independently established trust anchor.

**Your ballot** shows the Tracking ID the page looks for, as the short ID and the full ID in groups of four. Provide the expected full Tracking ID, event and election when the receipt file does not already carry the entry the voter expects. The field refuses a short ID, and **Change the tracking ID** looks for another one. **Result** shows a headline and three steps: **In the public record**, **Confirmed by witnesses** and **Anchored publicly**. Each step shows what its checks found, such as the entry number, how many witnesses signed and how many the policy needs, and how many hubs hold the record.

Pending is distinct from a contradiction. A step that is still being checked is not a failure, and **Taking longer than expected** is not evidence of tampering. **The record does not match** means that validly signed objects contradict each other: **Save evidence** saves the receipt file that holds them, from which an auditor can build the evidence file with the Vitrine verifier. **Couldn’t be checked** means that some data looks damaged, and an invalid object alone is not cryptographic evidence of misconduct. Missing profile meaning metadata keeps the overall result pending, so the status does not reach **Verified**, while each step keeps its own result.

The first check fetches only from policy logs. The full check can contact mirrors, hubs and monitors. It runs after a random delay of 1 to 30 minutes, at once when an anchor hub or the browser layer is late, or when you select **Check everything now**; the button then reads **Check again**. Private-event policies and upgrades are not public: obtain the policy and receipt through the authorised channel, check the included material offline, and ask the organiser for any missing opening or auditor attestation. Do not send another voter's private material to a public service.

## Audited ballots on a hub's ballot verifier

When an event has transparency enabled, an audited ballot is not downloaded. The voting portal publishes the ballot and the secret it encrypted with, and the voter checks it on a hub. A hub whose policy has a `hub-ballot-verifier <hub> <url>` line runs the ballot verifier at that URL. The voting portal's audit screen links each such hub, in a random order, as:

```text
<url>tenant/<tenant>/event/<event>/election/<election>/audited#id=<full Tracking ID>&policy=sha256:<hex>
```

The Tracking ID and the policy ID are in the fragment, so the hub's server never sees them. To check an audited ballot yourself, open that link, or build it from a Tracking ID and the policy ID you compared with the official notice.

The page needs no login and no file. It uses the election's policy only if that policy is pinned in this browser, or is in the event's public config with the link's policy ID, and only if it covers the election. Then, in your browser, it:

1. finds the ballot's entries in the policy's `ballots` log by a bucket of 3 characters of the Tracking ID, so no server learns which ballot you asked for, and keeps the entry whose full Tracking ID is the link's;
2. proves the `audited` entry with the log's receipt and checkpoint, and reads the `events` log for successor policies;
3. fetches the ballot and its published secret from Step's public ballot locator, and checks that the ballot's hash is its Tracking ID and that the secret reproduces the ciphertext;
4. only then decodes the ballot with the election's published ballot design.

It never fetches or decodes a cast ballot. Each other outcome has its own message instead of choices: an incomplete link, a policy it cannot get, another election, no such ballot, a cast ballot, an audit not yet in the record, a ballot not yet published, a proof that does not check out, or a ballot that does not match its Tracking ID (**Audit failed**). The last two are evidence: preserve the link and the saved evidence file, and report them to the election authority.

The ballot tracker runs the same ballot checks: see [Track your Vote](../../03-voters/01-tutorials/02-voter_track_your_vote.md#an-audited-ballot). For how a hub sets up its ballot verifier, see the developers' [Vitrine page](../../07-developers/14-vitrine.md#a-hubs-ballot-verifier).

An event without transparency keeps the classic audit: the voter downloads the audited ballot and imports it into the ballot verifier with its full Tracking ID. See [Audit your Vote](../../03-voters/01-tutorials/03-voter_audit_ballot.md#audit-without-transparency).

## Offline checks

With the Vitrine CLI installed, the equivalent offline starting points are:

```sh
vitrine policy check policy.txt
vitrine policy id policy.txt
vitrine chain verify receipt.vrc --policy policy.txt --offline \
  --expect event_id=EVENT_ID --expect election_id=ELECTION_ID --expect ballot_id=BALLOT_ID
vitrine evidence verify evidence.txt --pin policy.txt
```

Replace `EVENT_ID`, `ELECTION_ID` and `BALLOT_ID` with the values you expect independently. `BALLOT_ID` is the full Tracking ID, as 64 hexadecimal characters. An offline check cannot establish fresh successor discovery, so some claims remain pending.

Use `vitrine --help` and each subcommand's `--help` for the installed version's input and output options. Evidence verification requires a separately pinned policy; a policy embedded in an evidence file does not establish trust. Preserve the original bytes of receipts, signed notes, policies and evidence when reporting a contradiction. The receipt establishes logged inclusion and consistency properties, not the correctness of a tally or the secrecy of a voter's choices.

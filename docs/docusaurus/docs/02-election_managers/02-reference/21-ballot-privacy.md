---
id: ballot_privacy
title: Ballot privacy
sidebar_position: 21
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

New events that use voter pseudonyms list ballots under random codes such as **7QKM-2HXD**, instead of the voter's identity. A voter keeps the same pseudonym within an election event. The private relationship between the voter and that code is separate from ballots and signed log entries. Historical events and events using the Datafix integration retain their recorded identity mode; the **Data > Privacy** section shows the actual mode, which cannot be changed after creation.

New pseudonym events require device-held voter signing keys and signed ballot receipts. Enable those prerequisites and disable telephone voting before publication; a channel without device signatures cannot use this privacy mode.

## Checking a ballot

When **Show Cast Vote Logs Tab** is enabled, the Voting Portal's **Logs** tab lists cast ballots with their Ballot ID, pseudonym, cast time and channel. The signed-in voter's own ballot appears separately at the top. Search by the beginning of either code. Results are paged from the last displayed ballot; large searches show a bounded match count and ask for more of the code.

Opening a ballot checks four things on the device:

1. The election event log key signed the relationship between the pseudonym and voter key in the Received entry.
2. The ballot box received a ballot signed by that voter key.
3. The Ballot ID follows from the received signed ballot.
4. The same voter key cast the ballot, and the ballot box confirmed the cast.

The browser uses published keys independently of the proof response. **Still being added to the log** means a required signed entry has not arrived yet; retry later. **A check failed** means available evidence did not verify; do not rely on that entry. A failed check takes precedence even when another entry is pending. Signed entries can be downloaded for independent verification. The existing receipt checks-period setting still applies.

In the Admin Portal, open a ballot entry in the event's **Logs** tab to run the same checks. Other audit explanations, filters and exports remain available. For pseudonym events, **Voters > User's Logs** contains the voter's sign-ins and messages rather than their ballots. Participation remains available without an individual ballot timestamp.

## Network data

**Data > Privacy > Network data** offers two policies:

- **Do not store**: no new network data is retained, and the IP Addresses list is hidden.
- **Keep for forensic review**: successful voter sign-ins and ballot receive and cast requests retain IP address and available country and browser data separately in the forensic store. The dashboard lists pseudonyms, never voter identities, and each read is recorded.

Sign-in capture runs after a successful authentication transaction. Keycloak checks the event policy before sending attribution directly to the internal Harvest service; Harvest checks it again before storage. Ordinary sign-in audit records and queues contain no network data. Failed sign-ins, tenant administrator sign-ins and voters without an existing pseudonym are not captured.

Sign-in attribution is best effort: an unavailable service or failed policy check can leave a gap, while the completed sign-in remains valid. A first sign-in after automatic voter registration may also arrive before its pseudonym has been allocated and leave no forensic record. No durable retry queue retains the network payload. The forensic store therefore does not establish a complete history of sign-in addresses during an outage.

Forensic attribution is available only for new events using voter pseudonyms. Historical and Datafix events retain their existing identity mode; their Privacy settings do not offer forensic storage.

The IP Addresses list requires the dedicated **Forensic Read** permission on the server and in the portal. Opening an address shows its ballot records and records another read. List controls include columns, filters, export, selection, row actions and pagination. An address marked **Shared** has ballots from several pseudonyms; this can happen on a shared office or hotel network and does not identify a voter.

No preset role grants **Forensic Read** or **Resolve a pseudonym**. Privacy settings show the role assignments returned by the service and link to **Users and Roles**. A failed permissions lookup is shown as an error rather than as an empty assignment list.

## Authorized resolution

There is no portal screen for resolving a pseudonym. An explicitly authorized operator can use the CLI with its configured endpoint and authentication token:

```sh
step-cli step resolve-voter-pseudonym \
  --election-event-id 11111111-1111-4111-8111-111111111111 \
  --pseudonym 7QKM-2HXD \
  --reason "Authorized election protest review"
```

The command requests the `voter-pseudonym-resolve` role and resolves one code. A reason is required, with at most 512 UTF-8 bytes and no control characters. Do not put voter identities or network data in the reason: it becomes part of the signed resolution log. The service records the requesting actor, pseudonym and reason; the returned voter ID goes only to the requester. Handle the command's output as private information.

## Retention and limitations

The identity mapping and forensic records have separate retention requirements. The election authority must approve those periods and the operational purge procedure, including replicas and backups. Changing to **Do not store** stops new retention and disables forensic reads, including reads of previously stored records. Existing records remain retained until an explicit purge; changing policy does not erase them or their backups. Public signed ballot evidence remains verifiable after the identity mapping is removed.

Operators invoke the internal Windmill `purge_vote_privacy(tenant_id, election_event_id, scope)` task with the authority-approved scope. It is not a public endpoint and has no default schedule or retention duration:

- `{"kind":"mappings"}` removes the event's private voter-to-pseudonym mappings. The first batch records a permanent tombstone that prevents replacement mappings from being created; ballot verification remains available.
- `{"kind":"network","before":"2026-01-01T00:00:00Z"}` removes network records older than the supplied UTC cutoff. The date is an example, not a recommended retention period.

Each invocation visits the active event database and every retained copy of that event, committing a separate transaction of at most 5,000 removals per copy. It returns the aggregate removed count; repeat the same scope until that aggregate is zero. The task verifies the tenant and retains purge evidence containing the scope, cutoff where applicable, and removed count. Operators must retain the batch results and separately apply the approved procedure to replicas, backups and any exported copies.

New voter receipt downloads require the requesting voter's authenticated access, and newly generated receipts use private storage. Historical public receipt objects and copies already downloaded are not retroactively erased by this change.

The approved vote confirmation email may include the Ballot ID. Someone with access to that mailbox or the email provider can therefore link that ballot to its recipient. SMS and chat confirmations must not include the Ballot ID. Historical signed logs retain their historical contents; the new privacy mode does not rewrite those signatures.

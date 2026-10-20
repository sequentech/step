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

## Auditing a ballot

Auditing reveals the ballot's choices and encryption evidence so they can be checked. Before displaying or downloading that evidence, the portal asks the ballot box to permanently mark the received ballot as audited. An audited ballot cannot subsequently be cast, including by returning to Review. To vote after an audit, prepare a new ballot. A ballot that has already been cast cannot be audited; the portal discards its audit material and keeps its non-disclosing confirmation and receipts.

## Recovering after additional authentication

For elections requiring additional authentication, the portal keeps the exact signed cast request in this tab for at most five minutes. If the cast response cannot be confirmed, Retry sends that same request. Additional authentication may be required again after one minute; it does not extend the saved request's deadline. A resolved login alone never confirms a vote. Expiry does not establish whether a previous cast reached the ballot box: check the Ballot ID before starting again. After expiry, this tab retains only the Ballot ID for the same signed-in voter, so **Check ballot** still works after reloading; the expired signed request cannot be retried. Editing the ballot or signing out clears the saved recovery information.

## Participation exports

Voter exports, including the voter CSV inside an event archive, and messages selected by voted/not-voted status require permission for every included election. The service saves the authorized election set with each task and refuses to expand it when the task runs or retries. Downloading the generated document checks current permissions again. Old participation jobs and exports without a saved authorization set must be recreated by an authorized operator.

## Network data

**Data > Privacy > Network data** offers two policies:

- **Do not store**: no new network data is retained, and the IP Addresses list is hidden.
- **Keep for forensic review**: successful voter sign-ins and ballot receive and cast requests retain IP address and available country and browser data separately in the forensic store. The dashboard lists pseudonyms, never voter identities, and each read is recorded.

Sign-in capture runs in a bounded background worker after a successful authentication transaction, so service calls do not hold the login response. Keycloak checks the event policy before sending attribution directly to the internal Harvest service; Harvest checks it again before storage. Ordinary sign-in audit records and queues contain no network data. Keycloak’s configured audit database also stores a sanitized copy for event realms while retaining normal audit queries and expiry. Failed sign-ins, tenant administrator sign-ins, unpublished events and events whose private mapping has been purged are not captured.

New election-realm login sessions, including offline sessions, also omit the IP address under both policies. Upgrading from a version that retained addresses requires the operator to invalidate existing election-realm sessions during maintenance, following the [session cleanup procedure](../../07-developers/06-keycloak/developers_keycloak.md#existing-election-realm-sessions). Affected voters sign in again. Historical backups remain subject to the authority's retention procedure.

Temporary service failures and a pending voter registration are retried with the same delivery identifier; each attempt checks the current policy again. Retries stop if storage is disabled. Delivery uses bounded process memory, with no durable queue containing network data. A prolonged outage, exhausted capacity or a process restart can still leave a gap, while the completed sign-in remains valid. The forensic store therefore does not establish a lossless history of sign-in addresses.

Forensic attribution is available only for new events using voter pseudonyms. Historical and Datafix events retain their existing identity mode; their Privacy settings do not offer forensic storage.

The IP Addresses list requires the dedicated **Forensic Read** permission on the server and in the portal. Opening an address shows its ballot records and records another read. List controls include columns, filters, export, selection, row actions and pagination. An address marked **Shared** has ballots from several pseudonyms; this can happen on a shared office or hotel network and does not identify a voter.

No preset role grants **Forensic Read** or **Resolve a pseudonym**. Privacy settings show the role assignments returned by the service and link to **Users and Roles**. A failed permissions lookup is shown as an error rather than as an empty assignment list.

## Authorized resolution

There is no portal screen for resolving a pseudonym. An explicitly authorized operator can use the CLI with its configured endpoint and authentication token:

```sh
step-cli step resolve-voter-pseudonym \
  --election-event-id 11111111-1111-4111-8111-111111111111 \
  --pseudonym 7QKM-2HXD \
  --reason investigation
```

The command requests the `voter-pseudonym-resolve` role and resolves one code. The required reason must be one of `investigation`, `compliance-review`, or `test-validation`. The service records only this fixed category in the signed resolution log and rejects arbitrary text. Keep names, identifiers and detailed narratives in private case records. Existing clients that supplied prose must switch to an approved category; historical signed records keep their original bytes. The service records the requesting actor, pseudonym and reason; the returned voter ID goes only to the requester. Handle the command's output as private information.

## Retention and limitations

The identity mapping and forensic records have separate retention requirements. The election authority must approve those periods and the operational purge procedure, including replicas and backups. Changing to **Do not store** stops new retention and disables forensic reads, including reads of previously stored records. Existing records remain retained until an explicit purge; changing policy does not erase them or their backups. Public signed ballot evidence remains verifiable after the identity mapping is removed.

Operators invoke the internal Windmill `purge_vote_privacy(tenant_id, election_event_id, scope)` task with the authority-approved scope. It is not a public endpoint and has no default schedule or retention duration:

- `{"kind":"mappings"}` removes the event's private voter-to-pseudonym mappings. The first batch records a permanent tombstone that prevents replacement mappings from being created; ballot verification remains available.
- `{"kind":"network","before":"2026-01-01T00:00:00Z"}` removes network records older than the supplied UTC cutoff. The date is an example, not a recommended retention period.

Each invocation visits the active event database and every retained copy of that event, committing a separate transaction of at most 5,000 removals per copy. It returns the aggregate removed count; repeat the same scope until that aggregate is zero. The task verifies the tenant and retains purge evidence containing the scope, cutoff where applicable, and removed count. Operators must retain the batch results and separately apply the approved procedure to replicas, backups and any exported copies.

New voter receipt downloads require the requesting voter's authenticated access, and newly generated receipts use private storage. Historical public receipt objects and copies already downloaded are not retroactively erased by this change.

The approved vote confirmation email may include the Ballot ID. Someone with access to that mailbox or the email provider can therefore link that ballot to its recipient. SMS and chat confirmations must not include the Ballot ID. Historical signed logs retain their historical contents; the new privacy mode does not rewrite those signatures.

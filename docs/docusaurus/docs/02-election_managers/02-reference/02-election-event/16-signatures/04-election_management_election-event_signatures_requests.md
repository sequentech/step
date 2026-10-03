---
id: election_management_election_event_signatures_requests
title: Signing requests
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

**Signatures** > **Requests** lists every signing request of the election event, for
whoever follows the signing of all Posts. It needs `signing-requests-read`. Staff whose
roles carry permission labels see only the requests of their Posts.

## The list

| Column | Shows |
|---|---|
| Request | The action, the Post and the country, for example "Election returns · Post A · Country B". |
| Status | The status and the count, for example "Waiting · 1 of 3" with the expiry time, "Signed · 2 of 2", "Expired · 1 of 2", or "Cancelled" with the reason. |
| Started | When the request was started. |
| By | Who started it. |
| Last signature | Who signed last, and when. |
| Code | The signing code. |

Filter by status. Selecting a row opens the same request panel the signers use, with the
document or details, the code and each signer's status (see
[Signing a protected action](./05-election_management_election-event_signatures_signing.md)).

## Statuses

| Status | Meaning |
|---|---|
| Waiting | It needs more signatures. It can be signed until it expires. |
| Signed | Every signature is in and the action is running. |
| Done | The action ran. |
| Failed | Every signature is in, but the action failed. The signatures stay recorded; the [Logs](../11-election_management_election-event_logs.md) have the details. Start the action again. |
| Expired | Nobody signed in time. Its signatures no longer count. |
| Cancelled | It was cancelled, with one of the reasons below. Its signatures no longer count. |

A request is cancelled when:

| Reason | When |
|---|---|
| The person who started it cancelled it | The requester used **Cancel request**. |
| An operator cancelled it | Someone with `signing-requests-cancel` used **Cancel request**. |
| The action's signing rule changed | Someone saved the action's rule while it was waiting. |
| What it signs changed | For example a recount changed the election returns, or a new configuration version was generated. |
| A newer request replaced it | The same action was started again for the same Post and subject. |
| The certificate that signed it was revoked | A certificate counted in it was revoked. |

## Cancelling a request

Open the request and choose **Cancel request**, with an optional reason. The person who
started a request can always cancel it; anyone else needs `signing-requests-cancel`. Use
it for a request that is stuck, for example when a signer is unavailable. The people
involved start the action again.

## Exporting

**Export CSV** (`signing-requests-export`) downloads the election event's requests that the
user can see, whatever the status filter shows. The export is logged as `SigningRequestsExported`, with the number of rows and the file's
hash.

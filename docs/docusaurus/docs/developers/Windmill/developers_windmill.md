---
id: developers_windmill
title: Developers Windmill
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

## Ballot receipt tracker links

Windmill builds receipt tracker links from `VOTING_PORTAL_URL` and the ballot's
authorized tenant, election event, election and ballot ID. Configure an absolute
HTTP(S) base URL without credentials, query parameters or a fragment; a path
prefix and trailing slash are supported. Both portal settings ignore surrounding
whitespace and report their setting name when validation fails.
When `KIOSK_VOTING_PORTAL_URL` is set,
a receipt requested from that origin uses the configured kiosk base instead.
Selecting a distinct configured kiosk origin also restores the fixed `?kiosk`
login flag for older portal requests that omitted their query string.

The existing `ballot_tracker_url` request and task fields remain compatible,
including for queued tasks, but only select a configured kiosk origin and the
fixed `?kiosk` login flag on a configured portal origin. Other client paths and
query parameters are ignored. Receipt templates still receive the
`ballot_tracker_url` variable as a complete link. Existing deployed and custom
templates need no migration, and all PDF renderer backends receive the same
server-built link. Restart Windmill workers after updating the code or these
URL settings.

# Tally

## Discarded/Auditable Ballots

Discarded Ballots that for some reason are not included in the tally. Possible reasons:

- The voter was disabled.
- The voter was deleted.
- The ballot was cast outside the Voting Period.
- The voter is not authorized for the election of the ballot.
- The voter is not assigned to the area of the ballot.
- The ballot is from a previous revote (only the last vote counts). (Note, this is not counted as a discarded ballot yet).

## Eligible Voters

Eligible voters are voters that can vote. Voters not included here are disabled and deleted voters.
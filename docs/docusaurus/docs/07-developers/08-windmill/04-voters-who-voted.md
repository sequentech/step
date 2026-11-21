---
id: windmill_voters_who_voted
title: List of voters who actually voted
sidebar_position: 4
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

# List of voters who actually voted

The `OV_WHO_VOTED` report type
(`windmill/src/services/reports/ov_who_voted.rs`) lists, for every Post
(election) of an event or the one its report names, the voters who cast an
accepted ballot, by area. It writes a PDF and a CSV from one in-memory list,
read once per generation, so both formats hold the same voters in the same
order. The type name is the one release/9.0 stored, so its report rows import.

## Who is listed

| Event identity mode | Source of the voters | Accepted means |
| --- | --- | --- |
| `voter-id` | `PostgresStore::accepted_voters`: distinct `voter_id` of the Post's ballots with status `valid` | `valid` |
| `pseudonym` | `PostgresStore::participating_voters`: the private participation flags mapped to voter IDs | flags equal to the Post's valid ballots |

Neither read selects a ballot time, channel, content or ballot ID. The
pseudonymous path uses only the privacy adapter's participation reads; it does
not join the pseudonym mapping to ballots.

A participation flag is set by a `valid` or `pending` ballot and is never
removed, and every valid ballot sets one. `check_population` therefore
compares three counts per Post: the distinct identities with a valid ballot
(`participation`), the flags (`privacy_participation_count`) and the voters
named. Unequal counts fail the generation with a message instead of printing a
list that would name voters whose ballot was not accepted, or omit voters after
a purge. `identity_mapping_purged` refuses a purged event up front.

## Areas and counts

- A voter is placed in the first area, by name, of the Post's areas their
  Keycloak account carries (`area-id`), as in 2025. Voters in none of them are
  listed after the areas, with no area.
- `registered` per area is `count_keycloak_enabled_voters_of_areas`: enabled
  voters of the area authorized for the Post, as the tally's electorate.
  `not_voted` is `registered` minus the enabled voters listed; `total` adds the
  voters who voted with an account disabled since.
- Accounts are read with `fetch_voter_records` in batches of 1,000 IDs: names,
  username, `enabled`, `area-id`, `middleName` and `suffix`; no other attribute.
- Order: Posts by title, areas by name, voters by name ignoring case, then by
  exact name, username and voter ID, so equal names list the same way each time.

## PDF parts and CSV

`renders_pdf_in_batches` prints one PDF when the list fits in
`max_items_per_report` voters (default 1,000; at least 1) and parts otherwise,
delivered in a ZIP; each part keeps its areas' totals and says which rows it
holds. The CSV (`write_format`) writes every row with `csv_export_writer`, so
a signed configuration's stamp line comes first. Every cell goes through
`defuse_formula`. Its columns are not COMELEC's schema, which is still an open
decision (VOTE-REPORTS D3); SQL output is not offered.

## Access

`/generate-report` authorizes the tenant the request names, needs
`report-generate` (real) or `report-preview` (preview), refuses a request whose
event is not the report's, and for this type in real mode also needs
`voter-read`. A report without a Post needs every Post's permission label. The
list is never mailed (`should_send_email` is false) and never scheduled:
`authorize_scheduled_report` answers `NotSchedulable` for the type, so the
scheduler turns off an active `cron_config` (from the API or an imported
release/9.0 row) with a failed task instead of generating the list without the
Reports tab's `report-generate` and `voter-read` checks.

## Scripts the PDF can print

The PDF is drawn by the renderer's Chromium, whose images install only the
DejaVu fonts. Latin (with accents), Greek and Cyrillic names print; Chinese,
Japanese and Korean characters print as boxes. The CSV keeps every character.
`test_f_voters_who_voted` pins this: it checks with `pdffonts` that no CJK
font is embedded (`CJK_FONT_MARKERS`) and that the CJK voter's CSV row is
exact. Adding a CJK
font (for example `fonts-noto-cjk` in the Chromium layer of the Windmill and
E2E images) is a platform change for every report; update that test and the
user documentation with it.

## Availability in pseudonymous events

The guard fails closed. Participation flags are never removed and a rejected
ballot never becomes valid again, so one voter whose only ballot was rejected
(or is still pending) makes that Post's list, and therefore an all-Posts list,
unavailable for good. Lifting this needs a sanctioned accepted-participation
read in the privacy adapter (a VOTE-PRIVACY decision), not a join between the
pseudonym mapping and ballots. The three counts are read on separate
connections, so a ballot cast during the read can also refuse a generation;
generating again resolves that case.

## Tests

```sh
# From packages/, with the synthetic PostgreSQL of the Windmill test profile.
cargo test -p windmill --lib -- ov_who_voted --include-ignored
cargo test -p electoral-log --test ballot_box the_voters_who_voted -- --ignored
cargo test -p electoral-log --test privacy participation_flags_cover -- --ignored
cargo test -p harvest --bin harvest -- routes::reports::route_tests
```

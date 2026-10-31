---
id: regroup-voters-by-attribute
title: Regroup voters by attribute for a new tally
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

:::warning Not yet validated end to end on v10.1.0-rc.4

The procedure below is based on reading the `v10.1.0-rc.4` source. The script
has only been run against its focused tests
(`python3 scripts/test_regroup_voters_by_attribute.py`: 36 tests, OK, using
in-memory Hasura and Keycloak). A full run on an rc.4 stack (baseline tally,
regroup, second tally) is still pending. Do not use it on a production event
until this notice is replaced with that evidence.

:::

After voting has closed, an election event that used one area can be tallied
again with one result per value of a voter attribute, for example a "Service
Unit". `scripts/regroup_voters_by_attribute.py` creates one area per value,
moves each voter and **all of that voter's existing cast votes** to that area,
and checks that nothing else changed. A ballot publication and a new tally
then use the product's normal flow. No voter casts again and no release needs
to be deployed. The procedure was traced against `v10.1.0-rc.4`.

## What changes and what does not

| Changes | Does not change |
| --- | --- |
| New areas named exactly after the attribute values, with the source area's contests | Cast vote content, ballot id, status, voter, timestamps (only `area_id`) |
| `cast_vote.area_id` of the regrouped voters' votes, every status | Signed ballot styles (`ballot_eml`, `ballot_signature`) of earlier publications |
| The Keycloak `area-id` attribute of the regrouped voters | Bulletin board batches, and electoral log entries, which keep the area of each cast |
| After publishing: the previous publication's `deleted_at` (standard republication) | Earlier tally sessions, their contests, executions and results rows |

The new tally draws its contest areas and electorates from the live published
ballot styles, which is why a new publication is required. It reads each voter's
ballot from `cast_vote.area_id` joined to the voter's Keycloak `area-id`, which
is why both are moved. It appends new batches to the board, numbered after every
existing one.

The earlier tally's rows, results, documents and board batches are kept and
checked unchanged. That session must **not** be re-executed after the regroup:
it points at the source area, which no longer holds the moved votes, and at the
live ballot styles.

## Limits to agree with the client first

- **Secrecy of small groups.** A per-area result for an area with few voters
  can reveal how those voters voted, and comparing it with the earlier
  single-area result can isolate one voter. The script lists every area with
  fewer than 5 voters who voted. The windmill tally logs a warning for weighted
  batches with fewer than 5 ballots.
- **Missing values.** By default a voter without a value refuses the whole
  plan. Fix the census, or pass `--blank-policy keep-in-source-area`, which
  leaves those voters and their votes in the source area. This is a partial
  grouping: the source area is then reported as one more group. The script never
  invents an area name.
- **Values are used exactly.** Values with leading or trailing spaces, several
  values for one voter, or values that differ only by case refuse the plan.
- **Turnout and rounding.** Per-area census and turnout count the voters
  assigned to each area. Percentages are rounded per area and need not add up.
- **Adding areas back up.** For plurality-at-large contests (the only
  algorithm voter-weighted voting allows), each candidate's count is an integer
  sum of the weights of the voters who chose it. Per-area counts therefore add up
  exactly to the earlier single-area count, and so do the blank and invalid counts.
  Winners are decided per area. This does not hold for methods whose result is
  not a sum of per-ballot counts, such as instant runoff, and is not claimed
  for them.
- **Each voter's own data.** Only `area-id` changes. `verify` checks that every
  regrouped voter's enabled flag, username, weight, authorized elections and
  every other attribute are unchanged.

## Preconditions (maintenance window)

- Voting is closed on every channel of the event and of every election.
- No tally session is running, no cast vote is `in-progress`, and nobody
  edits voters, areas or publications until `verify` reports `READY_FOR_TALLY`.
  The script checks this state when it starts and guards every move, but it
  cannot lock Keycloak. Between reading a voter and saving it, a concurrent edit
  to that voter could still be lost. The maintenance window, with admin access
  restricted, is what prevents this; the guards only detect most conflicts.
- The source area holds every voter to regroup. No other area may use one of
  the new names.
- Access to the Hasura endpoint with its admin secret, and to the Keycloak
  admin API (an admin client or the admin user of the admin realm).

## Procedure

Keep the plan and journal files private. They contain voter ids and attribute
values, but no names or contact details.

```bash
export HASURA_ADMIN_SECRET=...                  # never on the command line
export KEYCLOAK_ADMIN_CLIENT_ID=... KEYCLOAK_ADMIN_CLIENT_SECRET=...
# or: export KEYCLOAK_ADMIN_USER=... KEYCLOAK_ADMIN_PASSWORD=...

regroup() {
  python3 scripts/regroup_voters_by_attribute.py "$1" \
    --hasura-endpoint https://hasura.example \
    --keycloak-url https://keycloak.example \
    --tenant-id <tenant> --election-event-id <event> \
    --source-area-id <area> --attribute service_unit \
    --plan regroup-plan.json --journal regroup-journal.json "${@:2}"
}

regroup plan      # read-only: validations, intended counts per area, warnings
regroup apply     # refuses if anything changed since the plan
```

`apply` first writes the journal with `O_EXCL` and `fsync`. The journal holds
the reviewed plan and hashes of every pre-existing tally, results, publication,
ballot style and cast vote row. Only then does `apply` make changes:

1. One Hasura request, run as one transaction, inserts the areas and their
   contests and moves the cast votes. Each move only applies to a vote still in
   the source area. If any count differs, apply stops before touching Keycloak.
2. Each voter's `area-id` is updated only if it still held the source area
   when it was read. The full user is saved and read back, and every other
   attribute is checked unchanged. Keycloak has no conditional update, so this
   narrows, but does not close, the window for a concurrent edit.
3. `verify` runs (see below).

Then, in the Admin Portal, **generate and publish** a ballot publication for
the event, and run `regroup verify`:

- `PUBLICATION_PENDING` (exit 3): the live publication lacks some (election,
  area, contest) combination of the new areas.
- `READY_FOR_TALLY` (exit 0): every voter and vote is in its area, the actual
  per-area counts equal the plan, the earlier rows are unchanged, and the
  publication matches exactly.

Start a new tally. Each area then has its own results.

Running `plan` again after a successful `apply` shows nothing left to do.

### Recovery

Keep the journal written by the first `apply` unchanged. It is the only
snapshot taken before the first change. Do not start over with a new plan and
journal after a partial apply, because a later snapshot would hide anything
that changed in between.

- **Finish an interrupted apply:** `regroup apply --resume`, with the same
  `--journal`. It only performs moves recorded in that journal, exactly as
  recorded, and refuses anything else. It then verifies against that original
  snapshot. Later `regroup verify` runs also use the original journal.
- **Undo:** `regroup revert`, with the same `--journal`. It is allowed only
  while voting is closed, no tally is running, and no tally session has been
  created since the apply. The baseline tally that existed before is fine. Once
  a new tally exists, moving its inputs back would make that session unsafe to
  re-run, so revert is refused and any recovery needs its own review. Revert
  moves voters and votes back only where nothing else changed them, verifies
  the result, and otherwise fails and lists what it left alone. Areas created by
  `apply` stay in place, so publish again before any tally.

Exit codes: 0 done, 2 refused (with reasons), 3 publication pending, 1
connection or file error.

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# Ceremony board mapping

`audit-tally` and `export-ballot-audit` support shared/default and per-election keys ceremonies on restored local PostgreSQL copies.

For a default ceremony (`is_default` true or null), B3 and ImmuDB use the saved election-event board reference. For a non-default ceremony, supply `--environment-slug` from independently authenticated deployment metadata. The commands cross-check that slug against the saved event board, require exactly one event election belonging to the ceremony, validate tenant/event/election ownership, and derive the election B3 board using the same naming rule as Windmill. The event board remains the electoral-log database. The tally plan records both names.

Example extra option for either command:

```sh
--environment-slug '<authenticated deployment ENV_SLUG>'
```

The saved ceremony/election rows, deployment slug, scope and public-key pins still require authentication outside the exported bundle. Inferring a key or slug solely from the same untrusted data checks self-consistency only. This change does not rewrite signed messages, keys, batch numbers or source IDs. Existing native signature, public-key, batch, history and eligibility checks remain in place.

`audit-tally` plans the supplied tally-session-contest rows. Independently reconcile that inventory with all expected election/area/contest pairs before treating a successful run as complete coverage. Braid verification includes all Ballots batches present on the selected B3 board, including prior tallies. ImmuDB row-ID gaps are not in themselves evidence of missing records.

Run the unit and file-based integration suite with:

```sh
cd packages
cargo test --locked -p ballot-audit
```

The ignored PostgreSQL export integration test requires three fresh disposable databases and `BALLOT_AUDIT_RUN_SYNTHETIC_DB_TESTS=1`; it covers default and per-election boards, preserved native bytes, missing/wrong slug rejection, wrong-board rejection, scoped queries, and read-only snapshots. Never run this synthetic fixture against production or restored election data.

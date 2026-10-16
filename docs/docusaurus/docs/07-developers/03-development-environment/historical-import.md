---
title: Historical SQL and workbook import
---

<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

The Admin Portal event-import drawer and `step-cli step build-election-event` share
the pure Rust `legacy-seat-v1` adapter. It reads a data-only MySQL dump together
with a supplemental XLSX workbook and feeds the existing builder, validator and
native archive writer. It does not execute SQL, evaluate views, contact source
databases or use an AI service.

## Opt-in offline Admin plugin

The historical converter appears only when the Admin Portal's runtime
`global-settings.json` sets `ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL` to a
nonempty extensionless base URL. There is no enabled-by-default fallback. The
standard native ZIP import remains available when the setting is absent, blank,
or the converter fails to load. Historical SQL/XLSX files never enter the native
upload path.

The loader follows `IVR_EMULATOR_BASE_URL`: it strips query/fragment and loads
`<base>.js` plus `<base>_bg.wasm`. It checks the initializer, conversion export
and Rust-exported handshake: ABI 1, profile `legacy-seat-v1`, package version
`0.1.0`. Unsupported or missing exports/version fail before conversion.
Configured JS is trusted deployment code; the version check validates
compatibility and does not authenticate the plugin.

The production build copies the code-only package into local static assets.
For an offline demo served by the Admin web server, use this configuration:

```json
{
  "ELECTION_EVENT_IMPORT_CONVERTER_BASE_URL": "/wasm/election_event_import_converter"
}
```

The build supplies `/wasm/election_event_import_converter.js`, its `_bg.wasm`
and `/historical-import-worker.js`. Conversion requires no Internet or S3 access.
Do not point this setting at a source SQL/XLSX file. The normal backend upload
and import still require the existing backend services.

The normal Import Event side drawer has one drag/drop zone, with multi-file
browsing available from that same zone. File extensions route the selection;
the Rust adapter verifies the actual SQL profile and required workbook tabs.
File count alone never identifies a format.

1. Drop a native ZIP (or existing native JSON/encrypted archive), or the original
   SQL and supplemental XLSX together or separately, in either order. Duplicate
   source types in one drop, mixed native/source files and unsupported files are
   refused. Dropping a replacement source invalidates the previous result.
2. A complete source pair automatically converts and validates in a Web Worker.
   Read the local table/sheet/row/field diagnostics. Source hashes, counts and
   exclusions are available under **Source details and exclusions**. Local
   errors retain files for retry or replacement. No backend request is made.
3. A valid local package enables **Create**. That explicit action uploads only
   the generated ZIP, requests backend `checkOnly` validation, and submits the
   separate creation request only after that validation succeeds. A rejected
   dry run blocks creation; transport failures remain retryable.
4. The import task reports final asynchronous success or errors. A passing dry
   run is not a guarantee of creation. Cancel/reopen starts a fresh local
   selection; a late result from a cancelled/replaced conversion is ignored.

The native archive path retains its existing upload, validation and final
Import action. An unconfigured historical plugin does not load converter assets.

Plugin loading/ABI errors, local source diagnostics, upload transport,
backend dry-run and final creation are displayed separately. Unexpected runtime
exceptions are sanitized instead of logging source values. SQL/XLSX originals
and source templates are not embedded or generated in the ZIP; keep them
separately in controlled storage.

## Source contract

The versioned profile reads these base tables: `allbgy`, `allmun`, `allprov`,
`candidate`, `canvass_center`, `contest`, `district_type`, `geographical`,
`party_list`, `polling_centers`, `pop`, `ref_typeboard`, `seat`,
`trans_route` and `voters`. It records view names without evaluating their
definitions. The restricted reader recognizes the dump's table definitions,
literal inserts and inert mysqldump envelope. Unknown statements, computed
insert values, malformed rows and unsupported escaping modes are refused.

Literals remain bytes, preserving leading zeros and binary picture content.
Mapped text uses declared UTF-8 or MySQL latin1 encoding. Binary content remains
in the unmodified original; this profile does not publish candidate photographs
from opaque blobs.

The workbook contains `ElectionEvent`, `Elections`, `Areas`,
`ScheduledEvents` and `CcsServers`. Blank styling rows are not records.
Election aliases must be unique because schedules refer to aliases. Several
election rows may share a post; they remain separate contexts.

| Relationship                   | Rule                                                                                     |
| ------------------------------ | ---------------------------------------------------------------------------------------- |
| Precinct to polling center     | `pop.POLLCENTER_CODE = polling_centers.ID`                                               |
| Precinct to barangay           | `pop.CLUSTERPOLLCENTER = allbgy.ID_BARANGAY`                                             |
| Precinct to municipality       | Concatenated province/municipality codes match `allmun.ID_CITY`                          |
| Precinct to transmission route | Province + municipality + literal zero + barangay code match `TRANS_SOURCE_ID`           |
| Seat to contest                | `seat.CONTEST = contest.POSTNAME`                                                        |
| Candidate to seat              | Both district code and contest post code must match                                      |
| Candidate to party             | Affiliation code matches `party_list.CODE_PARTY`, unless explicitly preserved unresolved |
| Election context to seats      | Explicit source-row associations when a post has several contexts                        |
| Area to precinct               | Exact historical name rule, or explicit workbook/source-row associations                 |
| Schedule to election           | Unique workbook alias; original instant or explicitly reviewed UTC offset                |

## Historical profile defaults

The drawer uses the checked-in
`packages/admin-portal/src/services/historical-profile-defaults.json` preset:
configuration-only census (no voter import), seat plurality at large (minimum
zero, maximum choices and winners from `seat.ELIGIBLEAMOUNT`), complete candidate,
post and precinct coverage, and known party references. These are declared
profile policies, not deductions from file count or election names.

There is no default schedule timezone or seat/area remapping. Dates without a
timezone, ambiguous repeated post contexts, missing coverage and unresolved
references produce blocking diagnostics. Correct the source workbook, or use
the CLI with explicitly reviewed, source-hash-bound associations when necessary.
The drawer never picks the first context or invents essential election settings.
Voter exclusion is reported in the summary and stored in provenance.

The advanced CLI decisions contract remains available: limited post/precinct
coverage, explicit associations and preserving unresolved affiliation codes
require both source SHA-256 values. Changed bytes invalidate those decisions.
No source policy controls or separate file pickers appear in the drawer.

## CLI

From `packages/`, using a built CLI:

```sh
step-cli step build-election-event \
  --workbook /private/input/supplement.xlsx \
  --historical-sql /private/input/source.sql \
  --historical-decisions admin-portal/src/services/historical-profile-defaults.json \
  --out /private/output \
  --check-only
```

Run without `--check-only` to write a validated package. `--strict` also refuses
warnings. The existing workbook-only command remains available.

The command above uses exactly the drawer preset. For advanced CLI conversion,
supply a separate reviewed decisions file. Such a file uses these fields; replace the hash placeholders with the
displayed hashes and review the row associations against the original files:

```json
{
  "census": "configuration-only",
  "ballot": "seat-plurality-at-large",
  "scope": "require-complete-coverage",
  "postCoverage": "require-complete-coverage",
  "precinctCoverage": "require-complete-coverage",
  "affiliation": "require-known-party",
  "scheduleOffsetMinutes": 480,
  "sourceDigests": {
    "sqlSha256": "<original SQL SHA-256>",
    "workbookSha256": "<original XLSX SHA-256>"
  },
  "electionMappings": [{ "workbookRow": 2, "seatRows": [1] }],
  "areaMappings": []
}
```

Workbook rows use spreadsheet row numbers, including the header. SQL source rows
are one-based per base table, in original INSERT order. Optional coverage mode
`workbook-posts-only` limits the corresponding post or precinct scope to
workbook matches and reports excluded rows. Optional affiliation mode
`preserve-unresolved-code` retains the source code without inventing party names.

The source, decision and profile provenance is carried in the event's
`import:provenance` annotation. The native ZIP contains the existing event JSON
and scheduled-event CSV members. Configuration-only conversion emits no voter
CSV. Keep original source files in controlled local storage; hashes do not
replace the originals.

## Verification and limits

The release helper also initializes the generated WASM and checks a synthetic
rejection through its exported API before packing. It requires Binaryen 110 or
newer; the repository Nix environment provides the build tools.

Synthetic tests cover literal escaping/binary bytes, active SQL rejection,
keys/references, composite contexts, coverage, stale decisions, annotations,
timezone handling and byte-identical archives. Frontend tests cover automatic local
conversion, invalidation on source change/cancellation, both drop orderings,
source privacy, the explicit Create boundary, and the existing native path.

The SQL source limit is 128 MiB with bounded token and row counts. The workbook
limit is 32 MiB compressed, 64 MiB of declared expanded members and 1,024 members.
These are implementation limits, not a capacity certification.

Historical compatibility does not establish compatibility with a later official
EMS schema, resolve no-remapping procurement requirements or authorize live
imports. A test association scenario demonstrates mechanics; issuer-approved
seat, area, census and ballot semantics remain a separate decision.

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
non-string (including `null` or `false`), or the converter fails to load. Omit
the key or set it to `""` to disable conversion. The key is read from the
Admin runtime JSON through `SettingsContextProvider`; it is not a separate
build-time environment flag. Historical SQL/XLSX files never enter the native
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

Historical conversion waits for loaded runtime settings and passes `APP_VERSION`
to the builder as `baseExport.version`. Use the deployment's version (for example,
`v11.0.0-rc.0`), or `dev` for a development deployment. A missing, placeholder or
malformed version blocks conversion before reading the source files; it never
falls back to the standalone converter's version. Reload the page after correcting
`global-settings.json`. Changing the runtime version invalidates any prior
conversion and rebuilds from the retained sources. Native archives keep their
existing embedded version and backend compatibility check.

The normal Import Event side drawer opens with the same native-import UI
whether the plugin is enabled or disabled. There are no initial converter
controls or policy settings. Its one drag/drop zone accepts multiple source
files when configured, with browsing available from that same zone. File extensions route the selection;
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
3. A valid local package reveals a three-dot **Converted ZIP options** menu
   with optional **Download ZIP**. It downloads the same accepted File that
   Create will upload, preserving its filename, `application/zip` MIME type
   and bytes. Download can be repeated and does not upload, validate with the
   backend, or create an event. A download failure leaves Create available.
   Replacing sources, clearing configuration, or closing/reopening invalidates
   the prior download. No download is offered for native archives or invalid
   conversion results.
4. A valid local package enables **Create**. That explicit action uploads only
   the generated ZIP, requests backend `checkOnly` validation, and submits the
   separate creation request only after that validation succeeds. A rejected
   dry run blocks creation; transport failures remain retryable.
5. The import task reports final asynchronous success or errors. A passing dry
   run is not a guarantee of creation. Cancel/reopen starts a fresh local
   selection; a late result from a cancelled/replaced conversion is ignored.

The native archive path retains its existing upload, validation and final
Import action. An unconfigured historical plugin does not load its UI module, worker or
converter assets. A configured plugin is loaded lazily after a recognized
source selection; the initial drawer and ordinary native imports do not load it.

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

Names and descriptions are kept per language in `presentation.i18n.<lang>`.
`<lang>` is the row's `presentation.language_conf.default_language_code`, or
`en` when the row sets none:

- Event and election names go to `presentation.i18n.<lang>.name`. A plain
  `name` column is accepted and moved there.
- A plain `description` is copied to `presentation.i18n.<lang>.description`,
  which is the field the Admin Portal edits. The plain column is kept.
- Contest and candidate names are `seat.CONTEST` and `candidate.NAMEONBALLOT`,
  exactly as the source spells them, in their election's language.
- Translations the workbook already gives in other languages are kept. None are
  added.
- A row that gives a plain and a localized value must give the same text in
  each, or it is refused with `conflicting_columns` at that cell.
- An event or election with no name in its default language, or a blank contest
  or candidate name in the SQL, is refused with `missing_field` where it is.

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

Candidate references are resolved against the whole source before any seat is
selected, whatever the scope policy. A `candidate.POST_CODE` with no
`contest.POSTCODE`, a `candidate.DISTRICTCODE` with no `seat.DISTRICTCODE`, a
referenced seat whose `CONTEST` names no contest, and a seat that belongs to a
different contest than the candidate's `POST_CODE` are blocking errors naming the
source table, row, column and the offending code (`dangling_reference` or
`conflicting_columns`, with `reference` in the problem details). Only a
candidate whose contest and seat both resolve can be counted as outside the
configured seats under `configured-seats-only`.

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

The command above uses the drawer's historical policy preset. The drawer additionally
supplies its loaded runtime version through `baseExport.version`. For advanced CLI conversion,
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

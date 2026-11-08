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

The same records also arrive in the documented post-qualification schema, which
renames some tables and columns. It is recognized by its table names, mapped onto
the names above and converted the same way. The summary and provenance record
which schema was read (`sourceSchema`: `legacy` or `post-qualification`), and
every problem names the table and column as the dump spells them. A dump that
mixes both sets of table names is refused, as is a table containing both names
of a renamed column, even when their values agree.

| Post-qualification name | Read as                    |
| ----------------------- | -------------------------- |
| `candidates`            | `candidate`                |
| `contests`              | `contest`                  |
| `seats`                 | `seat`                     |
| `seats.NUM_SEAT`        | `seat.ELIGIBLEAMOUNT`      |
| `seats.PROC_CCS_CODE`   | `seat.CCS_CODE`            |
| `political_party`       | `party_list`               |
| `canvass_centers`       | `canvass_center`           |
| `trans_routes`          | `trans_route`              |
| `trans_routes.TYPE`     | `trans_route.MACHINE_TYPE` |
| `pop.VOTING_MODE`       | `pop.MACHINE_TYPE`         |
| `jurisdictions`         | `geographical`             |

`trans_servers`, new in that schema, is accepted and not read. Its voters carry a
mail country and embassy instead of a country, embassy and precinct; see the
voter area joins below.

Literals remain bytes, preserving leading zeros and binary picture content.
Mapped text uses declared UTF-8 or MySQL latin1 encoding. Binary content remains
in the unmodified original; this profile does not publish candidate photographs
from opaque blobs.

The workbook carries everything the dump does not. It contains `ElectionEvent`,
`Elections`, `Areas`, `ScheduledEvents` and `CcsServers`, and optionally
`Contests`, `Parameters` and any other sheet an authoring workbook takes. Blank
styling rows are not records.

- **`Contests`** gives contests the columns the dump lacks, such as
  `description` and `presentation.i18n.<lang>.description`. A row without a
  `name` applies to every contest; a row with a `name` (the source's
  `seat.CONTEST`) to that contest, after the defaults. `{max_votes}` in a value
  is the contest's seat count, so `Vote for no more than {max_votes}` describes
  every contest. What the seats decide (`external_id`, `election.external_id`,
  the vote counts, `voting_type`, `counting_algorithm`, the `miru:` annotations)
  is refused.
- **`Parameters`** goes to the build unchanged, as in an authoring workbook. It
  carries the Keycloak realm, which the dump has nothing about: a whole realm,
  as an event export's `keycloak_event_realm` holds it, under the key
  `keycloak_event_realm`, split across `value+` continuation cells. The build
  patches it the way it patches a base export's: the realm's name and its URLs
  take this event's id, `displayName` follows the event name, `login_custom_css`
  becomes the login stylesheet in every enabled language, an `auth_type` applies
  its preset, and `keycloak_event_realm.<path>` rows set anything else. The
  importer then checks it against the deployment's template and generates new
  client secrets, as for any imported realm. Without a realm (and without a base
  export) those settings reach the realm patch beside the package instead.
- **`Templates`, `Reports`, `Admin Users`, `Permissions`** and the plan sheets
  (`Contacts`, `Trustees`, `Ceremony`, `Milestones`, `Messages`, `Notes`) go to
  the build unchanged. A `Reports` row names its election by `election_alias`,
  as a schedule does, in place of `election.external_id`.

A cell holds at most 32,767 characters, and a stylesheet can be longer. A column
headed `<column>+` directly after `<column>` (or after another `<column>+`)
continues it: a row's text there is joined onto the column's, left to right,
before the cell is read. This holds for every workbook sheet, and the workbook
writer splits longer text the same way.
Election aliases must be unique because schedules refer to aliases. Several
election rows may share a post; they remain separate contexts.

### Defaults workbook

A full workbook names one election per post and one area per cluster, so it fits
one dump. A workbook without an `Elections` sheet is a **defaults workbook**: it
carries only what the dump does not hold, and the elections and areas are
derived from the dump, so the same workbook serves any dump of that source.

- **Elections**: one per polling center that a `pop` row places voters in,
  named and aliased by its `POLLING_PLACE`. An `ElectionDefaults` sheet gives
  elections their other columns, `name` included. A row without a
  `polling_place` applies to every election; a row with one applies to the
  elections whose polling place it matches, as a whole name or a pattern with
  one `*` (`*-T`, `* PE`), and the most specific match goes on top: a whole name
  before a pattern, then the pattern that spells out more. Two equally specific
  matches, a second row for the same pattern, and `alias` or `election_post`
  columns are refused; a pattern that matches nothing is not, since one defaults
  workbook serves every dump. `{place}` in a value is what the `*` stood for,
  written as a voter reads it (`ABU DHABI` becomes `Abu Dhabi`), or the whole
  polling place as its embassy label reads. The alias stays the polling place,
  because schedules name elections by it; `presentation.i18n.<lang>.alias` is
  the alias voters see.
- **Areas**: one per precinct cluster (`pop.CLUSTERPOLLCENTER`), named by the
  historical name rule below and described by the cluster's own polling place.
  Without its own polling center, every precinct row of that cluster must refer
  to the same post; conflicting posts are refused. An `Areas` row without a
  `name` gives its other columns (CCS server tags, trustee servers, station id)
  to every area; a row with a `name` gives them to that area only, after the
  defaults. A named row that matches no derived area or more than one, and a
  second row for the same area or for every area, are refused.
- **Schedules**: a `ScheduledEvents` row without an `election_alias` applies to
  every election; one with an alias (the post) to that election only.
- **Event and CCS servers**: the `ElectionEvent` and `CcsServers` sheets, read
  as in a full workbook.

Election and area associations name workbook rows, so they are refused with a
defaults workbook. The summary and provenance record `defaultsWorkbook: true`.

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

### Preserving the source voters as the census

Census policy is an enum with three reviewed options:

- `require-reviewed-mapping` (default) — refuse while the source lists voters, so
  the choice below is always explicit.
- `configuration-only` — count the voters and drop them (no census is built).
- `preserve-source-voters` — import every source voter as the event census.

Under `preserve-source-voters` each voter is emitted into the `Voters` sheet and
flows through the shared voters builder into the `export_voters-*.csv` native
member. The decisions must also name the voter area join:

- `population-precinct-tuple` (legacy dumps) equates
  `(voters.COUNTRY, voters.EMBASSY, voters.PRECINCT)` with
  `(pop.PROV_CODE, pop.MUN_CODE, pop.PRECINCT)` of exactly one in-scope polling
  row. The same precinct under different embassies therefore resolves to
  different areas.
- `mail-country-embassy` (post-qualification dumps, which have no voter
  precinct) equates `(voters.MAILCOUNTRY, voters.MAILEMBASSY)` with
  `(pop.PROV_CODE, pop.MUN_CODE)`.

Under either, a key shared by rows with different outcomes is a conflict.
Without the join decision the conversion is refused as not approved. The join is
recorded in provenance under `decisions`.

Each voter carries two kinds of attribute:

- Canonical identity, under the keys enrollment matching reads: id → `username`,
  first/last name, maternal name → `middleName`, `suffix`, the date-of-birth parts
  composed as `dateOfBirth` (`yyyy-MM-dd`), and `country`/`embassy` derived from the
  voter's area with the same formatting as the registration option lists.
- Raw source values for audit, unchanged: `source_maternal_name`, `source_suffix`,
  `source_dob_year`/`_month`/`_day`, `source_country`, `source_embassy`,
  and, from a legacy dump, `source_precinct` and `source_abroad_status`. A
  post-qualification dump has neither column, so neither is emitted. The abroad
  status is never turned into eligibility.

By default, email, mobile and password are left unset; enrollment collects them
later, so the conversion does not warn that voters cannot be sent a one-time code.
An optional supplemental **Voters** sheet can restore account values that are
not in the SQL census. It requires `preserve-source-voters` and joins by the
source `username`. Each username may occur once and must already be in the census.
Only these columns are accepted:

- `username`: the source voter ID, used only as the join key.
- `email`: the account email address.
- `email_verified` and `enabled`: explicit account flags, including `false`.
  Use Excel booleans or `true`/`false`, `yes`/`no`, `1`/`0` (`x` means true).
- `authorized-election-ids`: election **external IDs**, separated by `||`.
  They must name generated elections, such as `center:P01:context:alias`.
  Duplicate IDs are removed in source order. Unknown elections block the build.

Omitted rows and blank cells retain the existing defaults: no email or election
restriction, enabled, and email verification based on whether an email is present.
An explicit verification flag is preserved even without an email. SQL identity,
location and audit fields cannot be overridden; other account, enrollment and
credential columns are rejected. Source SQL contact columns are not copied
implicitly. Direct builds, the Wizard, and saved/reopened plans preserve the
supplied account values. The exported voters CSV uses a single `|` between
canonical external IDs, which the Keycloak mapper resolves to platform IDs.
A blank restriction stays blank and leaves the existing area-based rules in place.
Every SQL census row is checked before the conversion is refused, and each failing
voter is reported by row and column: a location with no in-scope area, a duplicate id, an incomplete
or impossible date of birth, or an area without an embassy/country label. Nothing
is dropped silently. The preserved count is reported in the summary and stored in
provenance as `preservedVoters`.

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
warnings. The existing workbook-only command remains available. `--workbook` may
be a defaults workbook (no `Elections` sheet), with the voter area join in the
decisions when voters are preserved:

```json
{
  "census": "preserve-source-voters",
  "voterAreaJoin": "mail-country-embassy",
  "ballot": "seat-plurality-at-large",
  "scope": "configured-seats-only",
  "scheduleOffsetMinutes": 480
}
```

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
CSV; `preserve-source-voters` adds an `export_voters-*.csv` census member.
Keep original source files in controlled local storage; hashes do not
replace the originals.

## Editing a converted event

`openFromHistorical(sql, workbook, decisions)` in the core's WASM converts the
dump as `adapt` does and reads the result back with `plan_from_workbook`. It
returns an editable plan, the same as opening a wizard workbook does. The
Election Architect uses it for *Edit in the Wizard*. Compiling that plan
unchanged builds the same ballot as the direct build:

- Columns the plan has no field for are carried on each entity (`columns`).
  So are the `miru:*` and `import:provenance` annotations. The writer puts both
  back into the workbook the build reads.
- Contest behaviour the template would supply for a blank cell is read as
  explicit overrides. A value every contest shares becomes a plan default.
- The event's `presentation.i18n.<lang>` object gives its name and description
  to the plan's fields and its remaining keys to the portal texts (`i18n`). It is
  not carried whole, so an edited name wins.
- `presentation.ceremonies_policy`, which the Admin Portal's ceremony screens
  read, is the plan's ceremony policy. A `Ceremony` sheet overrides it.

The test `a_converted_event_opened_in_the_wizard_builds_the_same_ballot` checks
the unchanged round trip. `wizard_edits_of_a_converted_event_win_over_its_source_cells`
checks that edits win.

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

## Event messaging configuration

A `Parameters` row with key `messaging_config` and a JSON object in `value`
sets the native export's top-level `messaging_config`. This is the same
`EventMessagingConfig` used by the Admin Portal: `version`, `channels` (account
references, purposes, template bindings and out-of-window policies), and
`election_channels`. It survives direct SQL/XLSX conversion, Edit in the Wizard,
plan save/reopen and opening a native event export.

In workbook input, `election_channels` keys may be election `external_id`
values or the generated election UUIDs. Native exports and base exports use
UUIDs; these are mapped through their elections' external IDs to the rebuilt
UUIDs. Unknown elections and conflicting references are refused, rather than
silently dropping a restriction.

An absent or blank parameter keeps a base export's configuration. A literal
`null` overrides the base and emits no event configuration. To explicitly
configure no channels, use `{"version":1,"channels":[],"election_channels":{}}`.
The legacy `messaging:config` annotation and Keycloak's public
`sequent.messaging` projection do not substitute for the top-level field.

The Architect validates the configuration shape and its local references. It
cannot verify destination accounts: the referenced accounts must already exist
in the target tenant, support the channel and be ready for the enabled purposes.
The import still requires `messaging-config-write` and performs those checks.
No provider accounts or credentials are created or exported by this parameter.

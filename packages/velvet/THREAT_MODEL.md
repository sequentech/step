<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->

# velvet threat model

velvet is the tally and report pipeline. It decodes the mixed and decrypted plaintext ballots, counts them per area and contest (plurality at large, instant runoff, acclaimed), adds approved tally sheets, marks winners, and writes JSON and HTML reports (PDF when enabled), a SQLite results database and, in ballot-image runs, signed ballot-image PDFs. Production runs it in-process inside the windmill worker container: windmill writes `velvet-config.json` and an input tree, then steps `src/cli/state.rs` `State` through the pipes. Operators and tests can also run the CLI `velvet run <stage> <pipe_id> -c -i -o` (`src/main.rs`). Stages exchange data only through JSON, CSV, HTML, PDF and SQLite files under the output directory. velvet's own code opens no sockets and starts no processes; PDF rendering and page signing go through sequent-core, which does both. The official counts, the winners and the result documents all come from velvet. See the [system threat model](../../THREAT_MODEL.md).

## Assets

- **Decoded ballots**: per-ballot choices and write-in text in `velvet-decode-ballots/**/decoded_ballots.json`, `velvet-decode-mcballots/**`, and the optional `ballot` table of the results database. Integrity: every count comes from them. Confidentiality: they are unlinked from voters after the mix, but they are still ballot content.
- **Contest results and winners**: `velvet-do-tally/**/contest_result.json` and `velvet-mark-winners/**/winners.json`, which windmill copies into the results tables. Integrity.
- **Preferential count record**: the rounds, transfers and tie resolutions in `ContestResult.process_results`. Integrity and auditability.
- **Counting configuration**: contest and area config, area weights, `tally_operation` scopes, census, channel counts and recorded tie resolutions. Integrity: they decide how ballots turn into results.
- **Tally-sheet inputs**: `default/tally_sheets/**/tally-sheet.json`, the counts entered for paper and other channels. Integrity.
- **Result documents and `results_hash`**: `velvet-generate-reports/**` and the results database. Integrity and authenticity of the published outcome.
- **ACM signing key**: the event's key, which windmill passes to velvet for ballot-image runs. Confidentiality: it signs ballot-image pages for third parties.
- **Ballot-image PDFs and manifest**: `velvet-mcballot-images/**`, `velvet-ballot-images/**` and `ballots_files.csv`, handed to auditors and external systems. Integrity and completeness.
- **Worker host**: the credentials, files and network position of the windmill worker that velvet runs in. Confidentiality.
- **Tally availability**: a failed or blocked run delays the results. Availability.

## Entry points and trust boundaries

| Entry point | Who can reach it | Trust | Checks in place |
|---|---|---|---|
| `src/cli/cli.rs` `CliRun`, `src/cli/state.rs` `State` | windmill tally and template tasks (`services/ceremonies/velvet_tally.rs` `call_velvet`) | internal service | Config parsing checks that every stage in `stages.order` is defined and pipe types are unique per stage. `src/pipes/pipe_name.rs` `PipeName` is a closed enum |
| `src/main.rs` `main` | operator, developer, tests | operator | Same as above. Input and output paths are taken as given |
| `src/cli/state.rs` `State::get_results` | windmill results persistence (`services/ceremonies/results.rs`) and transmission package (`services/consolidation/create_transmission_package_service.rs`) | internal service | Reads the reports the run wrote |
| `default/configs` read by `src/pipes/pipe_inputs.rs` `PipeInputs::new` | written by windmill from the published ballot style; names, descriptions, annotations and weights come from election managers or event imports | internal service, carrying admin-authored fields | Typed serde structs |
| `default/ballots/**/ballots.csv` read by the decode pipes in `src/pipes/decode_ballots/` | written by windmill from the mixnet output; the content is chosen by voters | untrusted content, delivered by an internal service | `BigUint` parsing, then the sequent-core decoder and checker |
| `default/tally_sheets/**/tally-sheet.json` read by the tally pipe in `src/pipes/do_tally/` | written by windmill from approved sheets entered by data-entry users | authenticated admin | Every candidate must belong to the contest; sheets for acclaimed contests are rejected |
| `default/database` and `PipeConfigGenerateDatabase`, used by `src/pipes/generate_db/generate_db.rs` `populate_results_tables` | windmill | internal service | Bound SQL parameters; decoded ballots are copied only when `include_decoded_ballots` is set |
| Templates, `extra_data` and `pdf_options` in `src/config/generate_reports.rs` `PipeConfigGenerateReports` and `src/config/ballot_images_config.rs` `PipeConfigBallotImages` | windmill; the templates, including the optional `report_content_template` override, are authored by tenant admins | authenticated admin | Typed per-pipe `get_config`; rendered with sequent-core Handlebars |
| Ballot-image runs: the pipes in `src/pipes/ballot_images/` | windmill template generation through `call_velvet` | internal service | Same input parsing as a tally. windmill starts it only for a tally session that completed successfully |

## Threats

| ID | STRIDE | Threat | Controls in the code | Status |
|---|---|---|---|---|
| velvet-T1 | Tampering | Someone with write access to the input tree, the config or the intermediate stage files changes the count | None inside velvet beyond typed parsing. The plaintexts come from braid's verifiable mix and the configs from the published ballot style, both written by windmill | Accepted (velvet is a transform of the tree windmill gives it; the tree's authenticity belongs to windmill and braid) |
| velvet-T2 | Tampering | Invalid ballots are counted as valid, or ballots are misclassified | The sequent-core decoder and checker mark over-votes, duplicate ranks and preference gaps as invalid | Partial |
| velvet-T3 | Tampering | Votes or tally-sheet counts are credited to the wrong candidates or to an acclaimed contest | An unknown candidate ID in ballots or sheets fails the tally; acclaimed contests get a fixed result before any ballot is read, and their tally sheets are rejected. Tally-sheet review happens upstream (see Assumptions) | Partial |
| velvet-T4 | Tampering | Ballots are lost between the mix and the count, or incomplete results are published | Counted participation is reconciled with windmill's per-area counts | Partial |
| velvet-T5 | Tampering | The marked winners or seat allocation do not match the count | Acclaimed contests keep the configured order; the preferential count is bounded by `max_rounds` | Partial |
| velvet-T6 | Repudiation | A preferential tie resolution is wrong or disputed, or contest annotations change counting behaviour | `TieBreakingPolicy` enum; an external-procedure resolution comes from an admin decision checked against the tied set (windmill `services/ceremonies/tally_resolution.rs`); every round, transfer and resolution is serialized into `process_results` | Partial |
| velvet-T7 | Tampering | Area weights or aggregation produce wrong totals | An area with no configured weight counts with weight 1 | Partial |
| velvet-T8 | Repudiation | Re-running the tally on the same inputs gives different documents, or a report is replaced after generation | `src/pipes/generate_reports/generate_reports.rs` `GenerateReports::generate_report` embeds as `results_hash` a `hash_b64` of the serialized report data, or the hash the caller passes in; `sort_report_sections`, `compare_report_contests`, `sort_candidates` and `ballot_level_report` fix the layout. velvet does not sign; storage and the electoral log are downstream | Partial |
| velvet-T9 | Elevation of privilege | Untrusted text or templates inject HTML or script into generated documents, which run in the PDF renderer or in a reader's browser | Handlebars output escaping; sequent-core provides a `sanitize_html` helper (ammonia allow-list) | Partial |
| velvet-T10 | Information disclosure | Template content steers the renderer into reading local files or reaching internal hosts, or the renderer loads untrusted scripts | Deployment (`DOC_RENDERER_BACKEND`); windmill points template asset URLs at the public-assets bucket | Not verified |
| velvet-T11 | Information disclosure | Decoded ballots or the ACM key are disclosed | Decoded ballots go into the results database only when `include_decoded_ballots` is set; sequent-core removes the temporary key file it signs with | Partial |
| velvet-T12 | Elevation of privilege | Page signing runs the ECIES tool through `sh -c`: `MCBallotImages::print_ballot_images` calls sequent-core `ecies_sign_data_bulk` (sequent-core `src/signatures/ecies_encrypt.rs`, `src/signatures/shell.rs` `run_shell_command`) | Paths in the command come from a fresh temporary directory | Open on release/10.0 (fix in sequentech/step#3522) |
| velvet-T13 | Tampering | Input-derived values become path components and files land outside the output directory | Path builders use fixed prefixes, and output folders come from a closed set (`src/pipes/pipe_name.rs` `PipeNameOutputDir`); the input tree is written by windmill | Partial |
| velvet-T14 | Denial of service | Malformed input, configuration or ballots abort or crash the tally | Typed serde parsing and error mapping to `src/pipes/error.rs` `Error`. The sequent-core `ballot_codec/vec.rs` `decode_array_to_vec` panic on an out-of-range length byte is open on release/10.0 (fix in sequentech/step#3522) | Partial |
| velvet-T15 | Tampering | Ballot-image artefacts handed to third parties are altered or incomplete | File names carry the SHA-256 of the PDF bytes | Partial |
| velvet-T16 | Tampering | SQL in IDs, names or annotations changes the results database | `src/pipes/generate_db/generate_db.rs` `process_decoded_ballots` and sequent-core `sqlite/results_*.rs` bind every value as a rusqlite parameter; sequent-core `sqlite/utils.rs` `ensure_column` interpolates only literals from its callers | Mitigated |
| velvet-T17 | Information disclosure | Documents sent to a remote PDF renderer are exposed in transit or at rest | The deployment chooses the transport and storage through `DOC_RENDERER_BACKEND` | Not verified |
| velvet-T18 | Denial of service | Ballot-image runs hang or exhaust the worker | Worker resource limits (deployment) | Not verified |
| velvet-T19 | Tampering | The preferential count does not follow the rules published for the contest | Tests in `tests/instant_runoff/` | Partial |

## Assumptions

- **windmill** builds the input tree from braid's verified mix output, the published ballot style and the approved tally sheets it has validated.
- **Upstream services** run the tally-sheet approval workflow. velvet counts every sheet it is given.
- **windmill** passes velvet every mixed plaintext.
- **windmill** gives each run its own working directory and treats any velvet error as a failed run.
- **windmill** supplies the contest configuration in force for the election, blocks incompatible policy combinations and accepts tie resolutions only from authorized admins.
- **harvest and windmill** let only authorized users start ballot-image runs, which sign with the event's ACM key.
- **sequent-core** decodes and validates ballots correctly, escapes template output, and isolates the PDF renderer for whichever `DOC_RENDERER_BACKEND` the deployment uses.
- **Tenant admins** are trusted with their tenant's templates. Write access to the MinIO public-assets bucket is restricted to operators.
- **windmill and electoral-log** store the result documents and record `results_hash`, so a document replaced later can be detected.
- **Operators** who run the CLI do so on trusted machines, with input trees from a trusted source.

## Review focus

1. Winner determination for every counting algorithm. This decides who is elected.
2. Preferential counting against the rules published for each contest.
3. Completeness of the count, from the mix to the published results.
4. Weighting and aggregation of counts.
5. Output encoding in report and ballot-image templates.
6. Isolation of the PDF renderer and handling of the documents sent to it.
7. Protection of the ACM key and of ballot content.
8. The ballot-image artefacts handed to third parties.
9. Reproducibility of results and documents.
10. Path construction, symlinks in the input tree, and robustness against malformed input and configuration.
11. Tally-sheet inputs.

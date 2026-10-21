---
id: offline-ballot-input-audit
title: Offline Ballot Input Audit
---

<!--
SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
SPDX-License-Identifier: AGPL-3.0-only
-->

This workflow links signed electoral-log cast-vote statements to stored ballot
contents and the exact B3 pre-shuffle input multiset for one tally/area/contest.
It operates on copied evidence, without live Windmill, Hasura, or Keycloak APIs.

The four core commands have different boundaries:

1. `export-ballot-audit` reads restored local Hasura, Keycloak, and B3 PostgreSQL
   databases, plus copied B3 blob files when needed.
2. `export-electoral-log` reads a loopback ImmuDB service restored from a copy.
3. `prepare-ballot-audit` selects and validates inputs from pinned local exports.
4. `ballot-audit` checks the complete prepared scope using local files only.

Optionally, `audit-cast-history` compares event/election-wide Hasura history
across **all areas** with signed cast-vote logs. This is a separate coverage
layer, not an inference from the area-scoped selected set.

Acquisition, input reconciliation, and universal cryptographic verification
are separate. A passing input audit is not a complete election audit.

## Existing checks and the gap addressed

The reviewed `release/10.0` baseline already has:

- Auditable-ballot re-encryption checks using uploaded plaintext/randomness.
  They check the uploaded ballot against itself, not against tally inputs.
- Receipt lookup in stored cast-vote rows, without tally membership verification.
- Windmill batch reconciliation during extraction: weighted divergence aborts,
  nonweighted divergence warns and keeps the posted batch, and completed batch
  extraction can be skipped. This is not an independent completed-tally audit.
- Braid universal verification of protocol signatures, configuration, mixing,
  and decryption. It does not reproduce receipt hashing or voter/revote/census
  selection. Its existing CLI uses a gRPC server, and its
  [`--ballot-hash` argument is unimplemented](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/braid/src/bin/verify.rs#L29-L33).

The new commands connect signed electoral-log evidence, original cast-vote
content, complete scoped selection, and signed B3 input artifacts. They do not
replace re-encryption checks, immudb history proofs, mix/decryption proofs, or
final counting verification.

## Prerequisites: frozen evidence and independent trust

Preserve the original backups unchanged. Restore disposable working copies
into isolated local databases, prevent application writers from connecting,
and use read-only accounts. A read-only transaction does not mean the database
process never writes logs or other runtime files.

Establish a consistent historical boundary across Hasura, B3, immudb, and the
eligibility evidence. Independent database transactions do not create a common
cross-database snapshot. B3 blob files must also match the copied SQL state.
Record and authenticate backup identities, checkpoint references, scope, and
exact-byte SHA-512 source digests independently of the files under audit.

The three Hasura/B3/immudb copies alone may not recover historical enabled-user
state, area/election authorization, delegation, or weights. A corresponding
frozen Keycloak copy or independently preserved complete eligibility projection
may be necessary. Do not substitute current voter state or require live APIs.
If that evidence is unavailable, report the missing selection provenance.

Signed electoral-log messages do **not** prove membership in an immudb snapshot,
append-only consistency, archive completeness, or authenticated absence.
Transaction-inclusion and consistency proofs against trusted historical
checkpoints remain separate prerequisites; these commands do not verify them.

A newly calculated checksum only describes the bytes currently supplied.
Building a fresh manifest after alteration cannot prove the original historical
contents or selection. Generated acquisition/bundle hashes must retain a chain
to independently authenticated sources; they do not manufacture that trust.

### Save and pin the exact scope

Create `scope.json` with `format_version: 1`, a `scope` object, and `batches`.
There is no selected-row list in this preparation input. The scope fields are:

- `tenant_id`, `election_event_id`, `election_id`, `area_id`, `tally_session_id`,
  `tally_session_contest_id`, and `board_name`.
- `contest_id`: the named contest for single-contest encryption, or JSON `null`
  for the consolidated multiple-contest ciphertext.
- `base_batch`: numeric `TallySessionContest.session_id`, not the string tally
  session ID; `policy`: `unweighted`, `delegated`, or `weighted`.
- `public_key_sha512`: the full lowercase hexadecimal B3 public-key artifact
  hash expected in the `Ballots` statements.
- `electoral_log_system_public_key`: the independently authenticated system
  public key in DER/SPKI, standard-base64 encoded with padding where required.

`batches` is the exact complete batch list, recovered from the saved mapping
and weighted `annotations.weight_bit_mask`, not a guessed numeric neighborhood.
Pin the exact `scope.json` bytes with a full SHA-512 digest, 128 lowercase hex
characters. Whitespace changes its digest.

B3 `Ballots` statements carry configuration hashes and numeric batch IDs, not
tenant/area/tally labels. The authenticated saved mapping supplies that
application-level binding; a matching ciphertext alone cannot establish it.

For the default keys ceremony, the actual event board is
`election_event.bulletin_board_reference.database_name`. Non-default ceremonies
can use an election-specific board and deployment slug. Never invent an
area-specific board name or assume the event board always applies.
[Native board mapping](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/windmill/src/services/ceremonies/keys_ceremony.rs#L39-L74).

## 1. Export the restored PostgreSQL evidence

`export-ballot-audit` connects directly to local PostgreSQL, not to application
APIs. Its current mapping validation supports **default keys ceremonies only**.
A non-default ceremony fails closed and needs separately reviewed acquisition
with an authenticated environment-slug/election-to-board mapping. Do not alter
the saved ceremony data to make the exporter accept it.

Prepare a private `connections.json` using the actual restored databases and
backup identifiers. This example assumes local authentication is already
configured:

```json
{
  "format_version": 1,
  "hasura": {
    "connection_string": "host=127.0.0.1 port=55432 dbname=hasura_copy user=audit_reader",
    "restore_reference": "authenticated Hasura backup identifier"
  },
  "keycloak": {
    "connection_string": "host=127.0.0.1 port=55432 dbname=keycloak_copy user=audit_reader",
    "restore_reference": "authenticated historical Keycloak backup identifier"
  },
  "b3": {
    "connection_string": "host=127.0.0.1 port=55432 dbname=b3_copy user=audit_reader",
    "restore_reference": "authenticated B3 backup identifier"
  }
}
```

A connection may instead use an absolute Unix socket directory as `host`.
If credentials are needed in connection strings, keep them in this private
local file, not command arguments, shared output, or a published audit.
The PostgreSQL exporter currently requires a Unix platform for these private
file-permission checks. The file must be regular, not a symlink, and have no
group/other permissions:

```sh
chmod 600 ./connections.json
export-ballot-audit \
  --scope ./scope.json --scope-sha512 "$SCOPE_SHA512" \
  --connections ./connections.json \
  --output ./postgres-export
```

Add `--b3-blob-root /absolute/path/to/copied/blob-root` when needed. B3 can store
message bytes outside SQL; a database-only copy can then be insufficient.
[Native B3 blob storage](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/b3/src/grpc/server.rs#L189-L218).

The exporter:

- Allows one literal loopback IP or absolute Unix socket host per connection.
  It rejects remote/DNS hosts, `hostaddr` overrides, custom startup options,
  and TLS-required connections. Do not tunnel production through loopback.
- Uses a separate read-only, repeatable-read transaction for each database.
  It checks the saved event/election/session/ceremony/contest, policies, base
  batch, and weighted mapping against the pinned scope.
- Reads **all cast-vote history in the pinned tenant/event/election/area**,
  including every status and revote. It preserves original content strings.
  In the same Hasura transaction, it also captures event/election-wide history
  without an area filter for the optional historical-coverage check.
- Reproduces the frozen native eligibility predicate and records source
  attributes and reasons, rather than deriving eligibility from B3 membership.
- Reads the complete B3 board. It recovers missing message bytes from copied
  blobs, rejects database/blob disagreements, and checks decoded message
  fields against row index columns. Signature and inclusion checks happen later.

Valid ASCII board identifiers longer than 63 bytes retain their full names for
index/blob lookup; SQL relation lookup follows PostgreSQL's native lowercase
and 63-byte truncation, rejecting indexed name collisions. The best-effort
`index.message_count` is preserved beside the observed count and a mismatch
diagnostic, not treated as omission evidence or a fatal integrity check.

The new private output directory contains:

- `scope.json`, `cast-votes.json`, `eligible-voters.json`, `board-messages.json`.
- `cast-vote-history.json`, covering all areas/statuses/revotes in the pinned
  tenant/event/election, for the optional history command.
- `saved-mapping.json`, `keycloak-source.json`, `eligibility-reasons.json`,
  `board-source.json`, for examining provenance and upstream decisions.
- `acquisition.json`, recording file checksums, counts, per-database snapshots,
  operator-supplied restore references, and limitations.

Restore references are claims, not cryptographic proof. Authenticate the
acquisition record independently and retain historical source anchors before
using its hashes. This command neither acquires immudb data nor establishes a
cross-database/filesystem atomic snapshot. It requires all three restored
PostgreSQL databases; if the historical Keycloak copy is absent, use separately
authenticated exports and a complete frozen eligibility projection with step 3.

## 2. Export the locally restored ImmuDB records

Restore the copied database into an isolated local ImmuDB process using the
appropriate version/restore procedure. Keep original backups untouched and
prevent new election writes. `export-electoral-log` does not restore/start the
server, change its configuration, or read raw immudb storage files.

Set `ELECTORAL_LOG_DB` to the verified local database and
`IMMUDB_READ_ONLY_USER` to an existing read-only account. This example assumes
the disposable service listens on port `43322`; use its actual local port:

```sh
(
  read -r -s -p 'Local read-only ImmuDB password: ' BALLOT_AUDIT_IMMUDB_PASSWORD
  printf '\n'
  export BALLOT_AUDIT_IMMUDB_PASSWORD
  export-electoral-log \
    --server-url http://127.0.0.1:43322 \
    --database "$ELECTORAL_LOG_DB" \
    --username "$IMMUDB_READ_ONLY_USER" \
    --output ./immudb-export
)
```

The default password environment variable is `BALLOT_AUDIT_IMMUDB_PASSWORD`;
`--password-env` accepts another variable's name, not a password. Only literal
loopback HTTP endpoints with a nonzero port are accepted, including IPv6
`http://[::1]:43322`. DNS names, remote addresses, embedded credentials, and URL
paths are rejected. Do not proxy production through this boundary.

The exporter opens a session and a `ReadOnly` transaction, reads
`electoral_log_messages.message` BLOBs in ascending ID pages of 500, then rolls
back the read transaction and closes the session. It rejects non-increasing
IDs and missing, empty, or non-BLOB message values.

It creates `electoral-log.json`, an unpadded-base64 array of original message
bytes, and `electoral-log-provenance.json`, with counts, a checksum, and limits.
This is **raw-record acquisition**, not signature verification. Signatures are
checked during preparation/audit. The generated checksum is not a historical
anchor or an immudb inclusion/consistency proof. The local snapshot is not
automatically synchronized with PostgreSQL or eligibility snapshots.

## 3. Prepare the complete selected bundle

`prepare-ballot-audit` reads five local files and requires a separately
supplied, independently authenticated 128-character lowercase SHA-512 pin for
each. It makes no database/network connections:

- Scope: the `format_version`, `scope`, and `batches` envelope described above.
- Cast votes: complete scoped history, including all statuses/revotes, or a
  full table export. Required fields: `id`, `tenant_id`, `election_event_id`,
  `election_id`, `area_id`, `created_at`, `voter_id_string`, `content`,
  `ballot_id`, `status`, `annotations`. Extra columns are ignored; required
  nullable columns must remain present. IDs must be unique valid UUIDs; scoped
  timestamps are RFC3339 or null. Status is `valid`, `discarded`, or `in-progress`.
- Frozen eligibility: `{format_version: 1, scope_sha512, voters}`. The digest
  binds the exact saved scope file; `voters` is the complete array of unique
  `{voter_id, multiplicity}` entries from the historical eligibility decision.
- Full B3 board: an unpadded-base64 array of original serialized B3 `Message`
  bytes, containing exactly one configuration message plus other protocol data.
- Full electoral log: an unpadded-base64 array of original Borsh-serialized
  `electoral_log::Message` bytes, not a hand-picked subset or JSON display export.

After authenticating acquisition provenance and the five source pins:

```sh
prepare-ballot-audit \
  --scope ./postgres-export/scope.json --scope-sha512 "$SCOPE_SHA512" \
  --cast-votes ./postgres-export/cast-votes.json --cast-votes-sha512 "$CAST_VOTES_SHA512" \
  --eligible-voters ./postgres-export/eligible-voters.json \
  --eligible-voters-sha512 "$ELIGIBLE_VOTERS_SHA512" \
  --board-messages ./postgres-export/board-messages.json \
  --board-messages-sha512 "$BOARD_MESSAGES_SHA512" \
  --electoral-log ./immudb-export/electoral-log.json \
  --electoral-log-sha512 "$ELECTORAL_LOG_SHA512" \
  --output ./prepared-audit
```

Preparation rejects scoped in-progress rows; reproduces latest-valid selection
and the frozen census join; preserves selected content; extracts the original
configuration and exact `Ballots` messages; and finds a unique signed log match
for each selected voter-pseudonym/full-ballot-hash pair. Missing/ambiguous
matches fail. The complete generated bundle must pass `ballot-audit` before
being written. This automates selection **from** the supplied frozen decision;
it does not independently reconstruct or authenticate that decision's history.

The output directory must be new and its parent must exist. It contains
`manifest.json`, `configuration.borsh`, `cast-votes.json`, `messages.json`,
`electoral-log.json`, `anchors.json`, `provenance.json`, and `selection.json`.
On Unix, directories are mode `0700` and files `0600`; files are not overwritten.
stdout gives aggregate counts. Source pins, accounting, and limitations are
retained in `provenance.json`.

Generated `anchors.json` pins the prepared manifest/configuration for a repeat
check. Those hashes inherit trust only from the independently authenticated
sources and preparation record. They cannot retroactively prove historical
metadata integrity missing from the native receipt hash.

### Inspect a nonselected stored receipt privately

```sh
jq --arg receipt "$FULL_RECEIPT" \
  '.rows[] | select(.ballot_id == $receipt)' ./prepared-audit/selection.json
```

Reasons are `outside_scope`, `superseded_revote`, `discarded`,
`no_eligible_voter`, and `selected`, with latest scoped row references,
multiplicity, and stage counts. Only selected rows are
`selected_audit_verified`; other stored receipt labels are `not_checked`.
They have not had their receipt hashes or signed-log links verified. This
reconstruction does not authenticate archived exclusion JSONL. Keep it private.

## 4. Audit the prepared scope

`ballot-audit` accepts five files, two independent digest arguments, and an
optional report filter:

```sh
ballot-audit \
  --manifest ./prepared-audit/manifest.json \
  --manifest-sha512 "$AUTHENTICATED_MANIFEST_SHA512" \
  --cast-votes ./prepared-audit/cast-votes.json \
  --configuration ./prepared-audit/configuration.borsh \
  --configuration-sha512 "$AUTHENTICATED_CONFIGURATION_SHA512" \
  --messages ./prepared-audit/messages.json \
  --electoral-log ./prepared-audit/electoral-log.json \
  --ballot-id "$FULL_RECEIPT"
```

Authenticate the exact manifest/configuration bytes independently. For a bundle
created by step 3, retain the authenticated original source pins and preparation
record alongside its generated anchors. Do not treat checksums supplied by an
untrusted bundle as authentication.

Omit `--ballot-id` to report all selected receipts. With it, **every selected row
and batch is still checked**; only reported positions are filtered. A receipt
outside the selected scope errors, not proves that it was never cast. Use the
private selection record to examine stored nonselected labels.

The selected-scope result does not reconcile every historical signed `CastVote`
log to a Hasura row. An accepted cast missing from both the supplied history
and tally inputs can escape this selected-set comparison. An unmatched historical
receipt is **unexplained/incomplete**, not automatically a lawful exclusion.
Use the optional historical-coverage check below for a separate
event/election-wide comparison across all areas; the unsigned outer
electoral-log area metadata is not sufficient.

### Exact prepared-file contract

- Manifest: `format_version: 1`, `scope`, `batches`, and complete `selected`.
  Each selected entry has `cast_vote_id`, `ballot_id`, `content_sha512`,
  `electoral_log_message_sha512`, and positive `multiplicity`.
- Cast votes: exactly the selected JSON-array rows, each containing
  `cast_vote_id`, `ballot_id`, `voter_id`, `voting_channel`, and `content`.
  Preserve the original content string, including whitespace. Escaping it for
  the outer JSON is valid; parsing/reserializing the inner ballot is not.
- Configuration: original raw Borsh `Configuration<RistrettoCtx>` artifact
  bytes, not JSON, base64 text, or the enclosing B3 message.
- Messages: exactly one original signed B3 `Ballots` message per declared
  batch, in a standard unpadded-base64 JSON array. No configuration, mix,
  decryption, or unrelated batch messages belong in this prepared file.
- Electoral log: exactly the selected, pinned original serialized messages,
  also in an unpadded-base64 JSON array. Extra, duplicate, missing, or reused
  evidence is rejected.

The native B3 CSV export's `message` column already uses unpadded base64.
Conversion must preserve its bytes; the CSV labels are not board provenance.
Neither excluded-ballot JSONL nor the receipt-only `step-cli` cast-vote export
is a replacement for the full-content inputs.

### Four different hashes and the signed bridge

1. Receipt: SHA-512 of the Borsh raw hashable ballot, **truncated to 32 bytes**
   and hex-encoded. It covers version, issue date, and decoded contests, but
   excludes config, ballot-style hash, and voter signing fields. It is not
   SHA-256 or a hash of JSON text.
2. Signed electoral-log ballot hash: the **full 64-byte** version of that same
   raw-ballot SHA-512. It still excludes the same metadata.
3. `content_sha512`: full SHA-512 of the original content string's UTF-8 bytes,
   binding all text to the manifest's separate anchor.
4. `electoral_log_message_sha512`: full SHA-512 of the original serialized
   whole-message bytes, binding the exact selected log evidence.

The existing [receipt implementation](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/sequent-core/src/encrypt.rs#L545-L571)
provides the hashing rules. A newly created full-content digest does not by
itself prove when those bytes were recorded or validate signing fields inside
the ballot content. Unsupported ballot versions are rejected.

For each selected row, the audit verifies both electoral-log sender and system
signatures; requires the signed header's event/kind and body's election to
match scope; and compares the full ballot hash and voter pseudonym. The
pseudonym is SHA-512 of the **Borsh-serialized String** voter ID, not its plain
UTF-8 or JSON encoding. `CastVoteWithChannel` binds a signed channel; legacy
`CastVote` lacks that field, so `ONLINE` is an explicit reported compatibility
inference, not signed channel evidence.

Electoral-log signatures cover the statement only. Unsigned outer `user_id`,
`username`, `election_id`, `area_id`, and `ballot_id` are not identity/scope
proof. The audit does not use them as authority. Whole-message pinning does not
turn them into signed claims or independently establish the sender's identity.

### B3 checks and result meaning

The audit verifies the protocol-manager signature and configuration binding,
the pinned public-key hash, and SHA-512 of the **original artifact bytes**
against the `Ballots` statement's `CiphertextsHash`. Hashing a JSON rendering is
not equivalent. The explicit artifact comparison is necessary because the
baseline [`Message::verify` artifact hook is a no-op](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/b3/src/messages/message.rs#L399-L418).

The recomputed receipt covers the whole ballot. Projection then selects the
named single-contest ciphertext or the consolidated multiple-contest
ciphertext. Exact per-batch **multisets and multiplicities** must match; one
matching ciphertext anywhere is insufficient. Order is not compared.

Success prints JSON with `scope`, `checked_cast_votes`, `checked_batches`,
`checked_electoral_log_messages`, `legacy_online_channel_inferences`,
`receipts`, and an assurance statement. Receipt locations use zero-based
`{batch, index}` positions. Identical ciphertexts are not uniquely attributable:
all matching positions are reported, not a unique row-to-position assignment.

Errors exit unsuccessfully, without a partial passing report. Rejection cases
include malformed/unsupported inputs; wrong pins/content/receipts; duplicate
IDs/receipts/contests; missing selected rows; extra unselected evidence;
invalid signatures/event/election/pseudonym/channel/full hash; incorrect batch
mapping or multiplicity; and any omitted, added, duplicated, or substituted
ciphertext that changes an expected batch multiset. CLI errors suppress source
chains that could reveal malformed personal data.

## Optional: reconcile all-area cast history with signed logs

`audit-cast-history` is file-only and does **not** select latest votes, apply
eligibility, or inspect B3. Supply the complete tenant/event/election history
from `cast-vote-history.json`, not the area-scoped `cast-votes.json` or prepared
selected rows. Supply the full electoral-log export, not just selected messages.
Authenticate all three exact-byte SHA-512 pins independently:

```sh
audit-cast-history \
  --scope ./postgres-export/scope.json --scope-sha512 "$SCOPE_SHA512" \
  --cast-vote-history ./postgres-export/cast-vote-history.json \
  --cast-vote-history-sha512 "$CAST_VOTE_HISTORY_SHA512" \
  --electoral-log ./immudb-export/electoral-log.json \
  --electoral-log-sha512 "$ELECTORAL_LOG_SHA512" \
  --ballot-id "$FULL_RECEIPT"
```

The history array requires `id`, `tenant_id`, `election_event_id`, `election_id`,
`voter_id_string`, `content`, and `ballot_id`; nullable fields must be present.
Every row must belong to the pinned tenant/event/election. All areas, statuses,
and revotes are retained: area/status/timestamp are not selection filters here.
Missing voter/content, malformed or duplicate row UUIDs, unsupported ballots,
and a stored receipt inconsistent with the native ballot hash fail validation.

Before filtering, the command verifies every supplied message's sender/system
signatures against the single pinned system key. A `CastVote` header/body
mismatch in either direction is invalid. Matching signed event/election
statements are grouped by signed voter pseudonym plus **full** ballot hash;
unsigned outer area/user/receipt fields are never authority. Other verified
events/elections are counted as ignored. Archives using legitimate foreign or
rotated system keys are unsupported and fail closed, not proof of tampering.
JSON goes to stdout, with aggregate coverage, source pins, and receipt groups
containing row IDs, `signed_log_records`, `distinct_signed_log_message_bytes`,
`distinct_signed_statements`, `duplicate_log_message_records`, and
`classification`. Voter IDs, pseudonyms, and content are not printed:

- `matched` or `matched_repeated_logs`: a row has signed matching evidence.
  Byte-identical repeated records do not prove distinct events.
- `matched_ambiguous_row_attribution`: multiple stored rows share the same
  pseudonym/full-hash pair, so the log cannot identify which row it describes.
- `matched_ambiguous_log_occurrences`: distinct complete messages share the
  pair. Counts cannot establish one-to-one row/event correspondence; even
  differing unsigned wrappers do not prove separate signing events.
- `signed_log_without_hasura_row` or `hasura_row_without_signed_log`:
  unexplained/incomplete coverage, not a lawful tally-selection exclusion.

Omit `--ballot-id` to report every group. With it, the entire history is still
checked; only reported groups are filtered. Exit `0` means consistent supplied
coverage; exit `2` means incomplete coverage, ambiguous row/log attribution, or
a requested receipt absent from the supplied evidence. Both produce JSON. Exit
`1` means invalid input, pins, signatures, or hashes, without a passing report.

Native success logging is best effort, so a row without a signed log is **not
proof of tampering**. Even exit `0` does not establish historical completeness:
an event absent from both supplied sources is undetectable here. Nor does this
check authenticate source history, prove immudb transaction inclusion or
consistency, establish a cross-database atomic snapshot, or prove eligibility
or tally inclusion. Retain the independent provenance and selected-scope audit.

## Selection, multiplicity, and exclusion boundaries

### Reproduce the historical predicate, not a convenient subset

Native selection uses `created_at DESC NULLS LAST, id DESC` per scoped voter.
Any valid row takes precedence over discarded rows; the latest discarded is
retained for exclusion auditing only when no valid exists. An unresolved
in-progress row blocks extraction.
[Selection source](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/windmill/src/services/cast_votes.rs#L103-L159).

The frozen tally-eligibility predicate requires the event realm, no service
account (`service_account_client_link IS NULL`), enabled status, matching
`area-id`, and `authorized-election-ids` matching the election **alias** or SQL
NULL. An absent attribute qualifies; an empty string is not automatically NULL.
There is no channel predicate in this query.
[Eligibility and multiplicity source](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/windmill/src/services/users.rs#L349-L437).

Weights use the numeric minimum of nonempty `vote-weight` attributes, default
one; malformed numbers fail. Delegation counts matching `delegate-vote-to`
attribute rows targeting the voter's username, excluding service accounts.
The native delegation subquery has **no realm/enabled/area/election filters**;
do not silently add them. Preserved source attributes distinguish disabled,
area mismatch, and authorization failures; `no_eligible_voter` alone cannot.

Channels come from `annotations.voting_channel`; missing/null defaults to
`ONLINE`, with explicit values limited to `ONLINE`, `KIOSK`, `TELEPHONE`, or
`EARLY_VOTING`. Channel-related discard decisions occurred upstream; do not
invent an additional tally-time channel exclusion.

### Exact contribution rules

- Unweighted: multiplicity is one, with one ciphertext in the base batch.
- Delegated: multiplicity is `1 + delegate_count`, repeated in the base batch.
- Weighted: multiplicity is weight, from one through `100000`. Each set bit `b`
  contributes once to `base_batch + b`. Weight five contributes once at offsets
  zero and two, not five times to the base batch. Later tally multipliers
  `2^b` are outside this command's checks.

The declared batch set must equal all selected contributions. An empty scope
still requires one empty base batch. Zero multiplicities, invalid unweighted
counts, unsupported weights, and arithmetic overflow fail. Neighboring areas
must not be swept into an arbitrary batch range.

### An absent receipt is not automatically tampering

- Authorization, channel/time, parsing, receipt, proof, or signature rejection
  can happen before accepted storage, without a stored row/exclusion entry.
- Committed `discarded` status and historical evidence explain that stage;
  in-progress is unresolved. Existing electoral-log writes are best effort:
  missing signed evidence makes this audit incomplete, not automatically fraud.
- Superseded rows have zero stage exclusion count. Only the selected latest
  discarded row, or latest valid row without eligible membership, contributes
  one exclusion and one channel participation count. Included channel counts
  use multiplicity; included cast counts are one except under delegation.
- Archived `encrypted-ballots.jsonl` covers selected `discarded` and
  `no_eligible_voter` cases, not all historical requests. It strips signing
  fields, omits a separate original stored receipt, and can replace malformed
  content with an error marker and raw SHA-256, **not** the receipt hash.
  [Archive format](https://github.com/sequentech/step/blob/d6d05a1cbaf9cc967c329005fa852fca10152a4e/packages/windmill/src/services/ceremonies/auditable_ballots.rs#L20-L74).
- Malformed selected ciphertexts abort rather than become legitimate new
  exclusions. Missing batches are failures, not zero votes.
- Post-decryption conversion/decode failures and semantic invalid, declined,
  blank, or valid classifications are separate. An included ciphertext need
  not contribute a candidate vote. This workflow does not change Velvet or
  attribute aggregate post-mix categories to individual receipts.

## Synthetic examples and operational limits

From the configured Rust development environment's `packages` directory, these
opt-in tests generate synthetic data only. Choose new output directories:

```sh
BALLOT_AUDIT_PREPARATION_FIXTURE_DIR=/path/new-preparation-demo \
  cargo test -p ballot-audit --test prepare export_synthetic_preparation_inputs -- --ignored

BALLOT_AUDIT_FIXTURE_DIR=/path/new-audit-demo \
  cargo test -p ballot-audit --test audit export_synthetic_demo_bundle -- --ignored

BALLOT_AUDIT_HISTORY_FIXTURE_DIR=/path/new-history-demo \
  cargo test -p ballot-audit --test history export_synthetic_historical_inputs -- --ignored
```

The first writes the five preparation sources and `source-anchors.json`; the
second writes the five prepared audit inputs, invalid variants, and
`anchors.json`. The third writes `valid`, `missing-row`, and `invalid-signature`
history examples with source pins and expected outcomes. Use their actual
filenames with the commands above. These freshly generated hashes are
**synthetic test anchors**, not a method for creating trusted historical
election evidence.

Acquisition/preparation/audit currently retain complete arrays and comparison
data in memory; paginated ImmuDB reads do not make the exporter streaming.
Provision sufficient memory. Validated scope boundaries are appropriate;
arbitrary selected-row partitions or omitted batches undermine reconciliation.
The history check requires the whole event/election across areas, not separate
area subsets.
No throughput or maximum-election-size claim is implied by the examples.

Keep voter IDs, receipts, cast-vote IDs, frozen census records, and signed logs
private; logs may contain IP/country and other personal metadata. The audit
report omits voter IDs/channels but still contains linkable receipt/row IDs.
Its positions stop **before the shuffle**. Do not track an individual voter's
ciphertext through the mixnet to an identified decrypted ballot.

## Universal verification remains separate

Braid's existing verifier uses gRPC, not raw SQL/files. A local B3 server over
copied evidence can support separate verification without live Hasura,
Windmill, or Keycloak. However, its startup calls `create_index_ine`, and
`blob_root` reads can create/cache files. Preserve originals and use disposable
working copies; no dedicated immutable/read-only B3 server mode or complete
file-only universal-verification adapter is supplied here.

The reviewed baseline printed a summary and could return success with
unestablished required checks. The accompanying verifier fix preserves the
summary, then returns an error/nonzero CLI status if any required root or
descendant batch check is false or unpopulated. Still inspect its individual
results and intended scope. That fix does not authenticate archive completeness
or add receipt/census reconstruction to Braid.

A passing input audit proves consistency with the authenticated selected-set
and byte anchors, including signed log links and exact B3 input counts. It does
not by itself prove historical eligibility, voter intent, encryption correctness,
public-key construction, immudb history, mixing/decryption, or final counting.

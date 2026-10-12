<!-- SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io> -->
<!-- SPDX-License-Identifier: AGPL-3.0-only -->

# Public ceremony and tally verification

`trustee verify` and the browser `WasmVerifier` independently verify public
wbraid transcripts. Neither requires trustee keys, a custody proof, a login,
or a trustee message store. Both call the same verifier in `wbraid::verification`.

Obtain the configuration hash from a trusted election record independently of
the bulletin board being verified. A configuration's self-signature establishes
its internal provenance; it does not establish which election it belongs to.
For a tally, also supply the trusted `protocol_board.id` UUID. Its unsigned
128-bit integer value is the proof domain; the tally-session UUID and batch
number are different identifiers. Browser requests use a decimal string so
JavaScript cannot round it.

## Native CLI

Build with the repository toolchain and target directory:

```sh
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo build --release -p trustee --locked'
```

Fetch a DKG and tally, verify them, and save public evidence:

```sh
packages/rust-local-target/release/trustee verify \
  --board-url https://boards.example.org \
  --dkg-board dkg_0123456789abcdef0123456789abcdef \
  --configuration-hash "$CONFIGURATION_HASH" \
  --tally-board tally_fedcba9876543210fedcba9876543210_1 \
  --tally-id "$PROTOCOL_BOARD_UUID" \
  --export transcript.json
```

Omit the tally arguments to verify only DKG. Repeat the verification offline:

```sh
packages/rust-local-target/release/trustee verify \
  --transcript transcript.json \
  --configuration-hash "$CONFIGURATION_HASH" \
  --tally-id "$PROTOCOL_BOARD_UUID"
```

`--ballots-hash` additionally pins the SHA3-512 hash of the trusted manager's
original **Ballots body**, not its complete wire message or a ballot-box seal.
Repeat `--ballot-ciphertext` to check stored cast-vote ciphertexts for inclusion
in this batch. The verifier reports the number of occurrences of each supplied
ciphertext. It never decrypts an individual voter ciphertext separately.

The CLI writes a JSON report and returns zero only for `verified`. Incomplete
and invalid transcripts return nonzero. An export is public evidence, not an
authentication decision; verification may reject the exported evidence. Export
refuses to replace an existing file.

## Browser API

The administration portal ships the production verifier in its existing
`wbraid-trustee` WASM bundle. Initialize it on an HTTPS page with the same
COOP/COEP isolation and worker configuration as browser trustees:

```js
import init, { initThreadPool, WasmVerifier } from "/wbraid-trustee/index.js";
await init({ module_or_path: "/wbraid-trustee/index_bg.wasm" });
await initThreadPool(Math.min(navigator.hardwareConcurrency || 2, 4));

const evidence = await WasmVerifier.fetchTranscript(
  boardUrl,
  dkgBoard,
  tallyBoard,
);
const report = JSON.parse(
  WasmVerifier.verifyTranscript(
    evidence,
    JSON.stringify({
      configuration_hash: independentlyTrustedConfigurationHash,
      tally_id: independentlyTrustedProtocolBoardIntegerString,
      ballots_hash: independentlyTrustedBallotsBodyHash,
      ballot_ciphertexts: storedCastVoteCiphertexts,
    }),
  ),
);
```

`verifyTranscript` also accepts a file exported by the CLI, without network
access. `fetchTranscript` is read-only and requires board and blob CORS access.
For DKG-only verification, omit `tallyBoard` and set `tally_id` to `null`.
`configurationHash` and `ballotsHash` derive pins from padded-base64 canonical
messages obtained from a trusted platform record; deriving pins from the board
being audited defeats the independent trust anchor.

## Checks and interpretation

Verification checks configuration validity and the independent pin; canonical
messages, signatures, signer roles, configuration and key references, and
equivocation; every DKG commitment's Schnorr proof; and the joint public key and
all verification keys derived from those commitments. Every committee member
must publish the derived public key.

For tally it verifies ballot Naor-Yung proofs, the ordered threshold quorum,
every shuffle proof and its tally-specific domain, all quorum endorsements,
and each attributed threshold decryption proof. It combines the contributions
itself and compares every published plaintext with that result. Agreement among
trustees on false plaintexts does not pass. Empty batches require bound null
mixes. Identical repeat publications are idempotent; conflicting publications
are invalid.

`verified` means all required public proofs and publications passed.
`incomplete` means required publications are missing and must never be displayed
as successful verification. `invalid` means an input, signature, reference or
proof failed. The report's plaintexts are base64-encoded 30-byte payloads in
final mix order. Counting rules, weights and result aggregation remain separate
checks against the election's published configuration.

Encrypted DKG shares are private dealings: only recipients can decrypt them and
check them against commitments. The public verifier checks the commitments and
derived keys, rather than claiming to validate private share contents.

Ballot tracking remains the voting portal's scoped locator and receipt checks.
A receipt verifies storage using the published ballot-box signing key. An audit
opening verifies the ballot's encryption and tracker against the signed ballot
style. This transcript verifier checks ciphertext inclusion in the manager's
input and its shuffle/decryption chain. A tracker alone does not bind a public
tally input to the entire sealed ballot box; use signed ballot-box seals and an
independent input pin when checking batch construction and revote filtering.

## Regression tests

The protocol-board tests produce real DKG and tally transcripts and reject
altered proofs even when their authors re-sign them. They cover noncontiguous
quorums, sibling-tally replay, false quorum plaintexts, incomplete transcripts,
empty batches, ballot inclusion and trusted batch pins.

`scripts/e2e/run-browser-trustees.sh` runs actual manual and automatic ceremonies,
voting and tally with Chromium trustees. A separate observer page with no
trustee keys checks each resulting public transcript and exports its anchors,
evidence and report under the run's `public-verifier/` directory for offline
native verification.

The fixed public fixture in `packages/protocol-board/tests/data/` also checks
CLI exit codes and equality with the browser's real WASM output:

```sh
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test --release -p trustee --test public_verifier_cli --locked'
CHROMIUM_EXECUTABLE_PATH=/path/to/chromium node --test packages/protocol-board/tests/browser/public-verifier.test.mjs
```

The browser ceremony runner additionally checks combined-contest weighted
voting and its independently verified mix/decryption transcripts.

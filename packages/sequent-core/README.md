<!--
 SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>

SPDX-License-Identifier: AGPL-3.0-only
-->
# sequent-core

Sequent shared Rust code. This code might be used in different
projects/packages, like the Ballot Verifier or the Voting Portal.

This includes auditable ballot types, ballot encoding and decoding, wbraid
Naor–Yung encryption and audit openings, ballot trackers, signatures and
receipts. Native trustee DKG, mixing and decryption are handled by
`protocol-board` and the supplied `packages/wbraid` workspace.

## Building

Within the step monorepo dev container:

```bash
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo build -p sequent-core --features default_features'
```

## Generate javascript package

From the repository root, see the fast feedback loops developer guide:

```bash
scripts/dev/step-dev wasm                  # development package for dev servers
nix develop ./packages/sequent-core --command \
  scripts/dev/step-dev wasm --release-package  # committed tgz files
scripts/dev/step-dev wasm --check-package  # committed tgz matches the sources
```

## Run rust tests

To run rust tests:

```bash
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo test -p sequent-core --features default_features --lib'
```

## Browserstack tests

The smoke page encrypts a fresh canonical wbraid sample ballot, checks its tracker,
and verifies its audit opening with the installed `sequent-core` WASM package.
From the repository root, first install the Yarn workspace dependencies and check
the committed WASM package, then serve the page:

```bash
python3 packages/sequent-core/src/wasm/test/serve.py
```

Open `http://localhost:8080/sequent-core/src/wasm/test/test.html`. A successful
run displays `verified: <ballot tracker>`. To run the same page on BrowserStack,
install the dependencies in `packages/sequent-core/browserstack`, set your
BrowserStack credentials, and run:


```bash
cd packages/sequent-core/browserstack
npm run local
```

## Generate JSON schema

```bash
devenv shell -- bash -c 'cd packages && CARGO_TARGET_DIR="$PWD/rust-local-target" cargo build -p sequent-core --release --features default_features'
packages/rust-local-target/release/sequent-core > ballot-schema.json
```



## Candidate portraits in historical SQL + Excel imports

Supply **one** optional portrait per source candidate, not both:

- In the source dump, use the existing `candidate.PICTURE` (legacy) or
  `candidates.PICTURE` (post-qualification) BLOB. Store the raw bytes of a
  static PNG, JPEG, WebP or AVIF, not a base64 string, URL, filesystem path, serialized
  object or image filename. The data-only reader accepts MySQL dump byte
  literals, including `0x…` (complete hex pairs), `X'…'` and escaped
  `_binary '…'`. Do not execute the SQL. An absent column, SQL NULL or a
  zero-length BLOB means no SQL photo. A structure-only dump does not tell
  you which image format, if any, a supplier actually provided.
- Alternatively, the supplemental Excel `Candidates` sheet may specify
  `source_candidate_id` and `image_data_url`: canonical standard base64
  after `data:image/png;base64,` or `data:image/jpeg;base64,`, fitting one
  32,767-character cell or adjacent `image_data_url+` continuation cells (up to
  128 KiB decoded in all). A blank supplemental cell leaves the SQL photo alone.
- Two populated sources are an error, even if identical. Unrecognized,
  malformed, oversized or conflicting images for selected candidates stop
  the import. Nothing is fetched, and no fallback picture is invented.

Both paths decode pixels and output a new metadata-free static PNG. SQL accepts
up to 512 KiB and 1,048,576 pixels per original. PNG/JPEG are limited to
1,024 pixels per side; WebP/AVIF can be up to 1,280 pixels per side without
exceeding the same total pixel limit. SQL produces aspect-preserving ballot
thumbnails up to 160 pixels on the long edge;
smaller images are never enlarged. Supply upright pixels: rotated EXIF input
is refused with a clear diagnostic rather than being displayed sideways.
WebP accepts lossy and lossless stills, including alpha. Lossless streams with
more than 128 entropy groups are refused before allocating their tables.
AVIF accepts an ordinary SDR AV1 still image with optional alpha; HDR or
wide-gamut input, embedded ICC profiles, sequences, grids and other derived images, and
display transformations (rotation, mirroring, cropping or nonsquare pixels)
must first be exported as an upright still. The native and WASM importers use
the same bounded Rust decoders; no browser codec or external converter is needed.
Excel retains its 512 × 512 limit; a photographic Excel input must also fit
the normalized PNG limit below (use a smaller image or the SQL source).
Each output is capped at 128 KiB. Unique SQL sources have a 64 MiB / 64-megapixel
decoding budget. Output placements, including repeat appearances across
elections and contests, share a 32 MiB / 16-megapixel budget. Files have
distinct deterministic names and tenant-correct document links per placement.
For scale, 186 distinct 600 × 600 originals fit the 64-megapixel
decode budget; the format deliberately refuses larger totals rather than
silently dropping portraits. The original SQL and workbook remain unchanged. When a converted workbook is
saved, normalized PNGs exceeding one cell are split automatically without data loss.

## Troubleshooting

Use this checkout's `packages/rust-local-target` for Cargo. The Docker-owned
`packages/target` directory belongs to service builds; do not change its ownership
or clean another checkout's build directory. See the
[build and test guide](../../docs/docusaurus/docs/07-developers/03-development-environment/build-and-test.md)
and [core testing guide](../../docs/docusaurus/docs/07-developers/03-development-environment/testing/sequent-core.md).

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The shared vectors of Step's `sequent-core/tests/ballot_receipts.rs`, which this
// module must reproduce byte for byte:  node --test scripts/vitrine-preview/lib/
import assert from "node:assert/strict";
import crypto from "node:crypto";
import test from "node:test";
import {
  createBallotBox,
  crockfordCharacters,
  digestOfTrackingId,
  normalizeBallotId,
  normalizeTrackingId,
  publicKeyBytes,
  shortTrackingId,
  signCastStatement,
  trackingId,
  trackingIdDigest,
  trackingIdStartsWith,
  verifyBallotSignature,
  verifyCastSignature,
} from "./receipts.mjs";
import { readFileSync } from "node:fs";

const BALLOT_BOX_SEED = Buffer.alloc(32, 1);
const VOTER_SEED = Buffer.alloc(32, 2);
const PKCS8 = Buffer.from("302e020100300506032b657004220420", "hex");
const voterKey = crypto.createPrivateKey({ key: Buffer.concat([PKCS8, VOTER_SEED]), format: "der", type: "pkcs8" });
const voterPk = crypto.createPublicKey(voterKey).export({ format: "der", type: "spki" }).subarray(12).toString("base64");

const BALLOT_HASH = "0b1f6a5c2d7e4f809a1b2c3d4e5f60718293a4b5c6d7e8f9000102030405060";
const statement = () => ({
  tenant_id: "90505c8a-23a9-4cdf-a26b-4e19f6a097d5",
  election_event_id: "33f18502-a67c-4853-8333-a58630663559",
  election_id: "f2f1065e-b784-46d1-b81a-c71bfeb9ad55",
  ballot_hash: BALLOT_HASH,
  voter_signing_pk: voterPk,
  voter_ballot_signature: crypto.sign(null, Buffer.from("fixture-ballot"), voterKey).toString("base64"),
  received_at: "2028-05-08T03:00:00.000Z",
});

test("the shared vector of the Received statement and the receipt key", () => {
  const box = createBallotBox(BALLOT_BOX_SEED);
  assert.equal(box.key.key_id, "fd110d301d2f077d");
  const received = box.signReceived(statement());
  assert.equal(
    received.received_signature,
    "+RR9tbkpia9wUTsGALqnqU3S+af2JmMGKIk0BNDlOZrsE6cBP/PxU0vcxWKVIx1oHPN57Mx5zZEocsY4I0nkAQ==",
  );
  assert.equal(received.ballot_id, "MSY2-HTDT");
  assert.equal(box.verifyReceived(received), "MSY2-HTDT");
  assert.equal(box.verifyReceived({ ...received, received_at: "2028-05-08T03:00:00.001Z" }), undefined);
});

test("the shared vector of the Cast statement and the Cast receipt", () => {
  const box = createBallotBox(BALLOT_BOX_SEED);
  const received = box.signReceived(statement());
  const castSignature = signCastStatement(VOTER_SEED, received.election_id, received.ballot_id);
  assert.equal(castSignature, "sJuZoBaypaHpQejzUs/ANBe8AYx8OsYQq61dPFzYJONC1Buw/h1WuPRamzANQi/g2qIomDAHDIZj7cHe1dzFAg==");
  assert.ok(verifyCastSignature(voterPk, received.election_id, received.ballot_id, castSignature));
  assert.ok(!verifyCastSignature(voterPk, received.election_id, "AAAA-AAAA", castSignature));
  const receipt = box.signCastReceipt({
    election_event_id: received.election_event_id,
    election_id: received.election_id,
    ballot_id: received.ballot_id,
    received_at: received.received_at,
    cast_at: "2028-05-08T03:04:05.678Z",
    cast_signature: castSignature,
  });
  assert.equal(
    receipt.cast_receipt_signature,
    "nrig3o+JjRR4/37Rjqs22Q0Lnba5yy93pA5HPST33irYTlbi3MhMTJEvjla6YR/Cn8li4UcapIzRWLm7hooMBQ==",
  );
  assert.ok(box.verifyCastReceipt(receipt));
});

test("the Tracking ID vectors", () => {
  const key = "gTl3Dqh9F19Wo1Rmw0x+zMuNipG07jeiXfYPW4/Js5Q=";
  assert.equal(trackingId(BALLOT_HASH, key), "BJHF-GCZX-A25K-VT2F-4DSV-GTX1-VAHA-6NDF-6M0R-NQ8B-BAP3-GCJJ-4A9G");
  assert.equal(trackingId(BALLOT_HASH, null), "KQCN-K5ZA-A41X-XW33-2G8Z-ZVPE-7SY9-SJE7-1HQF-XE7X-ZK8E-306D-MMC0");
  assert.equal(trackingIdDigest(BALLOT_HASH, null).length, 32);
  assert.equal(shortTrackingId(trackingId(BALLOT_HASH, key)), "BJHF-GCZX");
  assert.throws(() => trackingId(BALLOT_HASH, "not base64"), /voter_signing_pk/);
});

test("the digest of a Tracking ID is the digest it was written from", () => {
  const digest = trackingIdDigest(BALLOT_HASH, publicKeyBytes(voterPk).toString("base64"));
  assert.equal(digestOfTrackingId(trackingId(BALLOT_HASH, voterPk)), digest.toString("hex"));
});

test("typed IDs are read the way people write them", () => {
  for (const typed of ["FTBE-MHRX", "ftbe-mhrx", " ftbemhrx ", "FTBE MHRX"]) {
    assert.equal(normalizeBallotId(typed), "FTBE-MHRX", typed);
  }
  assert.equal(normalizeBallotId("fTbe-Mhrx-"), "FTBE-MHRX");
  assert.equal(normalizeBallotId("OIL0-IL1O"), "0110-1110");
  for (const bad of ["FTBE", "FTBE-MHRXX", "FTBU-MHRX", "FTBE:MHRX", ""]) assert.equal(normalizeBallotId(bad), undefined, bad);
  assert.equal(normalizeTrackingId("0abc12345"), "0ABC-1234-5");
  assert.equal(normalizeTrackingId("0abc"), undefined);
  assert.equal(normalizeTrackingId("FTBE:MHRX"), undefined);
  assert.equal(normalizeTrackingId("a".repeat(53)), undefined);
  assert.equal(crockfordCharacters("o-i l"), "011");
  assert.ok(trackingIdStartsWith("BJHF-GCZX-A25K", normalizeTrackingId("bjhf gczx")));
  assert.ok(!trackingIdStartsWith("BJHF-GCZX-A25K", normalizeTrackingId("BJHF-GCZY")));
});

// A signed cast made with Step's voting portal code (Vitrine's test vectors for Step), if the Vitrine checkout is there.
const VECTORS = `${process.env.VITRINE_DIR ?? `${process.env.HOME}/vitrine`}/packages/extension/test/vectors/step/casts.json`;
test("the voter's signature on a ballot made by Step's portal verifies", { skip: !(() => { try { readFileSync(VECTORS); return true; } catch { return false; } })() }, () => {
  const cases = JSON.parse(readFileSync(VECTORS, "utf8")).cases.filter((c) => c.name === "single-2-contests-signed");
  assert.equal(cases.length, 1);
  const { variables } = JSON.parse(cases[0].request.body);
  const ballot = JSON.parse(variables.content);
  const args = { ballotHash: variables.ballotId, electionId: variables.electionId, ballot };
  assert.ok(verifyBallotSignature(args));
  assert.ok(!verifyBallotSignature({ ...args, electionId: "another" }));
  assert.ok(!verifyBallotSignature({ ...args, ballot: { ...ballot, issue_date: "24/10/2026" } }));
  // The tracker of the vector is the digest of its hash and key.
  assert.equal(digestOfTrackingId(trackingId(variables.ballotId, ballot.voter_signing_pk)).length, 64);
});

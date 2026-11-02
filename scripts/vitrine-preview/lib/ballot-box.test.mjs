// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot box against a fake sidecar:  node --test scripts/vitrine-preview/lib/
import assert from "node:assert/strict";
import crypto from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import { createBallotBox } from "./ballot-box.mjs";
import { createBallotBox as createSigner, digestOfTrackingId, hashableBallotBytes, signCastStatement, trackingId } from "./receipts.mjs";
import { decodeEntry } from "./vitrine.mjs";

const VOTER = "voter-1";
const EVENT = "event-1";
const ELECTION = "election-1";
const signer = createSigner(Buffer.alloc(32, 7));
const stage = (entry) => decodeEntry(entry).fields[2].toString("ascii");
const sha256Hex = (text) => crypto.createHash("sha256").update(text).digest("hex");

const PKCS8 = Buffer.from("302e020100300506032b657004220420", "hex");
const voterSeed = Buffer.alloc(32, 9);
const voterKey = crypto.createPrivateKey({ key: Buffer.concat([PKCS8, voterSeed]), format: "der", type: "pkcs8" });
const voterPk = crypto.createPublicKey(voterKey).export({ format: "der", type: "spki" }).subarray(12).toString("base64");

const field = (part) => {
  const bytes = Buffer.isBuffer(part) ? part : Buffer.from(part, "utf8");
  const length = Buffer.alloc(8);
  length.writeBigUInt64LE(BigInt(bytes.length));
  return Buffer.concat([length, bytes]);
};

/** A ballot signed by the voter's key, the way the voting portal signs it. */
function signedBallot(issueDate = "2026-10-31") {
  const hashable = { version: 3, issue_date: issueDate, contests: ["Y29udGVzdA"], config: "style", ballot_style_hash: "abc" };
  const hash = sha256Hex(JSON.stringify(hashable));
  const message = Buffer.concat([field(hash), field(ELECTION), field(hashableBallotBytes(hashable))]);
  const signature = crypto.sign(null, message, voterKey).toString("base64");
  const ballot = { ...hashable, voter_signing_pk: voterPk, voter_ballot_signature: signature };
  return { hash, ballot, args: { election_id: ELECTION, ballot_id: hash, content: JSON.stringify(ballot) } };
}

function setup(electionId = ELECTION) {
  const added = [];
  const sidecar = {
    add: async (_log, entry) => {
      added.push(entry);
      return { status: 202, body: "queued" };
    },
  };
  const rows = [];
  const audited = { ballot_hash: "9020e384".padEnd(64, "0"), voter_signing_pk: null, config: { election_id: ELECTION, election_event_id: EVENT } };
  const box = createBallotBox({
    live: { eventId: EVENT, ballots: {} },
    sidecar,
    electionId,
    ids: { tenantId: "tenant-1", areaId: "area-1", voterId: VOTER },
    box: signer,
    audited,
    rows,
    log: () => undefined,
  });
  return { added, rows, box, audited };
}

test("a ballot is received once, then cast on the voter's signed request", async () => {
  const { added, rows, box } = setup();
  const { args, hash } = signedBallot();
  const received = await box.receive(args, VOTER);
  assert.equal(received.vitrine.kind, "queued");
  assert.equal(received.key_id, signer.keyId);
  assert.equal(signer.verifyReceived(received), received.ballot_id);
  assert.deepEqual(await box.receive(args, VOTER), received);
  assert.deepEqual(added.map(stage), ["received"]);
  const digest = digestOfTrackingId(trackingId(hash, voterPk));
  assert.equal(decodeEntry(added[0]).fields[3].toString("hex"), digest);

  assert.deepEqual(box.castVotes(VOTER, {}), []);
  assert.equal(box.locate(VOTER, trackingId(hash, voterPk)).status, "not-found");

  const key = received.ballot_id;
  const castSignature = signCastStatement(voterSeed, ELECTION, key);
  const wrong = signCastStatement(voterSeed, ELECTION, "AAAA-AAAA");
  await assert.rejects(box.cast({ election_id: ELECTION, ballot_id: key, cast_signature: wrong }, VOTER), (e) => e.code === "PokValidationFailed");
  await assert.rejects(box.cast({ election_id: ELECTION, ballot_id: "AAAA-AAAA", cast_signature: castSignature }, VOTER), (e) => e.code === "PokValidationFailed");

  const cast = await box.cast({ election_id: ELECTION, ballot_id: key.toLowerCase(), cast_signature: castSignature }, VOTER);
  assert.equal(cast.vitrine.kind, "queued");
  assert.deepEqual(added.map(stage), ["received", "cast"]);
  assert.equal(
    signer.verifyCastReceipt({ election_event_id: EVENT, election_id: ELECTION, ballot_id: key, received_at: received.received_at, cast_at: cast.cast_at, key_id: cast.key_id, cast_signature: castSignature, cast_receipt_signature: cast.cast_receipt_signature }),
    true,
  );
  const again = await box.cast({ election_id: ELECTION, ballot_id: key, cast_signature: castSignature }, VOTER);
  assert.deepEqual({ ...again, vitrine: null }, { ...cast, vitrine: null });
  assert.equal(again.vitrine, null);
  assert.equal(added.length, 2);
  assert.equal(rows.length, 1);

  const other = signCastStatement(Buffer.alloc(32, 3), ELECTION, key);
  await assert.rejects(box.cast({ election_id: ELECTION, ballot_id: key, cast_signature: other }, VOTER), (e) => e.code === "PokValidationFailed");
  await assert.rejects(box.audit({ election_id: ELECTION, ballot_id: key }, VOTER), (e) => e.code === "Conflict");

  const tracking = trackingId(hash, voterPk);
  assert.equal(box.castVotes(VOTER, { ballot_id_prefix: tracking.slice(0, 9).toLowerCase().replace("-", "") }).length, 1);
  assert.equal(box.locate(VOTER, tracking.slice(0, 9)).status, "found");
  assert.equal(box.locate(VOTER, tracking.slice(0, 9)).ballot_id, tracking);
  assert.equal(box.locate(VOTER, hash).ballot_id, tracking);
  assert.equal(box.locate(VOTER, tracking.slice(0, 4)).status, "not-found");
  assert.equal(box.castVotes("someone-else", {}).length, 0);
  const own = box.ownBallots(VOTER, {}).payload.ballots;
  assert.deepEqual(own.map((b) => [b.tracking_id, b.stage]), [[tracking, "cast"]]);
  assert.deepEqual(box.ownBallots("someone-else", {}).payload.ballots, []);
});

test("a ballot the voter did not sign is refused", async () => {
  const { box } = setup();
  const { args, ballot } = signedBallot();
  const refused = (e) => e.code === "PokValidationFailed";
  await assert.rejects(box.receive({ ...args, content: JSON.stringify({ ...ballot, issue_date: "2026-11-01" }) }, VOTER), refused);
  await assert.rejects(box.receive({ ...args, content: JSON.stringify({ ...ballot, voter_ballot_signature: undefined }) }, VOTER), refused);
  await assert.rejects(box.receive({ ...args, content: "not json" }, VOTER), refused);
  await assert.rejects(box.receive({ ...args, ballot_id: "xyz" }, VOTER), (e) => e.code === "BallotIdMismatch");
  await assert.rejects(box.receive({ ...args, election_id: "other" }, VOTER), (e) => e.code === "validation-failed");
});

test("an audited ballot cannot be cast", async () => {
  const { added, box } = setup();
  const { args } = signedBallot("2026-11-02");
  const { ballot_id: key } = await box.receive(args, VOTER);
  const audit = await box.audit({ election_id: ELECTION, ballot_id: key.toLowerCase() }, VOTER);
  assert.equal(audit.status, "audited");
  assert.equal(audit.vitrine.kind, "queued");
  assert.deepEqual(added.map(stage), ["received", "audited"]);
  assert.equal((await box.audit({ election_id: ELECTION, ballot_id: key }, VOTER)).vitrine, null);
  assert.equal(added.length, 2);
  const castSignature = signCastStatement(voterSeed, ELECTION, key);
  await assert.rejects(box.cast({ election_id: ELECTION, ballot_id: key, cast_signature: castSignature }, VOTER), (e) => e.code === "PokValidationFailed");
  await assert.rejects(box.audit({ election_id: ELECTION, ballot_id: "AAAA-AAAA" }, VOTER), (e) => e.code === "NotFound");
  assert.deepEqual(box.castVotes(VOTER, {}), []);
});

test("the audited fixture is recorded received, then audited, and found by its Tracking ID", async () => {
  const { added, box, audited } = setup();
  await box.recordAudited();
  assert.deepEqual(added.map(stage), ["received", "audited"]);
  const tracking = trackingId(audited.ballot_hash, null);
  assert.equal(box.locate(VOTER, tracking.slice(0, 9)).ballot_id, tracking);
  assert.equal(box.locate(VOTER, tracking.slice(0, 4)).status, "not-found");
  assert.equal(box.locate(VOTER, audited.ballot_hash).status, "found");
  assert.deepEqual(box.castVotes(VOTER, {}), []);
});

test("the direct path writes received, then cast", async () => {
  const { added, rows, box } = setup();
  const { args, hash } = signedBallot("2026-11-03");
  const row = await box.insert(args, VOTER);
  assert.deepEqual(added.map(stage), ["received", "cast"]);
  assert.equal(rows.length, 1);
  assert.equal(row.ballot_id, hash);
  assert.equal(await box.insert(args, VOTER), row);
  assert.equal(added.length, 2);
});

// A cast made by Step's voting portal (Vitrine's test vectors for Step), if the Vitrine checkout is there.
const VECTORS = `${process.env.VITRINE_DIR ?? `${process.env.HOME}/vitrine`}/packages/extension/test/vectors/step/casts.json`;
test("a ballot made by Step's portal is received", { skip: !(() => { try { readFileSync(VECTORS); return true; } catch { return false; } })() }, async () => {
  const vector = JSON.parse(readFileSync(VECTORS, "utf8")).cases.find((c) => c.name === "single-2-contests-signed");
  const { variables } = JSON.parse(vector.request.body);
  const { box } = setup(variables.electionId);
  const received = await box.receive({ election_id: variables.electionId, ballot_id: variables.ballotId, content: variables.content }, VOTER);
  assert.equal(signer.verifyReceived(received), received.ballot_id);
  assert.equal(received.ballot_hash, variables.ballotId);
});

test("a voter's audited ballot opens in the tracker once it is published, and only then", async () => {
  const { box } = setup();
  const { args, hash, ballot } = signedBallot("2026-11-01");
  const received = await box.receive(args, VOTER);
  const tracking = trackingId(hash, voterPk);
  const auditable = JSON.stringify({ ...ballot, ballot_hash: hash });
  assert.throws(() => box.publishAudited(auditable, VOTER), (e) => e.code === "NotFound");
  await box.audit({ election_id: ELECTION, ballot_id: received.ballot_id }, VOTER);
  assert.equal(box.locate(VOTER, tracking.slice(0, 9)).status, "not-found");
  assert.throws(() => box.publishAudited("not json", VOTER), (e) => e.code === "BadRequest");
  assert.throws(() => box.publishAudited(auditable, "someone-else"), (e) => e.code === "NotFound");
  assert.equal(box.publishAudited(auditable, VOTER), tracking);
  const found = box.locate(VOTER, tracking.slice(0, 9));
  assert.equal(found.status, "found");
  assert.equal(found.ballot_id, tracking);
  assert.equal(found.content, auditable);
  assert.equal(box.castVotes(VOTER, {}).length, 0);
});

test("the audit publishes the auditable ballot the portal sends with it, if it is the audited one", async () => {
  const { box } = setup();
  const { args, hash, ballot } = signedBallot("2026-11-06");
  const received = await box.receive(args, VOTER);
  const tracking = trackingId(hash, voterPk);
  const auditable = JSON.stringify({ ...ballot, ballot_hash: hash });
  // First the audit alone, as the portal does: nothing is published.
  const audited = await box.audit({ election_id: ELECTION, ballot_id: received.ballot_id }, VOTER);
  assert.equal(audited.status, "audited");
  assert.equal(audited.publication, "not-published");
  assert.equal(box.locatePublic(tracking).status, "not-found");

  // Another ballot's JSON, or none that parses, publishes nothing; the audit stands.
  const other = signedBallot("2026-11-07");
  const { ballot_id: otherKey } = await box.receive(other.args, VOTER);
  await box.audit({ election_id: ELECTION, ballot_id: otherKey }, VOTER);
  const otherAuditable = JSON.stringify({ ...other.ballot, ballot_hash: other.hash });
  for (const sent of [otherAuditable, "not json"]) {
    const refused = await box.audit({ election_id: ELECTION, ballot_id: received.ballot_id, auditable_ballot: sent }, VOTER);
    assert.equal(refused.status, "audited");
    assert.equal(refused.publication, "not-published");
  }
  assert.equal(box.locatePublic(tracking).status, "not-found");

  // Then again with its auditable ballot, once the box can no longer cast it.
  const published = await box.audit({ election_id: ELECTION, ballot_id: received.ballot_id, auditable_ballot: auditable }, VOTER);
  assert.equal(published.publication, "published");
  const found = box.locatePublic(tracking);
  assert.equal(found.status, "found");
  assert.equal(found.content, auditable);

  // Published content stays as first published.
  const again = await box.audit({ election_id: ELECTION, ballot_id: received.ballot_id, auditable_ballot: JSON.stringify({ ...JSON.parse(auditable), extra: 1 }) }, VOTER);
  assert.equal(again.publication, "published");
  assert.equal(box.locatePublic(tracking).content, auditable);
});

test("a ballot sent with its first audit is published in the same call", async () => {
  const { box } = setup();
  const { args, hash, ballot } = signedBallot("2026-11-08");
  const received = await box.receive(args, VOTER);
  const auditable = JSON.stringify({ ...ballot, ballot_hash: hash });
  const answer = await box.audit({ election_id: ELECTION, ballot_id: received.ballot_id, auditable_ballot: auditable }, VOTER);
  assert.equal(answer.publication, "published");
  assert.equal(box.locatePublic(trackingId(hash, voterPk)).content, auditable);
});

test("the public locator opens published audited ballots only, whoever's they are", async () => {
  const { box, audited } = setup();
  await box.recordAudited();
  const fixture = trackingId(audited.ballot_hash, null);
  assert.equal(box.locatePublic(fixture).ballot_id, fixture);
  assert.equal(box.locatePublic(fixture.replaceAll("-", "").toLowerCase()).status, "found");

  const auditedBallot = signedBallot("2026-11-04");
  const { ballot_id: auditKey } = await box.receive(auditedBallot.args, VOTER);
  await box.audit({ election_id: ELECTION, ballot_id: auditKey }, VOTER);
  const auditedTracking = trackingId(auditedBallot.hash, voterPk);
  assert.equal(box.locatePublic(auditedTracking).status, "not-found");
  const auditable = JSON.stringify({ ...auditedBallot.ballot, ballot_hash: auditedBallot.hash });
  box.publishAudited(auditable, VOTER);
  const found = box.locatePublic(auditedTracking);
  assert.equal(found.status, "found");
  assert.equal(found.content, auditable);

  const castBallot = signedBallot("2026-11-05");
  const { ballot_id: castKey } = await box.receive(castBallot.args, VOTER);
  await box.cast({ election_id: ELECTION, ballot_id: castKey, cast_signature: signCastStatement(voterSeed, ELECTION, castKey) }, VOTER);
  const castTracking = trackingId(castBallot.hash, voterPk);
  assert.equal(box.locate(VOTER, castTracking).status, "found");
  assert.equal(box.locatePublic(castTracking).status, "not-found");
  assert.equal(box.locatePublic(castBallot.hash).status, "not-found");
});

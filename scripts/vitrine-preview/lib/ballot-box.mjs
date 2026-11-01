// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot box of the preview's stand-in, as Step's Harvest and Windmill run it
// for an event whose receipts are signed by the ballot box: it receives a
// voter-signed ballot at review, casts it on the voter's signed request or
// marks it audited, and writes each stage to the live ballots log as a
// `vitrine/election/ballots/v1` entry (`received`, then `cast` or `audited`).
// It answers the portal with the ballot box's signed statements and the
// log's receipt, promise or refusal for the entry. Nothing here is Step.

import crypto from "node:crypto";
import { GraphQLError } from "./graphql.mjs";
import {
  digestOfTrackingId,
  normalizeBallotId,
  normalizeTrackingId,
  signedTime,
  trackingId,
  trackingIdStartsWith,
  verifyBallotSignature,
  verifyCastSignature,
} from "./receipts.mjs";
import { electionEntry, judge, unavailable } from "./vitrine.mjs";

const SUBMIT_TIMEOUT_MS = 600;
const SUBMIT_BUDGET_MS = 800;
const SEQUENCED_TIMEOUT_MS = 4000;
const SEQUENCED_BUDGET_MS = 6000;
export const MAX_LISTED_MATCHES = 10;

/** Every refusal of a ballot's state is the same 400 to the portal, as in Harvest. */
const refuse = (why) => new GraphQLError(why, "PokValidationFailed");

/** The ballot box of one election. `rows` is the table the cast votes are kept in. */
export function createBallotBox({ live, sidecar, electionId, ids, box, audited, rows, log, tamper }) {
  const byReceiptKey = new Map();
  const byBallot = new Map();
  const inFlight = new Map();
  let auditedRecord;

  /** Writes one stage of a ballot, named by its Tracking ID digest, to the ballots log. */
  async function submit(stage, digest, acceptedAt, { sequenced = false } = {}) {
    const entry = electionEntry({ eventId: live.eventId, electionId, stage, ballotId: digest, acceptedAt });
    let vitrine;
    try {
      const answer = await sidecar.add(
        live.ballots,
        entry,
        sequenced ? SEQUENCED_TIMEOUT_MS : SUBMIT_TIMEOUT_MS,
        sequenced ? SEQUENCED_BUDGET_MS : SUBMIT_BUDGET_MS,
      );
      vitrine = judge(answer.status, answer.body, entry);
    } catch (e) {
      log(`ballot box: the ballots log didn't answer the ${stage} entry (${e.message})`);
      vitrine = unavailable(entry, "log-unavailable");
    }
    log(`ballot box: ${stage} ${digest.slice(0, 12)}... at ${acceptedAt}; the log answered ${vitrine.kind}${vitrine.reason ? ` (${vitrine.reason})` : ""}`);
    return { entry, vitrine };
  }

  const once = (key, work) => {
    if (inFlight.has(key)) return inFlight.get(key);
    const running = work().finally(() => inFlight.delete(key));
    inFlight.set(key, running);
    return running;
  };

  function checkElection(requested) {
    if (requested !== electionId) throw new GraphQLError(`election ${requested} is not open for voting here`, "validation-failed");
  }

  // -- Receive --------------------------------------------------------------------

  const receivedAnswer = (record) => ({ ...record.received, vitrine: record.receivedVitrine });

  /** Verifies and stores a voter-signed ballot, signs the Received statement and
   * writes the `received` entry. Receiving the same ballot again returns the
   * same record, and does not write the entry again. */
  function receiveRecord({ election_id: requested, ballot_id: hash, content }, voter, options) {
    checkElection(requested);
    let ballot;
    try {
      ballot = JSON.parse(content);
    } catch {
      throw refuse("the ballot isn't JSON");
    }
    const voterPk = ballot?.voter_signing_pk;
    const voterSignature = ballot?.voter_ballot_signature;
    if (typeof voterPk !== "string" || typeof voterSignature !== "string") throw refuse("the ballot isn't signed by the voter");
    if (!/^[0-9a-f]{64}$/.test(hash)) throw new GraphQLError("the ballot_id isn't a ballot hash", "BallotIdMismatch");
    if (!verifyBallotSignature({ ballotHash: hash, electionId, ballot })) throw refuse("the voter's signature doesn't verify");

    return once(`${voter}:${hash}`, async () => {
      const held = byBallot.get(`${voter}:${hash}`);
      if (held) return held;
      const now = new Date();
      const received = box.signReceived({
        tenant_id: ids.tenantId,
        election_event_id: live.eventId,
        election_id: electionId,
        ballot_hash: hash,
        voter_signing_pk: voterPk,
        voter_ballot_signature: voterSignature,
        received_at: signedTime(now),
      });
      const tracking = trackingId(hash, voterPk);
      const digest = digestOfTrackingId(tracking);
      const { vitrine } = await submit("received", digest, Math.floor(now.getTime() / 1000), options);
      const record = { voter, hash, content, voterPk, tracking, digest, received, receivedVitrine: vitrine, state: "received" };
      byReceiptKey.set(received.ballot_id, record);
      byBallot.set(`${voter}:${hash}`, record);
      return record;
    });
  }

  /** `receive_ballot`: the portal's call at review. */
  async function receive(args, voter) {
    return receivedAnswer(await receiveRecord(args, voter));
  }

  // -- Cast -----------------------------------------------------------------------

  const castAnswer = (record, vitrine) => ({
    ballot_id: record.received.ballot_id,
    election_event_id: live.eventId,
    election_id: electionId,
    received_at: record.received.received_at,
    cast_signature: record.cast.cast_signature,
    cast_vote_id: record.cast.id,
    cast_at: record.cast.cast_at,
    key_id: record.cast.key_id,
    cast_receipt_signature: record.cast.cast_receipt_signature,
    vitrine,
  });

  function castRow(record, content = record.content) {
    return {
      id: record.cast.id,
      ballot_id: record.hash,
      election_id: electionId,
      election_event_id: live.eventId,
      tenant_id: ids.tenantId,
      area_id: ids.areaId,
      created_at: record.cast.cast_at,
      last_updated_at: record.cast.cast_at,
      labels: null,
      annotations: null,
      content,
      cast_ballot_signature: null,
      voter_id_string: record.voter,
      status: "valid",
      vitrine: record.cast.vitrine,
    };
  }

  /** The `cast` stage of a received ballot: the entry, and the signed Cast receipt
   * when the voter signed the request to cast (the direct path has none). */
  async function castStage(record, castSignature) {
    const now = new Date();
    const castAt = signedTime(now);
    const acceptedAt = Math.floor(now.getTime() / 1000);
    const receipt =
      castSignature &&
      box.signCastReceipt({
        election_event_id: live.eventId,
        election_id: electionId,
        ballot_id: record.received.ballot_id,
        received_at: record.received.received_at,
        cast_at: castAt,
        cast_signature: castSignature,
      });
    const { entry, vitrine } = await submit("cast", record.digest, acceptedAt);
    record.state = "cast";
    record.cast = { id: crypto.randomUUID(), cast_signature: castSignature, cast_at: castAt, key_id: receipt?.key_id, cast_receipt_signature: receipt?.cast_receipt_signature, vitrine };
    record.castRow = castRow(record);
    tamper?.remember?.({ ballotId: record.digest, entry, acceptedAt });
    return vitrine;
  }

  /** `cast_ballot`: verifies the voter's signed request to cast the received ballot,
   * writes the `cast` entry and signs the Cast receipt. The same request again
   * answers with the same receipt, and no log answer, as Harvest does. */
  async function cast({ election_id: requested, ballot_id: receiptKey, cast_signature: castSignature }, voter) {
    checkElection(requested);
    const key = normalizeBallotId(receiptKey);
    const record = key && byReceiptKey.get(key);
    if (!record || record.voter !== voter) throw refuse("the ballot box has not received this ballot");
    return once(`cast:${key}`, async () => {
      if (record.state === "audited") throw refuse("an audited ballot cannot be cast");
      if (record.state === "cast") {
        if (record.cast.cast_signature !== castSignature) throw refuse("the ballot was already cast");
        return castAnswer(record, null);
      }
      if (!verifyCastSignature(record.voterPk, electionId, key, castSignature)) throw refuse("the voter's cast signature doesn't verify");
      const vitrine = await castStage(record, castSignature);
      rows.push(record.castRow);
      return castAnswer(record, vitrine);
    });
  }

  // -- Audit ----------------------------------------------------------------------

  /** `audit_ballot`: marks a received ballot audited and writes the `audited` entry. */
  async function audit({ election_id: requested, ballot_id: receiptKey }, voter) {
    checkElection(requested);
    const key = normalizeBallotId(receiptKey);
    if (!key) throw new GraphQLError("Invalid Ballot ID", "BadRequest");
    const record = byReceiptKey.get(key);
    if (!record || record.voter !== voter) throw new GraphQLError("Ballot not found", "NotFound");
    return once(`audit:${key}`, async () => {
      if (record.state === "cast") throw new GraphQLError("A cast ballot cannot be audited", "Conflict");
      if (record.state === "audited") return { ballot_id: key, status: "audited", vitrine: null };
      const now = Math.floor(Date.now() / 1000);
      const { vitrine } = await submit("audited", record.digest, now);
      record.state = "audited";
      return { ballot_id: key, status: "audited", vitrine };
    });
  }

  // -- The direct path --------------------------------------------------------------

  /** `insert_cast_vote`: the direct cast of a ballot that was never received at
   * review. The ballot box receives it, waits for the log to sequence the
   * `received` entry (the profile's received-first rule), then casts it. */
  async function insert(args, voter) {
    const record = await receiveRecord(args, voter, { sequenced: true });
    return once(`insert:${record.received.ballot_id}`, async () => {
      if (record.state === "cast") return record.castRow;
      if (record.state === "audited") throw refuse("an audited ballot cannot be cast");
      await castStage(record, null);
      rows.push(record.castRow);
      return record.castRow;
    });
  }

  // -- Lookups ------------------------------------------------------------------------

  const matcher = (typed) => {
    const tracking = typed && normalizeTrackingId(typed);
    if (tracking) return (record) => trackingIdStartsWith(record.tracking, tracking);
    const hex = typeof typed === "string" ? typed.trim().toLowerCase() : "";
    return /^[0-9a-f]+$/.test(hex) ? (record) => record.hash === hex : undefined;
  };

  const castRecords = (voter) => [...byBallot.values()].filter((r) => r.voter === voter && r.state === "cast");

  /** The audited ballots the tracker can open: the fixture, and the voter's own
   * once its auditable ballot was published (`publishAudited`). */
  const publishedAudited = (voter) => [...byBallot.values()].filter((r) => r.voter === voter && r.state === "audited" && r.published);

  /** The cast ballots of the voter that the typed ID names, and the audited
   * ballots too (the fixture's and the voter's published ones). */
  function find(voter, typed) {
    const matches = matcher(typed);
    if (!matches) return [];
    return [...castRecords(voter), ...publishedAudited(voter), ...(auditedRecord ? [auditedRecord] : [])].filter(matches);
  }

  /** `get_voter_cast_votes`: the voter's cast ballots, those a typed ID names if it is given. */
  function castVotes(voter, { ballot_id: id, ballot_id_prefix: prefix }) {
    const typed = id ?? prefix;
    const records = typed ? find(voter, typed) : castRecords(voter);
    return records.map((r) => r.castRow);
  }

  /** The audited ballots anyone may open, as a hub's ballot verifier asks with no
   * voter's token: the fixture, and every voter's once it was published. Never a
   * cast ballot, nor an audited one its voter didn't publish. */
  function findPublic(typed) {
    const matches = matcher(typed);
    if (!matches) return [];
    const published = [...byBallot.values()].filter((r) => r.state === "audited" && r.published);
    return [...published, ...(auditedRecord ? [auditedRecord] : [])].filter(matches);
  }

  /** `locate_ballot` with a voter's token: found, not found, or ambiguous (listing
   * the Tracking IDs when few). */
  const locate = (voter, typed) => located(find(voter, typed));

  /** `locate_ballot` with no token (the public locator a hub's ballot verifier
   * asks): only the published audited ballots. */
  const locatePublic = (typed) => located(findPublic(typed));

  function located(found) {
    const none = { status: "not-found", ballot_id: null, content: null, cast_at: null, checks_available_until: null, matches: [] };
    if (found.length === 0) return none;
    if (found.length === 1) {
      const [only] = found;
      return { ...none, status: "found", ballot_id: only.tracking, content: only.content, cast_at: only.castRow.created_at };
    }
    return { ...none, status: "ambiguous", matches: found.length <= MAX_LISTED_MATCHES ? found.map((r) => r.tracking) : [] };
  }

  // -- The audited ballot -------------------------------------------------------------
  // Step's backend doesn't publish audited ballots. This stand-in does, for one:
  // it writes the ballot's `received` and `audited` entries to the ballots log
  // under its Tracking ID, and the tracker's lookup finds it by that ID.

  async function recordAudited() {
    if (audited.config?.election_id !== electionId || audited.config?.election_event_id !== live.eventId) {
      log("audited ballot: it is for another election; not recorded");
      return;
    }
    const tracking = trackingId(audited.ballot_hash, audited.voter_signing_pk);
    const digest = digestOfTrackingId(tracking);
    const now = Math.floor(Date.now() / 1000);
    await submit("received", digest, now, { sequenced: true });
    await submit("audited", digest, now);
    const at = new Date(now * 1000).toISOString();
    auditedRecord = {
      voter: ids.voterId,
      hash: audited.ballot_hash,
      content: JSON.stringify(audited),
      tracking,
      state: "audited",
      castRow: {
        id: crypto.randomUUID(),
        ballot_id: audited.ballot_hash,
        election_id: electionId,
        election_event_id: live.eventId,
        tenant_id: ids.tenantId,
        area_id: ids.areaId,
        created_at: at,
        last_updated_at: at,
        labels: null,
        annotations: null,
        content: JSON.stringify(audited),
        cast_ballot_signature: null,
        voter_id_string: ids.voterId,
        status: "valid",
      },
    };
    log(`audited ballot: ${tracking.slice(0, 9)} recorded at ${now}`);
  }

  /** Publishes the auditable ballot of a ballot the voter audited (the JSON the
   * Audit screen shows), so the tracker can open and decode it. Step's backend
   * doesn't publish audited ballots; this stand-in does, when a recording asks
   * (`POST /__preview/audited-ballot`). Nothing is trusted from it: the tracker
   * checks the ballot against the log itself. */
  function publishAudited(content, voter) {
    let ballot;
    try {
      ballot = JSON.parse(content);
    } catch {
      throw new GraphQLError("the audited ballot isn't JSON", "BadRequest");
    }
    const record = [...byBallot.values()].find((r) => r.voter === voter && r.state === "audited" && r.hash === ballot?.ballot_hash && r.voterPk === ballot?.voter_signing_pk);
    if (!record) throw new GraphQLError("the voter has no audited ballot with this hash", "NotFound");
    const at = new Date().toISOString();
    record.content = content;
    record.published = true;
    record.castRow = {
      id: crypto.randomUUID(),
      ballot_id: record.hash,
      election_id: electionId,
      election_event_id: live.eventId,
      tenant_id: ids.tenantId,
      area_id: ids.areaId,
      created_at: at,
      last_updated_at: at,
      labels: null,
      annotations: null,
      content,
      cast_ballot_signature: null,
      voter_id_string: voter,
      status: "valid",
    };
    log(`audited ballot: ${record.tracking.slice(0, 9)} published for the tracker`);
    return record.tracking;
  }

  return {
    receive,
    cast,
    audit,
    publishAudited,
    insert,
    castVotes,
    locate,
    locatePublic,
    recordAudited,
    stats: () => ({
      received: byReceiptKey.size,
      cast: rows.length,
      audited: [...byReceiptKey.values()].filter((r) => r.state === "audited").length,
      auditedFixture: auditedRecord?.tracking ?? null,
    }),
  };
}

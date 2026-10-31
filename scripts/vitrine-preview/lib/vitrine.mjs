// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The Vitrine objects Step's backend makes, written as Step writes them:
// entries (electoral-log/src/vitrine/entry.rs), config statements
// (statement.rs), signed notes (note.rs), receipt chains (proof.rs) and how
// Harvest reads a log's answer to a cast (harvest/src/services/vitrine.rs).

import crypto from "node:crypto";

const PKCS8_ED25519_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");
const SIG_ED25519 = 0x01;

export const ELECTION_TAG = "vitrine/election/ballots/v1";
export const EVENTS_TAG = "vitrine/election/events/v1";
export const CONFIG_TYPE = "vitrine/config/v1";
export const CHAIN_TYPE = "vitrine/receipt-chain/v1";
export const TLOG_PROOF = "c2sp.org/tlog-proof@v1\n";
export const PROMISE = "vitrine/promise/v1\n";
export const PORTAL_PACKAGE = "sequent/voting-portal";

export const sha256 = (...parts) => {
  const h = crypto.createHash("sha256");
  for (const p of parts) h.update(p);
  return h.digest();
};

/** RFC 6962 leaf hash, `SHA-256(0x00 || entry)`. */
export const leafHash = (entry) => sha256(Buffer.of(0), entry);

// -- Notes ---------------------------------------------------------------------

/** A note signer from a `PRIVATE+KEY+<name>+<id>+<base64>` key file. */
export function noteSigner(keyText) {
  // The base64 may itself hold `+`: only the first four are separators.
  const parts = keyText.trim().split("+");
  if (parts.length < 5 || parts[0] !== "PRIVATE" || parts[1] !== "KEY") {
    throw new Error("not a note signing key");
  }
  const [, , name, idHex] = parts;
  const data = parts.slice(4).join("+");
  const raw = Buffer.from(data, "base64");
  if (raw.length !== 33 || raw[0] !== SIG_ED25519) throw new Error("not an Ed25519 note key");
  const privateKey = crypto.createPrivateKey({
    key: Buffer.concat([PKCS8_ED25519_PREFIX, raw.subarray(1)]),
    format: "der",
    type: "pkcs8",
  });
  const spki = crypto.createPublicKey(privateKey).export({ format: "der", type: "spki" });
  const publicKey = spki.subarray(spki.length - 32);
  const keyId = sha256(Buffer.from(`${name}\n`), Buffer.of(SIG_ED25519), publicKey).subarray(0, 4);
  if (keyId.toString("hex") !== idHex.toLowerCase()) throw new Error("the key ID doesn't match the key");
  const vkey = `${name}+${idHex}+${Buffer.concat([Buffer.of(SIG_ED25519), publicKey]).toString("base64")}`;
  return {
    name,
    vkey,
    /** `text` (ending in a newline) as a signed note. */
    sign(text) {
      if (!text.endsWith("\n")) throw new Error("a note's text ends with a newline");
      const sig = crypto.sign(null, Buffer.from(text, "utf8"), privateKey);
      return `${text}\n— ${name} ${Buffer.concat([keyId, sig]).toString("base64")}\n`;
    },
  };
}

/** A note's text: everything before the blank line that starts its signatures. */
export function noteText(note) {
  const i = note.indexOf("\n\n");
  if (i < 0) throw new Error("not a signed note");
  return note.slice(0, i + 1);
}

/** The signature lines of a note, as [name, base64]. */
export function noteSignatures(note) {
  const i = note.indexOf("\n\n");
  if (i < 0) return [];
  return note
    .slice(i + 2)
    .split("\n")
    .filter((l) => l.startsWith("— "))
    .map((l) => l.slice(2).split(" "));
}

/** The policy ID of a signed policy note: SHA-256 of its text. */
export const policyId = (note) => `sha256:${sha256(Buffer.from(noteText(note), "utf8")).toString("hex")}`;

// -- Entries --------------------------------------------------------------------

const publicField = (value) => {
  const len = Buffer.alloc(2);
  len.writeUInt16BE(value.length);
  return Buffer.concat([Buffer.of(0), len, value]);
};

export function encodeEntry(tag, fields) {
  const body = Buffer.concat(fields.map(publicField));
  const head = Buffer.alloc(1 + tag.length + 2);
  head[0] = tag.length;
  head.write(tag, 1, "ascii");
  head.writeUInt16BE(body.length, 1 + tag.length);
  const entry = Buffer.concat([head, body]);
  if (entry.length > 65535) throw new Error("an entry has at most 65535 bytes");
  return entry;
}

const ID = /^[A-Za-z0-9._\-:/@+]{1,255}$/;
export const checkId = (id) => {
  if (!ID.test(id)) throw new Error(`not a Vitrine id: ${JSON.stringify(id)}`);
  return id;
};

const u64be = (n) => {
  const b = Buffer.alloc(8);
  b.writeBigUInt64BE(BigInt(n));
  return b;
};

export const STAGES = ["received", "cast", "audited"];

/** A `vitrine/election/ballots/v1` entry, as Step's ballot box encodes it: one
 * stage of a ballot, named by its Tracking ID digest (`ballotId`, in hex). */
export function electionEntry({ eventId, electionId, stage, ballotId, acceptedAt }) {
  if (!STAGES.includes(stage)) throw new Error(`unknown stage ${JSON.stringify(stage)}`);
  const ballot = Buffer.from(ballotId, "hex");
  if (ballot.length !== 32 || ballot.toString("hex") !== ballotId.toLowerCase()) {
    throw new Error("ballot_id isn't a 32-byte hash");
  }
  return encodeEntry(ELECTION_TAG, [
    Buffer.from(checkId(eventId), "ascii"),
    Buffer.from(checkId(electionId), "ascii"),
    Buffer.from(stage, "ascii"),
    ballot,
    u64be(acceptedAt),
  ]);
}

/** A `vitrine/election/events/v1` entry with no subject and no details. */
export function eventsEntry({ kind, time, statement }) {
  return encodeEntry(EVENTS_TAG, [
    Buffer.from(kind, "ascii"),
    u64be(time),
    Buffer.of(0),
    Buffer.from(statement ?? "", "utf8"),
    Buffer.alloc(0),
  ]);
}

/** The public fields of an entry. */
export function decodeEntry(entry) {
  const tagLen = entry[0];
  const tag = entry.subarray(1, 1 + tagLen).toString("ascii");
  const fieldsLen = entry.readUInt16BE(1 + tagLen);
  let off = 3 + tagLen;
  const end = off + fieldsLen;
  const fields = [];
  while (off < end) {
    const kind = entry[off];
    if (kind === 0) {
      const len = entry.readUInt16BE(off + 1);
      fields.push(entry.subarray(off + 3, off + 3 + len));
      off += 3 + len;
    } else {
      fields.push(entry.subarray(off + 1, off + 33));
      off += 33;
    }
  }
  return { tag, fields };
}

// -- Config statements ------------------------------------------------------------

/** `style_digest`: SHA-256 of the exact ballot style text. */
export const styleDigest = (style) => sha256(Buffer.from(style, "utf8"));

/** `style_list`: sorted, without repeats. */
export const styleList = (digests) =>
  [...new Map(digests.map((d) => [d.toString("hex"), d])).values()].sort(Buffer.compare);

export const stylesHash = (digests) => sha256(...styleList(digests));

/** `ConfigStatement::text`. */
export function configText(c) {
  checkId(c.eventId);
  checkId(c.electionId);
  if (!(c.opens < c.closes)) throw new Error("a config opens before it closes");
  let t =
    `${CONFIG_TYPE}\nevent ${c.eventId}\nelection ${c.electionId}\n` +
    `key ${c.key.toString("base64")}\nstyles ${c.styles.toString("base64")}\n` +
    `opens ${c.opens}\ncloses ${c.closes}\npolicy ${c.policyId}\n`;
  for (const r of c.releases) t += `release ${r.package} ${r.version}\n`;
  t += `transparency ${c.transparency}\ndemo ${c.demo ? "yes" : "no"}\ntime ${c.time}\n`;
  return t;
}

// -- Receipts, promises and chains --------------------------------------------------

const lineValue = (text, key) => {
  const line = text.split("\n").find((l) => l.startsWith(`${key} `));
  return line === undefined ? undefined : line.slice(key.length + 1);
};

/** What a log answered: a receipt with the entry it proves, or a promise
 * with the leaf hash it promises (electoral-log's `Submitted::parse`). */
export function parseSubmitted(body) {
  if (body.startsWith(TLOG_PROOF)) {
    const extra = lineValue(body, "extra");
    const index = lineValue(body, "index");
    return extra === undefined
      ? undefined
      : { kind: "receipt", entry: Buffer.from(extra, "base64"), index: index && Number(index) };
  }
  if (body.startsWith(PROMISE)) {
    const leaf = lineValue(body, "leaf");
    return leaf === undefined ? undefined : { kind: "promise", leaf: Buffer.from(leaf, "base64") };
  }
  return undefined;
}

export const isFor = (submitted, entry) =>
  submitted !== undefined &&
  (submitted.kind === "receipt"
    ? submitted.entry.equals(entry)
    : submitted.leaf.equals(leafHash(entry)));

/** The checkpoint note a receipt carries. */
export function receiptCheckpoint(receipt) {
  const lines = receipt.split("\n");
  const i = lines.findIndex((l) => l === "checkpoint" || l.startsWith("checkpoint "));
  if (i < 0) return "";
  return lines.slice(i + 1).join("\n");
}

/** `receipt_chain`. */
export function receiptChain(policy, log, blocks) {
  let out = `${CHAIN_TYPE}\npolicy ${policy}\nlog ${log}\n`;
  for (const b of blocks) out += `${b.kind} ${Buffer.byteLength(b.data, "utf8")}\n${b.data}\n`;
  return out;
}

/** Harvest's `judge`: what the log's answer means for the cast. */
export function judge(status, body, entry) {
  const submitted = parseSubmitted(body);
  const own = isFor(submitted, entry) ? submitted : undefined;
  if (status === 200 && own?.kind === "receipt") return { kind: "receipt", text: body, reason: null };
  if (status === 202 && own?.kind === "promise") return { kind: "promise", text: body, reason: null };
  if (status === 202 && own === undefined && body.trimEnd() === "queued") {
    return { kind: "queued", text: entry.toString("base64"), reason: "queued" };
  }
  const reason =
    status === 403
      ? "frozen"
      : status === 404
        ? "unknown-entry"
        : status === 422
          ? "rejected"
          : status === 429
            ? "busy"
            : status >= 500 && status <= 599
              ? "log-unavailable"
              : "bad-response";
  return unavailable(entry, reason);
}

export const unavailable = (entry, reason) => ({
  kind: "unavailable",
  text: entry ? entry.toString("base64") : null,
  reason,
});

// -- Step's ballot ID -----------------------------------------------------------------

const B64_NO_PAD = /^[A-Za-z0-9+/]*$/;

function decodeB64NoPad(text) {
  if (typeof text !== "string" || !B64_NO_PAD.test(text) || text.length % 4 === 1) {
    throw new Error("not canonical base64 without padding");
  }
  const bytes = Buffer.from(text, "base64");
  if (bytes.toString("base64").replace(/=+$/, "") !== text) throw new Error("not canonical base64");
  return bytes;
}

const u32le = (n) => {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(n);
  return b;
};

/** The hex ballot ID Step computes from a cast's `content`: the first 32 bytes
 * of the SHA-512 of its hashable ballot. */
export function stepBallotId(content) {
  const ballot = JSON.parse(content);
  if (ballot?.version !== 2 || typeof ballot.issue_date !== "string") {
    throw new Error("not a version 2 hashable ballot");
  }
  const date = Buffer.from(ballot.issue_date, "utf8");
  const parts = [u32le(2), u32le(date.length), date];
  if (Array.isArray(ballot.contests)) {
    parts.push(u32le(ballot.contests.length));
    for (const c of ballot.contests) parts.push(decodeB64NoPad(c));
  } else if (typeof ballot.contests === "string") {
    parts.push(decodeB64NoPad(ballot.contests));
  } else {
    throw new Error("a hashable ballot has contests");
  }
  return crypto.createHash("sha512").update(Buffer.concat(parts)).digest().subarray(0, 32).toString("hex");
}

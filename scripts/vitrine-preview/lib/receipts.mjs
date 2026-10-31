// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the ballot box signs and the IDs that follow, written as Step's
// sequent-core writes them (`ballot_receipt.rs`): the Received statement, the
// receipt key (`ballot_id`), the Cast statement and the Cast receipt, the
// Tracking ID, and how typed IDs are read. The keys are Ed25519; a public key
// is read as 32 raw bytes or as a SubjectPublicKeyInfo, both in base64.

import crypto from "node:crypto";

const RECEIVED_STATEMENT_DOMAIN = "step/ballot-received/v1";
const BALLOT_ID_DOMAIN = "step/ballot-id/v1";
const TRACKING_ID_DOMAIN = "step/ballot-id/v2";
const CAST_STATEMENT_DOMAIN = "step/ballot-cast/v1";
const CAST_RECEIPT_DOMAIN = "step/ballot-cast-receipt/v1";

const KEY_ID_BYTES = 8;
const BALLOT_ID_BYTES = 5;
const BALLOT_ID_CHARACTERS = 8;
const GROUP = 4;
const SEPARATOR = "-";
export const TRACKING_ID_BYTES = 32;
export const TRACKING_ID_CHARACTERS = 52;
export const SHORT_TRACKING_ID_CHARACTERS = 8;
/** Crockford's base32: no I, L, O or U. */
const ALPHABET = "0123456789ABCDEFGHJKMNPQRSTVWXYZ";

const SPKI_ED25519_PREFIX = Buffer.from("302a300506032b6570032100", "hex");
const PKCS8_ED25519_PREFIX = Buffer.from("302e020100300506032b657004220420", "hex");

export class ReceiptError extends Error {
  constructor(code, message) {
    super(message ?? code);
    this.code = code;
  }
}

const sha256 = (bytes) => crypto.createHash("sha256").update(bytes).digest();
const sha512 = (bytes) => crypto.createHash("sha512").update(bytes).digest();

const STRICT_B64 = /^[A-Za-z0-9+/]*={0,2}$/;

function b64Bytes(text, field) {
  if (typeof text !== "string" || text.length % 4 !== 0 || !STRICT_B64.test(text)) {
    throw new ReceiptError("malformed", `${field}: invalid base64`);
  }
  return Buffer.from(text, "base64");
}

/** Length-prefixed like `extend_with_field`: a u64 little-endian length, then the bytes. */
function fields(...parts) {
  const out = [];
  for (const part of parts) {
    const bytes = Buffer.isBuffer(part) ? part : Buffer.from(part, "utf8");
    const len = Buffer.alloc(8);
    len.writeBigUInt64LE(BigInt(bytes.length));
    out.push(len, bytes);
  }
  return Buffer.concat(out);
}

/** The 32 raw bytes of a public key given as raw bytes or a SubjectPublicKeyInfo. */
export function publicKeyBytes(b64, field = "voter_signing_pk") {
  const bytes = b64Bytes(b64, field);
  if (bytes.length === 32) return bytes;
  if (bytes.length === 44 && bytes.subarray(0, 12).equals(SPKI_ED25519_PREFIX)) return bytes.subarray(12);
  throw new ReceiptError("malformed", `${field}: not an Ed25519 public key`);
}

export const publicKeyDer = (raw) => Buffer.concat([SPKI_ED25519_PREFIX, raw]);

export function signatureBytes(b64, field) {
  const bytes = b64Bytes(b64, field);
  if (bytes.length !== 64) throw new ReceiptError("malformed", `${field}: an Ed25519 signature has 64 bytes`);
  return bytes;
}

function verify(publicKeyRaw, message, signature) {
  try {
    const key = crypto.createPublicKey({ key: publicKeyDer(publicKeyRaw), format: "der", type: "spki" });
    return crypto.verify(null, message, key, signature);
  } catch {
    return false;
  }
}

// -- Tracking IDs and receipt keys ---------------------------------------------------

/** Crockford base32 of `bytes`, most significant bit first, the last character padded with zeros. */
function base32(bytes) {
  let bits = 0;
  let held = 0;
  let out = "";
  for (const byte of bytes) {
    bits = ((bits << 8) | byte) >>> 0;
    held += 8;
    while (held >= 5) {
      held -= 5;
      out += ALPHABET[(bits >>> held) & 0x1f];
    }
    bits &= (1 << held) - 1;
  }
  if (held > 0) out += ALPHABET[(bits << (5 - held)) & 0x1f];
  return out;
}

const grouped = (chars) => chars.match(/.{1,4}/g)?.join(SEPARATOR) ?? "";

/** The first 32 bytes of SHA-512 over the length-prefixed domain `step/ballot-id/v2`,
 * the ballot hash and the voter's key as DER (nothing when the voter did not sign). */
export function trackingIdDigest(ballotHash, voterSigningPk) {
  const key = voterSigningPk ? publicKeyDer(publicKeyBytes(voterSigningPk)) : Buffer.alloc(0);
  return sha512(fields(TRACKING_ID_DOMAIN, ballotHash, key)).subarray(0, TRACKING_ID_BYTES);
}

/** The digest in groups of four: `FTBE-MHRX-...`, 52 characters. */
export const formatTrackingId = (digest) => grouped(base32(digest));

export const trackingId = (ballotHash, voterSigningPk) => formatTrackingId(trackingIdDigest(ballotHash, voterSigningPk));

export const shortTrackingId = (id) => grouped(id.replaceAll(SEPARATOR, "").slice(0, SHORT_TRACKING_ID_CHARACTERS));

/** What a person typed, as Crockford characters: any case, hyphens and spaces
 * ignored, O as 0, I and L as 1. Undefined when a character is not in the alphabet. */
export function crockfordCharacters(typed) {
  let out = "";
  for (const raw of String(typed)) {
    if (raw === SEPARATOR || /\s/.test(raw)) continue;
    const c = raw.toUpperCase();
    const read = c === "O" ? "0" : c === "I" || c === "L" ? "1" : c;
    if (read.length !== 1 || !ALPHABET.includes(read)) return undefined;
    out += read;
  }
  return out;
}

/** A typed Tracking ID, or its start (8 to 52 characters), in groups of four. */
export function normalizeTrackingId(typed) {
  const chars = crockfordCharacters(typed);
  if (chars === undefined || chars.length < SHORT_TRACKING_ID_CHARACTERS || chars.length > TRACKING_ID_CHARACTERS) return undefined;
  return grouped(chars);
}

/** A typed receipt key: exactly 8 characters, as `FTBE-MHRX`. */
export function normalizeBallotId(typed) {
  const chars = crockfordCharacters(typed);
  if (chars === undefined || chars.length !== BALLOT_ID_CHARACTERS) return undefined;
  return `${chars.slice(0, GROUP)}${SEPARATOR}${chars.slice(GROUP)}`;
}

export const trackingIdStartsWith = (id, typed) =>
  id.replaceAll(SEPARATOR, "").startsWith(typed.replaceAll(SEPARATOR, ""));

/** The Tracking ID's digest as 64 lowercase hex digits (the ballots log's `ballot_id`). */
export function digestOfTrackingId(id) {
  const chars = crockfordCharacters(id);
  if (chars === undefined || chars.length !== TRACKING_ID_CHARACTERS) throw new ReceiptError("malformed", "not a full Tracking ID");
  let bits = 0n;
  for (const c of chars) bits = (bits << 5n) | BigInt(ALPHABET.indexOf(c));
  // 52 characters are 260 bits; the digest is the first 256.
  return (bits >> 4n).toString(16).padStart(64, "0");
}

// -- The voter's signature on a ballot -------------------------------------------------

const u32le = (n) => {
  const b = Buffer.alloc(4);
  b.writeUInt32LE(n);
  return b;
};
const borshString = (text) => {
  const bytes = Buffer.from(text, "utf8");
  return Buffer.concat([u32le(bytes.length), bytes]);
};

/** Borsh of a `HashableBallot`: version, issue date, contests, ballot style ID and hash. */
export const hashableBallotBytes = (ballot) =>
  Buffer.concat([
    u32le(ballot.version),
    borshString(ballot.issue_date),
    u32le(ballot.contests.length),
    ...ballot.contests.map(borshString),
    borshString(ballot.config),
    borshString(ballot.ballot_style_hash),
  ]);

/** Whether the voter's key signed this ballot, as `verify_ballot_signature` checks it: the
 * signature covers the ballot's hash, the election and the borsh of its hashable ballot. */
export function verifyBallotSignature({ ballotHash, electionId, ballot }) {
  try {
    const message = fields(ballotHash, electionId, hashableBallotBytes(ballot));
    return verify(publicKeyBytes(ballot.voter_signing_pk), message, signatureBytes(ballot.voter_ballot_signature, "voter_ballot_signature"));
  } catch {
    return false;
  }
}

// -- The Received statement ----------------------------------------------------------

function receivedBytes(s) {
  return fields(
    RECEIVED_STATEMENT_DOMAIN,
    s.tenant_id,
    s.election_event_id,
    s.election_id,
    s.ballot_hash,
    publicKeyDer(publicKeyBytes(s.voter_signing_pk)),
    signatureBytes(s.voter_ballot_signature, "voter_ballot_signature"),
    s.received_at,
    s.key_id,
  );
}

/** The ballot box's key for a received ballot: the first 40 bits of SHA-512 over
 * the voter-signed ballot and the ballot box's signature. */
function receiptKey(s, receivedSignature) {
  const prefix = sha512(
    fields(
      BALLOT_ID_DOMAIN,
      s.ballot_hash,
      publicKeyDer(publicKeyBytes(s.voter_signing_pk)),
      signatureBytes(s.voter_ballot_signature, "voter_ballot_signature"),
      s.received_at,
      s.key_id,
      signatureBytes(receivedSignature, "received_signature"),
    ),
  ).subarray(0, BALLOT_ID_BYTES);
  const chars = base32(prefix);
  return `${chars.slice(0, GROUP)}${SEPARATOR}${chars.slice(GROUP)}`;
}

// -- The Cast statement and the Cast receipt ------------------------------------------

const castStatementBytes = (electionId, ballotId) => fields(CAST_STATEMENT_DOMAIN, electionId, ballotId);

/** Signs "cast this Ballot ID" with a voter key (a 32-byte seed). Used by checks. */
export function signCastStatement(seed, electionId, ballotId) {
  const key = crypto.createPrivateKey({ key: Buffer.concat([PKCS8_ED25519_PREFIX, seed]), format: "der", type: "pkcs8" });
  return crypto.sign(null, castStatementBytes(electionId, ballotId), key).toString("base64");
}

/** Whether the key that signed the received ballot asks to cast it. */
export function verifyCastSignature(voterSigningPk, electionId, ballotId, castSignature) {
  try {
    return verify(publicKeyBytes(voterSigningPk), castStatementBytes(electionId, ballotId), signatureBytes(castSignature, "cast_signature"));
  } catch {
    return false;
  }
}

function castReceiptBytes(s) {
  return fields(
    CAST_RECEIPT_DOMAIN,
    s.election_event_id,
    s.election_id,
    s.ballot_id,
    s.received_at,
    s.cast_at,
    s.key_id,
    sha256(signatureBytes(s.cast_signature, "cast_signature")),
  );
}

/** UTC, RFC 3339 with milliseconds: 2028-05-08T03:00:00.000Z. */
export const signedTime = (date) => new Date(date).toISOString();

// -- The ballot box key ----------------------------------------------------------------

/** The published form of a ballot box key: the first 8 bytes of SHA-256 of its
 * SubjectPublicKeyInfo name it. */
export function publishedKey(publicKeyRaw) {
  return {
    key_id: sha256(publicKeyDer(publicKeyRaw)).subarray(0, KEY_ID_BYTES).toString("hex"),
    public_key: publicKeyRaw.toString("base64"),
  };
}

/** A ballot box that signs with an Ed25519 key from a 32-byte seed. */
export function createBallotBox(seed) {
  const privateKey = crypto.createPrivateKey({ key: Buffer.concat([PKCS8_ED25519_PREFIX, seed]), format: "der", type: "pkcs8" });
  const spki = crypto.createPublicKey(privateKey).export({ format: "der", type: "spki" });
  const raw = spki.subarray(spki.length - 32);
  const key = publishedKey(raw);
  const sign = (bytes) => crypto.sign(null, bytes, privateKey).toString("base64");
  return {
    key,
    keyId: key.key_id,
    /** Signs a Received statement (without `key_id`, which is the box's) and keys it. */
    signReceived(statement) {
      const full = { ...statement, key_id: key.key_id };
      const received_signature = sign(receivedBytes(full));
      return { ...full, received_signature, ballot_id: receiptKey(full, received_signature) };
    },
    /** Signs a Cast receipt statement (without `key_id`). */
    signCastReceipt(statement) {
      const full = { ...statement, key_id: key.key_id };
      return { ...full, cast_receipt_signature: sign(castReceiptBytes(full)) };
    },
    verifyReceived(received) {
      const ok = received.key_id === key.key_id && verify(raw, receivedBytes(received), signatureBytes(received.received_signature, "received_signature"));
      return ok && receiptKey(received, received.received_signature) === received.ballot_id ? received.ballot_id : undefined;
    },
    verifyCastReceipt(receipt) {
      return receipt.key_id === key.key_id && verify(raw, castReceiptBytes(receipt), signatureBytes(receipt.cast_receipt_signature, "cast_receipt_signature"));
    },
  };
}

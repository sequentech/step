// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The stack's events log, read as the booth reads it (Vitrine's
// specs/entry.md, sections 4 and 5): the election's config statement and the
// event's voting-opened milestone, each with the receipt that proves it, and
// what Step's backend does with them (windmill writes the statements, Harvest
// serves them to the booth as `vitrine_voter_context`).

import { configText, decodeEntry, leafHash, noteSignatures, noteText, receiptChain, EVENTS_TAG } from "./vitrine.mjs";

const TILE_WIDTH = 256;
const MAX_BUNDLES = 40;

async function get(url, timeoutMs = 8000) {
  const res = await fetch(url, { redirect: "error", signal: AbortSignal.timeout(timeoutMs) });
  if (res.status === 404) return undefined;
  if (res.status !== 200) throw new Error(`${url} answered ${res.status}`);
  return Buffer.from(await res.arrayBuffer());
}

const tilePath = (n) => {
  // C2SP tlog-tiles index path: groups of three digits, `x` prefixes above the last group.
  const digits = String(n).padStart(3, "0");
  const groups = [];
  for (let i = digits.length; i > 0; i -= 3) groups.unshift(digits.slice(Math.max(0, i - 3), i));
  while (groups.length > 1 && groups[0] === "000") groups.shift();
  return groups.map((g, i) => (i < groups.length - 1 ? `x${g}` : g)).join("/");
};

/** The size of a checkpoint note's tree. */
export const checkpointSize = (note) => Number(note.split("\n")[1]);

/** Every entry of the log up to its latest checkpoint, as raw bytes. */
export async function readEntries(logUrl) {
  const base = logUrl.endsWith("/") ? logUrl : `${logUrl}/`;
  const cp = await get(`${base}checkpoint`);
  if (!cp) return { size: 0, entries: [] };
  const size = checkpointSize(cp.toString("utf8"));
  const entries = [];
  for (let n = 0; n * TILE_WIDTH < size && n < MAX_BUNDLES; n++) {
    const width = Math.min(TILE_WIDTH, size - n * TILE_WIDTH);
    let bundle = await get(`${base}tile/entries/${tilePath(n)}${width < TILE_WIDTH ? `.p/${width}` : ""}`);
    bundle ??= await get(`${base}tile/entries/${tilePath(n)}`);
    if (!bundle) throw new Error(`entry bundle ${n} is missing`);
    let at = 0;
    while (at + 2 <= bundle.length && entries.length < (n + 1) * TILE_WIDTH) {
      const len = bundle.readUInt16BE(at);
      entries.push(bundle.subarray(at + 2, at + 2 + len));
      at += 2 + len;
    }
  }
  return { size, entries: entries.slice(0, size) };
}

const field = (fields, i) => fields[i]?.toString("utf8");

/** What the events log holds about an election: its config statements and the
 * log's voting-opened milestones, with each entry's leaf hash and position. */
export function readEventsLog(entries) {
  const out = { configs: [], opened: [], milestones: [], other: [] };
  entries.forEach((raw, index) => {
    let decoded;
    try {
      decoded = decodeEntry(raw);
    } catch {
      return;
    }
    if (decoded.tag !== EVENTS_TAG) return;
    const kind = field(decoded.fields, 0);
    const statement = field(decoded.fields, 3) ?? "";
    let text = "";
    try {
      text = noteText(statement);
    } catch {
      text = "";
    }
    const item = { index, raw, leaf: leafHash(raw), kind, statement, text, signers: statement ? noteSignatures(statement).map(([name]) => name) : [] };
    if (kind === "config") out.configs.push(item);
    else if (kind === "voting-opened") out.opened.push(item);
    else out.other.push(item);
  });
  return out;
}

const lines = (text) => text.split("\n").filter(Boolean);
const line = (text, key) => lines(text).find((l) => l.startsWith(`${key} `))?.slice(key.length + 1);

/** The config statement in force for an election: the one the first
 * voting-opened milestone names, if the log has both. */
export function configInForce(log, eventId, electionId) {
  const opened = log.opened[0];
  if (!opened) return { reason: "the events log has no voting-opened milestone yet" };
  if (line(opened.text, "event") !== eventId) return { opened, reason: `the milestone is for the event ${line(opened.text, "event")}` };
  const named = lines(opened.text).find((l) => l.startsWith(`config ${electionId} `));
  if (!named) {
    const elections = lines(opened.text).filter((l) => l.startsWith("config ")).map((l) => l.split(" ")[1]);
    return { opened, reason: `the voting-opened milestone names no configuration for ${electionId} (it names ${elections.join(", ") || "none"})` };
  }
  const leaf = Buffer.from(named.split(" ")[2], "base64");
  const config = log.configs.find((c) => c.leaf.equals(leaf));
  if (!config) return { opened, reason: `the configuration the milestone names for ${electionId} isn't in the log` };
  return { opened, config };
}

/** `GET receipt/<leaf>` of the events log: the receipt that meets the quorum. */
export async function receiptFor(live, leaf) {
  const base = live.events.url.endsWith("/") ? live.events.url : `${live.events.url}/`;
  const body = await get(`${base}receipt/${leaf.toString("hex")}`);
  return body?.toString("utf8");
}

const logged = async (live, item) => {
  const receipt = await receiptFor(live, item.leaf);
  if (!receipt) return null;
  return {
    statement: item.statement,
    entry: item.raw.toString("base64"),
    chain: receiptChain(live.policyId, live.events.name, [{ kind: "issued", data: receipt }]),
  };
};

/** `vitrine_voter_context` for the election: the real statements, or what is
 * missing from the log. */
export async function voterContext({ live, electionId, styleHashes, release }) {
  const log = readEventsLog((await readEntries(live.events.url)).entries);
  const inForce = configInForce(log, live.eventId, electionId);
  const context = {
    mode: "public",
    policy: null,
    policy_ack: null,
    config: null,
    opened: null,
    style_hashes: styleHashes,
    release: release.version,
  };
  if (inForce.config) context.config = await logged(live, inForce.config);
  if (inForce.opened) context.opened = await logged(live, inForce.opened);
  return { context, problem: inForce.reason };
}

/** The config statement lines Step's windmill signs for an election. */
export function configFor({ live, electionId, key, styles, release, time }) {
  return configText({
    eventId: live.eventId,
    electionId,
    key,
    styles,
    opens: live.opens,
    closes: live.closes,
    policyId: live.policyId,
    releases: [release],
    transparency: live.visibility,
    demo: true,
    time,
  });
}

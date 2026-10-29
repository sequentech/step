// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The live Vitrine stack, read at start-up from its deploy/.state: the sealed
// policy, the logs' internal names, tokens and acknowledgements, the
// authority key the stack signs the events log's statements with, and the
// demo booth's voting-opened milestone. Nothing here is hardcoded: a
// re-provisioned stack is read again by restarting the harness.

import fs from "node:fs";
import path from "node:path";
import { noteSigner, noteText, policyId } from "./vitrine.mjs";

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;

const read = (file) => fs.readFileSync(file, "utf8");

/** The `key = "value"` pairs of a flat TOML file, by section; `[[array]]`
 * sections give one object each. Enough for the stack's own files. */
export function readToml(text) {
  const out = {};
  let target = out;
  for (const raw of text.split("\n")) {
    const line = raw.replace(/\s+#.*$/, "").trim();
    if (line === "" || line.startsWith("#")) continue;
    const array = line.match(/^\[\[([^\]]+)\]\]$/);
    const table = line.match(/^\[([^\]]+)\]$/);
    if (array) {
      out[array[1]] ??= [];
      target = {};
      out[array[1]].push(target);
    } else if (table) {
      target = out[table[1]] ??= {};
    } else {
      const kv = line.match(/^([A-Za-z0-9_]+)\s*=\s*(.*)$/);
      if (!kv) continue;
      let value = kv[2];
      try {
        value = JSON.parse(value);
      } catch {
        // A bare value: keep its text.
      }
      target[kv[1]] = value;
    }
  }
  return out;
}

/** A policy's lines, by keyword. */
export function policyLines(policy) {
  const lines = new Map();
  for (const line of noteText(policy).split("\n").slice(1)) {
    if (line === "") continue;
    const [key, ...rest] = line.split(" ");
    if (!lines.has(key)) lines.set(key, []);
    lines.get(key).push(rest);
  }
  return lines;
}

export function readStack(vitrineDir) {
  const state = path.join(vitrineDir, "deploy", ".state");
  const stateFile = (p) => path.join(state, p.replace(/^\/state\//, ""));
  const stack = JSON.parse(read(path.join(state, "public", "stack.json")));
  const policy = read(path.join(state, "public", "policy"));
  const id = policyId(policy);
  if (stack.policy_id && stack.policy_id !== id) {
    throw new Error(`stack.json names policy ${stack.policy_id}, the policy file is ${id}`);
  }
  const lines = policyLines(policy);
  const one = (key) => lines.get(key)?.[0]?.[0];
  const params = new Map();
  for (const [log, key, ...values] of lines.get("param") ?? []) params.set(`${log} ${key}`, values);

  const logConfig = readToml(read(path.join(state, "log", "config.toml")));
  const internalUrl = new URL(logConfig.server.internal_url);
  const logs = {};
  for (const [name, vkey, url] of lines.get("log") ?? []) {
    const origin = vkey.split("+")[0];
    const sidecar = logConfig.logs.find((l) => l.origin === origin);
    if (!sidecar) throw new Error(`the log server has no log with origin ${origin}`);
    const short = path.basename(sidecar.token_file, ".token");
    const ackFile = path.join(state, "log", `${short}.policy-ack.json`);
    const enrolmentFile = path.join(state, "log", `${short}.enrolment`);
    const ack = fs.existsSync(ackFile) ? JSON.parse(read(ackFile)) : undefined;
    logs[name] = {
      name,
      origin,
      vkey,
      url,
      sidecar: sidecar.name,
      token: read(stateFile(sidecar.token_file)).trim(),
      ack: ack && ack.policy_id === id ? ack.ack : undefined,
      enrolment: fs.existsSync(enrolmentFile) ? read(enrolmentFile) : undefined,
      profile: (lines.get("profile") ?? []).find(([log]) => log === name)?.[1],
    };
  }
  const ballots = Object.values(logs).find((l) => l.profile === "vitrine/election/ballots/v1");
  const events = Object.values(logs).find((l) => l.profile === "vitrine/election/events/v1");
  if (!ballots || !events) throw new Error("the policy has no election ballots and events logs");

  const boothDir = path.join(state, "booth");
  const booth = readToml(read(path.join(boothDir, "config.toml")));
  // The booth writes its setup when it first starts; provision-events runs before that.
  const setupFile = path.join(boothDir, booth.setup.state_file);
  const setup = fs.existsSync(setupFile) ? JSON.parse(read(setupFile)) : { election: booth.election };
  const signer = noteSigner(read(path.join(boothDir, booth.setup.authority_key)));
  const authorities = (lines.get("authority") ?? []).map(([, vkey]) => vkey);
  if (!authorities.includes(signer.vkey)) {
    throw new Error(`the booth's authority key ${signer.vkey} isn't one of the policy's authorities`);
  }

  const eventId = params.get(`${ballots.name} event-id`)?.[0];
  const electionIds = params.get(`${ballots.name} election-ids`) ?? [];
  if (!eventId || electionIds.length === 0) throw new Error("the ballots log has no event-id or election-ids");

  return {
    stateDir: state,
    stack,
    policy,
    policyId: id,
    lines,
    visibility: one("visibility"),
    opens: Number(one("opens")),
    closes: Number(one("closes")),
    logs,
    ballots,
    events,
    internal: { host: internalUrl.hostname, port: Number(internalUrl.port || 80) },
    adminToken: read(stateFile(logConfig.server.admin_token_file)).trim(),
    signer,
    eventId,
    electionIds,
    demoBooth: { election: setup.election, event: setup.event, opened: setup.opened, config: setup.config },
    explorer: stack.apps?.explorer,
  };
}

/** The election Step's portal votes in: `preferred` if given, else the
 * first of the ballots log's elections that is a UUID (Step's own IDs), else
 * the first one the demo booth doesn't use. */
export function pickElection(live, preferred) {
  if (preferred) {
    if (!live.electionIds.includes(preferred)) {
      throw new Error(`${preferred} isn't one of the ballots log's election-ids: ${live.electionIds.join(" ")}`);
    }
    return preferred;
  }
  const others = live.electionIds.filter((e) => e !== live.demoBooth.election);
  const pick = others.find((e) => UUID.test(e)) ?? others[0];
  if (!pick) throw new Error("the ballots log names no election besides the demo booth's");
  return pick;
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// A local stand-in for Step's backend (Hasura, Harvest and Windmill), for
// previews only. It answers the GraphQL operations the voting and admin
// portals send from fixtures held in memory, signs the fixture users in
// through a fake Keycloak, and does what Step's backend does for Vitrine: it
// computes the election ballots entry for every cast, submits it to the live
// stack's ballots log, answers the portal with the log's receipt or promise,
// serves the booth's context from the live events log and proxies the
// sidecar's status for the admin's Transparency tab. Nothing here is Step.

import fs from "node:fs";
import http from "node:http";
import path from "node:path";
import crypto from "node:crypto";
import { Executor, GraphQLError, Schema, Tables } from "./graphql.mjs";
import { FakeKeycloak, claimsOf } from "./keycloak.mjs";
import { ADMIN_ID, AREA_ID, TENANT_ID, VOTER_ID, VOTER_USERNAME, ballotStyleText, eventPresentation, styleDigests, tables as fixtureTables, PORTAL_RELEASE } from "./fixtures.mjs";
import { voterContext } from "./events.mjs";
import { electionEntry, judge, stepBallotId, unavailable } from "./vitrine.mjs";

const MAX_BODY = 8 << 20;
const BOOTH_BUDGET_MS = 600;
const CAST_BUDGET_MS = 800;

const readBody = (req) =>
  new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on("data", (c) => {
      size += c.length;
      if (size > MAX_BODY) req.destroy(new Error("body too large"));
      else chunks.push(c);
    });
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    req.on("error", reject);
  });

/** The permissions the admin portal asks of the token: every string in its
 * `IPermissions` enum, read from the checkout so new ones are picked up. */
export function adminPermissions(stepPackages) {
  try {
    const text = fs.readFileSync(path.join(stepPackages, "admin-portal/src/types/keycloak.ts"), "utf8");
    return [...new Set([...text.matchAll(/=\s*"([a-z][a-z0-9-]+)"/g)].map((m) => m[1]))];
  } catch {
    return [];
  }
}

export function createStandIn({ live, sidecar, origin, stepPackages, electionId, version, voterOrigin, adminOrigin, tamper, log }) {
  const schemaFile = path.join(stepPackages, "admin-portal/graphql.schema.json");
  const schema = new Schema(JSON.parse(fs.readFileSync(schemaFile, "utf8")));
  const policyNote = live.policy;
  const text = ballotStyleText({ live, electionId, policyNote });
  const style = styleDigests(text);
  const release = PORTAL_RELEASE(version);
  const db = new Tables();
  for (const [name, rows] of Object.entries(fixtureTables({ live, electionId, policyNote, text }))) db.table(name).push(...rows);
  const casts = new Map();
  const permissions = adminPermissions(stepPackages);
  const state = { casts: 0, problems: [], lastContext: undefined };

  const keycloak = new FakeKeycloak({
    origin,
    log,
    users: (realm, clientId) => {
      const admin = clientId === "admin-portal" || !realm.includes("-event-");
      const claims = {
        "tenant-id": TENANT_ID,
        "area-id": AREA_ID,
        "https://hasura.io/jwt/claims": {
          "x-hasura-tenant-id": TENANT_ID,
          "x-hasura-user-id": admin ? ADMIN_ID : VOTER_ID,
          "x-hasura-default-role": admin ? "admin-user" : "user",
          "x-hasura-allowed-roles": admin ? ["admin-user", ...permissions] : ["user"],
          "x-hasura-area-id": AREA_ID,
          "x-hasura-permission-labels": [],
        },
      };
      return admin
        ? { sub: ADMIN_ID, username: "admin@example.org", email: "admin@example.org", firstName: "Admin", lastName: "Preview", attributes: { "tenant-id": [TENANT_ID] }, extra: claims }
        : { sub: VOTER_ID, username: VOTER_USERNAME, email: VOTER_USERNAME, firstName: "Alex", lastName: "Voter", attributes: { "tenant-id": [TENANT_ID], "area-id": [AREA_ID] }, extra: claims };
    },
  });

  // -- Casting ------------------------------------------------------------------

  /** `insert_cast_vote`: what Step's Windmill and Harvest do, in order. */
  async function insertCastVote({ election_id: castElection, ballot_id: ballotId, content }, ctx) {
    const voter = ctx.claims?.sub ?? VOTER_ID;
    if (castElection !== electionId) throw new GraphQLError(`election ${castElection} is not open for voting here`, "validation-failed");
    let computed;
    try {
      computed = stepBallotId(content);
    } catch (e) {
      log(`cast: couldn't recompute the ballot ID (${e.message}); keeping the portal's`);
    }
    if (computed && computed !== ballotId) throw new GraphQLError("the ballot_id doesn't match the ballot", "validation-failed");
    const held = casts.get(ballotId);
    if (held && held.voter_id_string === voter) return held;
    const now = Math.floor(Date.now() / 1000);
    const entry = electionEntry({ eventId: live.eventId, electionId, ballotId, acceptedAt: now });
    let vitrine;
    try {
      const answer = await sidecar.add(live.ballots, entry, BOOTH_BUDGET_MS, CAST_BUDGET_MS);
      vitrine = judge(answer.status, answer.body, entry);
    } catch (e) {
      log(`cast: the ballots log didn't answer (${e.message})`);
      vitrine = unavailable(entry, "log-unavailable");
    }
    const ts = new Date(now * 1000).toISOString();
    const row = {
      id: crypto.randomUUID(),
      ballot_id: ballotId,
      election_id: electionId,
      election_event_id: live.eventId,
      tenant_id: TENANT_ID,
      area_id: AREA_ID,
      created_at: ts,
      last_updated_at: ts,
      labels: null,
      annotations: null,
      content,
      cast_ballot_signature: null,
      voter_id_string: voter,
      status: "valid",
      vitrine,
    };
    casts.set(ballotId, row);
    db.table("sequent_backend_cast_vote").push(row);
    state.casts += 1;
    log(`cast: ballot ${ballotId.slice(0, 12)}... accepted at ${now}; the log answered ${vitrine.kind}${vitrine.reason ? ` (${vitrine.reason})` : ""}`);
    tamper?.remember?.({ ballotId, entry, acceptedAt: now });
    return row;
  }

  // -- The published ballot data ------------------------------------------------------
  // The voting portal downloads the event, the elections and each ballot style
  // as JSON files (Step serves them from object storage with presigned URLs).

  const row = (table, id) => db.table(table).find((r) => r.id === id);
  const files = () => {
    const event = row("sequent_backend_election_event", live.eventId);
    const election = row("sequent_backend_election", electionId);
    const styleRow = db.table("sequent_backend_ballot_style")[0];
    const wire = JSON.parse(styleRow.ballot_eml);
    return {
      event: { id: event.id, tenant_id: event.tenant_id, presentation: event.presentation, description: event.description, name: event.name, status: event.status },
      election,
      summary: { id: styleRow.id, area_presentation: wire.area_presentation ?? {}, election_dates: wire.election_dates ?? {} },
      style: styleRow,
    };
  };

  function publication() {
    const { event, election, style } = files();
    const url = (name) => `${origin}/files/${name}.json`;
    return {
      event_id: event.id,
      status: event.status,
      files: [
        {
          id: style.id,
          election_id: election.id,
          version: "1",
          urls: { event_url: url("event"), election_url: url("election"), summary_url: url("summary"), style_url: url("style") },
          status: election.status,
          num_allowed_revotes: election.num_allowed_revotes,
          voting_channels: election.voting_channels,
        },
      ],
    };
  }

  // -- The Vitrine contract --------------------------------------------------------

  async function boothContext(args) {
    const { context, problem } = await voterContext({
      live,
      electionId: args.election_id,
      styleHashes: style.digests.map((d) => d.toString("hex")),
      release,
    });
    state.lastContext = { at: Date.now(), problem: problem ?? null, config: Boolean(context.config), opened: Boolean(context.opened) };
    if (problem) log(`booth context: ${problem}`);
    return context;
  }

  const enrolments = () =>
    Object.values(live.logs)
      .filter((l) => l.enrolment && (l.name === "ballots" || l.name === "events"))
      .map((l) => ({ log: l.name, note: l.enrolment }));

  async function transparency() {
    const names = ["ballots", "events"];
    const status = {};
    for (const name of names) {
      try {
        status[name] = await sidecar.status(live.logs[name]);
      } catch (e) {
        log(`transparency: no status for ${name} (${e.message})`);
      }
    }
    let listing = [];
    try {
      listing = (await sidecar.adminLogs()).filter((l) => names.some((n) => l.name === live.logs[n].sidecar || l.name === n));
    } catch (e) {
      log(`transparency: no log listing (${e.message})`);
    }
    return {
      mode: "public",
      refusal: null,
      locked: true,
      logs: listing,
      status,
      draft_policy: null,
      policy: policyNote,
      policy_id: live.policyId.replace(/^sha256:/, ""),
      enrolments: enrolments(),
      explorer_url: live.explorer ?? null,
    };
  }

  // -- GraphQL -------------------------------------------------------------------

  const resolvers = {
    async root(kind, field, args, ctx) {
      switch (field) {
        case "insert_cast_vote":
          return insertCastVote(args, ctx);
        case "get_voter_cast_votes": {
          // The ballot tracker asks for a ballot by its ID, or by the prefix of a short ID.
          const voter = ctx.claims?.sub ?? VOTER_ID;
          const prefix = args.ballot_id_prefix?.toLowerCase();
          return [...casts.values()].filter(
            (c) =>
              c.voter_id_string === voter &&
              (!args.election_id || c.election_id === args.election_id) &&
              (!args.ballot_id || c.ballot_id === args.ballot_id) &&
              (!prefix || c.ballot_id.startsWith(prefix)),
          );
        }
        case "get_ballot_files_urls":
          return publication();
        case "vitrine_voter_context":
          return boothContext(args);
        case "vitrine_voter_forward":
          return { status: 404, body: null };
        case "get_transparency":
        case "enable_transparency":
        case "upload_transparency_policy":
          return transparency();
        case "list_cast_vote_messages":
          return { list: [], total: 0 };
        case "getElectionEventStats":
          return { total_areas: 1, total_distinct_voters: new Set([...casts.values()].map((c) => c.voter_id_string)).size, total_elections: 1, total_eligible_voters: 1, voters_by_channel: [], votes_per_day: [] };
        case "list_keys_ceremony":
        case "get_top_votes_by_ip":
          return { items: [], total: { aggregate: { count: 0 } } };
        default:
          return db.root(kind, field, args);
      }
    },
  };
  const executor = new Executor(schema, resolvers, log);

  // -- HTTP ----------------------------------------------------------------------

  const allowed = new Set([voterOrigin, adminOrigin]);
  const cors = (req, res) => {
    const from = req.headers.origin;
    if (from && allowed.has(from)) {
      res.setHeader("Access-Control-Allow-Origin", from);
      res.setHeader("Vary", "Origin");
      res.setHeader("Access-Control-Allow-Headers", "authorization,content-type,x-hasura-admin-secret,x-hasura-role,x-hasura-user-id,x-hasura-tenant-id");
      res.setHeader("Access-Control-Allow-Methods", "GET,POST,OPTIONS");
      res.setHeader("Access-Control-Allow-Credentials", "true");
      res.setHeader("Access-Control-Max-Age", "600");
    }
    res.setHeader("Cross-Origin-Resource-Policy", "cross-origin");
  };

  const send = (res, status, body, type = "application/json") => {
    res.writeHead(status, { "Content-Type": type, "Cache-Control": "no-store" });
    res.end(typeof body === "string" ? body : JSON.stringify(body));
  };

  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url, origin);
    try {
      cors(req, res);
      if (req.method === "OPTIONS") return res.writeHead(204).end();
      if (await keycloak.handle(req, res, url, readBody)) return;
      if (url.pathname === "/v1/graphql" && req.method === "POST") {
        const body = JSON.parse(await readBody(req));
        const claims = claimsOf(req.headers.authorization);
        const answer = await executor.execute(body, { claims });
        if (answer.errors) log(`graphql ${body.operationName ?? "?"}: ${answer.errors.map((e) => e.message).join("; ")}`);
        return send(res, 200, answer);
      }
      const bucket = /^\/public\/tenant-([^/]+)\/event-([^/]+)\/election_event_config\.json$/.exec(url.pathname);
      if (bucket) {
        return send(res, 200, { election_event_id: bucket[2], tenant_id: bucket[1], election_event_presentation: eventPresentation({ live, policyNote }) });
      }
      const file = /^\/files\/(event|election|summary|style)\.json$/.exec(url.pathname);
      if (file) return send(res, 200, files()[file[1]]);
      if (url.pathname === "/healthz" || url.pathname === "/v1/version") return send(res, 200, { status: "ok", standin: true });
      if (url.pathname === "/__preview/status") {
        return send(res, 200, {
          standin: "A local stand-in for Step's backend: previews only",
          policy: live.policyId,
          event: live.eventId,
          election: electionId,
          release,
          casts: state.casts,
          boothContext: state.lastContext ?? null,
          tamper: tamper?.status?.() ?? null,
        });
      }
      if (url.pathname.startsWith("/__preview/tamper") && tamper) return tamper.handle(req, res, url, readBody, send);
      send(res, 404, { error: "not found", path: url.pathname });
    } catch (e) {
      log(`error ${req.method} ${url.pathname}: ${e.stack ?? e}`);
      send(res, 500, { error: String(e.message ?? e) });
    }
  });

  return { server, state, db, style, text, release, electionId };
}

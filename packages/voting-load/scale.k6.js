// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import http from "k6/http";
import exec from "k6/execution";
import { SharedArray } from "k6/data";
import { Counter, Rate, Trend } from "k6/metrics";
import { replayJourney } from "./replay.k6.js";

const config = JSON.parse(open(__ENV.LOAD_CONFIG));
const shard = Number(__ENV.LOAD_SHARD);
const first = shard * config.shard_size;
const count = Math.min(config.shard_size, config.count - first);
const ballots = new SharedArray("shard ciphertexts", () =>
  config.mode === "status"
    ? []
    : open(__ENV.LOAD_BALLOTS).trim().split("\n").map(JSON.parse),
);
if (count <= 0 || (config.mode !== "status" && ballots.length !== count))
  throw new Error("Invalid shard");
const accepted = new Counter("accepted_casts");
const failed = new Rate("journey_failures");
const castLatency = new Trend("cast_latency", true);
const phases = [
  "auth",
  "login",
  "token",
  "GetVoterStatus",
  "event_url",
  "election_url",
  "summary_url",
  "style_url",
  "InsertCastVote",
];
export const options = {
  scenarios: {
    voters: {
      executor: "shared-iterations",
      vus: Math.min(config.vus, count),
      iterations: count,
      maxDuration: config.max_duration,
    },
  },
  thresholds: {
    journey_failures: ["rate==0"],
    iterations: [`count==${count}`],
    ...(config.mode === "status"
      ? {}
      : { accepted_casts: [`count==${count}`] }),
    ...Object.fromEntries(
      phases.flatMap((name) => [
        [`http_reqs{name:${name}}`, ["count>=0"]],
        [`http_req_duration{name:${name}}`, ["p(99)>=0"]],
      ]),
    ),
  },
  summaryTrendStats: ["med", "p(50)", "p(99)", "max"],
  systemTags: ["status", "method", "name", "scenario", "error_code"],
  discardResponseBodies: false,
};

/** One finite iteration owns one voter; failed casts are never retried automatically. */
export default function () {
  const local = exec.scenario.iterationInTest;
  const index = config.start + first + local;
  const ballot = {
    credentials: {
      ...config.login_fields,
      username: config.username_prefix + index,
      password: __ENV.LOAD_PASSWORD,
    },
    election_event_id: config.election_event_id,
    style_id: config.style_id,
    publication_version: config.publication_version,
    payload: {
      query: config.cast_query,
      operationName: "InsertCastVote",
      variables: ballots[local] || { electionId: config.election_id },
    },
  };
  const start = Date.now();
  let passed = false,
    receipt = null,
    castMs = null,
    statusMs = null;
  try {
    const result = replayJourney(
      config.profile,
      ballot,
      index,
      { ...config, pacing: 0, trace_http: config.trace_http === true },
      (authorization) => {
        const before = Date.now();
        const response = http.post(
          config.graphql_url,
          JSON.stringify(ballot.payload),
          {
            headers: {
              Authorization: authorization,
              "Content-Type": "application/json",
            },
            tags: { name: "InsertCastVote" },
            redirects: 0,
            timeout: config.cast_timeout || "60s",
          },
        );
        castMs = Date.now() - before;
        castLatency.add(castMs);
        const body = response.json();
        receipt = body.data?.insert_cast_vote;
        const ok =
          response.status === 200 &&
          !body.errors &&
          receipt?.ballot_id === ballot.payload.variables.ballotId &&
          receipt?.election_id === config.election_id &&
          receipt?.election_event_id === config.election_event_id;
        if (ok) accepted.add(1);
        else receipt = null;
        return ok;
      },
    );
    statusMs = result.timings.GetVoterStatus;
    passed = true;
  } catch (_) {
    // Credentials, signed URLs and response bodies must not enter shared reports.
  }
  failed.add(!passed);
  console.log(
    "RESULT " +
      JSON.stringify({
        index,
        passed,
        start,
        end: Date.now(),
        cast_ms: castMs,
        status_ms: statusMs,
        receipt: receipt?.id || null,
      }),
  );
}

/** Keep native k6 metrics beside the portable coordinator report. */
export function handleSummary(data) {
  return { [__ENV.LOAD_SUMMARY]: JSON.stringify(data) };
}

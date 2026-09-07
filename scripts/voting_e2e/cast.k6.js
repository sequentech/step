// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import http from "k6/http";
import exec from "k6/execution";
import { SharedArray } from "k6/data";
import { Counter, Rate, Trend } from "k6/metrics";
import { sleep } from "k6";

const config = JSON.parse(open(__ENV.LOAD_CONFIG));
const ballots = new SharedArray("prepared ballots", () =>
  JSON.parse(open(__ENV.LOAD_BALLOTS)),
);
const planned = config.rate * config.duration_seconds;
if (ballots.length !== planned)
  throw new Error("Prepared shard size does not match the planned arrivals");
const boundary = new Counter("arrival_boundary_ticks");
const accepted = new Counter("accepted_casts");
const failures = new Rate("cast_failures");
const latency = new Trend("cast_latency", true);
export const options = {
  setupTimeout: `${Math.max(60, Math.ceil((config.start_at_ms - Date.now()) / 1000) + 30)}s`,
  scenarios: {
    cast: {
      executor: "constant-arrival-rate",
      rate: config.rate,
      timeUnit: "1s",
      duration: `${config.duration_seconds}s`,
      preAllocatedVUs: config.vus,
      maxVUs: config.vus,
      gracefulStop: "30s",
    },
  },
  thresholds: {
    cast_latency: [
      `p(50)<=${config.goals.p50_ms}`,
      `p(99)<=${config.goals.p99_ms}`,
    ],
    cast_failures: ["rate==0"],
    dropped_iterations: ["count==0"],
    accepted_casts: [`count==${config.rate * config.duration_seconds}`],
  },
  maxRedirects: 0,
  summaryTrendStats: ["min", "med", "p(50)", "p(95)", "p(99)", "max"],
  systemTags: ["status", "method", "name", "scenario", "error_code"],
};
export function setup() {
  const remaining = (config.start_at_ms - Date.now()) / 1000;
  if (remaining < 0)
    throw new Error("Missed coordinated start; regenerate the run");
  sleep(remaining);
}
export default function () {
  const index = exec.scenario.iterationInTest;
  // k6 can schedule a final tick at the duration boundary. The finite fixture
  // is the cast cap; never wrap or reuse a ballot for that scheduler tick.
  if (index >= planned) {
    boundary.add(1);
    return;
  }
  const ballot = ballots[index];
  if (!ballot)
    exec.test.abort("Prepared census exhausted; no voter may be reused");
  const started = Date.now();
  const response = http.post(ballot.url, JSON.stringify(ballot.payload), {
    headers: {
      "Content-Type": "application/json",
      Authorization: ballot.authorization,
    },
    timeout: "30s",
    redirects: 0,
    tags: { name: "InsertCastVote" },
  });
  let cast = null;
  try {
    const body = response.json();
    if (response.status === 200 && !body.errors?.length)
      cast = body.data?.insert_cast_vote;
  } catch (_) {
    /* Malformed responses fail without printing sensitive bodies. */
  }
  const valid = !!(
    cast?.id &&
    cast.ballot_id === ballot.payload.variables.ballotId &&
    cast.tenant_id === ballot.tenant_id &&
    cast.election_event_id === ballot.election_event_id &&
    cast.election_id === ballot.payload.variables.electionId
  );
  const duration = Date.now() - started;
  latency.add(duration);
  failures.add(!valid);
  accepted.add(valid ? 1 : 0);
  console.log(
    JSON.stringify({
      kind: "cast",
      index,
      started_at_ms: started,
      ended_at_ms: Date.now(),
      duration_ms: duration,
      status: response.status,
      accepted: valid,
      method: "POST",
      endpoint: ballot.url,
      operation: "InsertCastVote",
      response_bytes: response.body
        ? encodeURIComponent(response.body).replace(/%[0-9A-F]{2}/g, "_").length
        : 0,
      receipt: valid
        ? {
            id: cast.id,
            ballot_id: cast.ballot_id,
            tenant_id: cast.tenant_id,
            election_event_id: cast.election_event_id,
            election_id: cast.election_id,
          }
        : null,
    }),
  );
}
export function handleSummary(data) {
  return { [__ENV.LOAD_SUMMARY]: JSON.stringify(data, null, 2) };
}

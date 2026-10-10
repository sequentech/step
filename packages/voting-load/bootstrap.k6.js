// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import { replayJourney } from "./replay.k6.js";

const input = JSON.parse(open(__ENV.LOAD_CONFIG));
export const options = { vus: 1, iterations: 1 };

/** Fetch the publication once without executing or intercepting a cast. */
export default function () {
  const ballot = {
    election_event_id: input.election_event_id,
    credentials: {
      ...input.login_fields,
      username: input.username_prefix + input.start,
      password: __ENV.LOAD_PASSWORD,
    },
    payload: { variables: { electionId: input.election_id } },
  };
  const result = replayJourney(
    input.profile,
    ballot,
    0,
    { ...input, bootstrap: true, pacing: 0, trace_http: false },
    () => {
      throw new Error("Bootstrap must never cast");
    },
  );
  // The coordinator captures this stream into a private file, never the terminal.
  console.log("PUBLICATION " + JSON.stringify(result));
}

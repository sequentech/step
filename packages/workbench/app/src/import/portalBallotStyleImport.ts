// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Importer for a single portal `IBallotStyle` JSON document — the
// shape produced by `select * from public.ballot_styles where id =
// $1` in the Hasura console, or by the admin portal's BS detail
// export button. The importer:
//
//   1. Generates a fresh workbench keypair and stamps the new public
//      key into `ballot_eml.public_key`.
//   2. Synthesizes minimal `elections` and `electionEvent` rows so
//      the booth screen has the metadata it expects.
//   3. Spawns a single voter named "voter".
//   4. Builds an eligibility overlay with the lone voter assigned to
//      this single ballot style.
//
// The result is a `PersistedSnapshot` ready to feed into
// `loadSnapshotViaReload(snap, null)`.

import type {PersistedSnapshot} from "../persistence"
import {
    assembleSnapshot,
    DEFAULT_OPEN_STATUS,
    makeVoter,
    rekeyBallotStyle,
    type PortalBallotStyleRow,
} from "./importHelpers"

/** Parse a raw JSON string into a portal `IBallotStyle` row. Throws
 *  with a human-readable message when required keys are missing. */
function parsePortalBallotStyle(input: string): PortalBallotStyleRow {
    let raw: unknown
    try {
        raw = JSON.parse(input)
    } catch (err) {
        throw new Error(
            `Could not parse JSON: ${err instanceof Error ? err.message : err}`
        )
    }
    if (!raw || typeof raw !== "object") {
        throw new Error("Top-level JSON value must be an object.")
    }
    const obj = raw as Record<string, unknown>
    // Tolerate two common shapes: the row directly, or wrapped under
    // `{ballot_style: <row>}` (the way some admin exports nest it).
    const row =
        typeof obj.ballot_eml === "object"
            ? (obj as unknown as PortalBallotStyleRow)
            : typeof (obj.ballot_style as {ballot_eml?: unknown})
                  ?.ballot_eml === "object"
                ? (obj.ballot_style as unknown as PortalBallotStyleRow)
                : null
    if (!row) {
        throw new Error(
            "Expected a portal IBallotStyle row with `ballot_eml`, " +
                "either at the top level or under `ballot_style`."
        )
    }
    const required: Array<keyof PortalBallotStyleRow> = [
        "id",
        "election_id",
        "election_event_id",
        "tenant_id",
    ]
    for (const key of required) {
        if (typeof row[key] !== "string") {
            throw new Error(`Missing or non-string field: ${key}`)
        }
    }
    if (!row.ballot_eml || typeof row.ballot_eml !== "object") {
        throw new Error("ballot_eml must be an object")
    }
    if (!Array.isArray(row.ballot_eml.contests)) {
        throw new Error("ballot_eml.contests must be an array")
    }
    return row
}

/** Build a `PersistedSnapshot` from a portal IBallotStyle JSON
 *  string. Async because keypair generation runs through the velvet
 *  WASM binding. */
export async function importPortalBallotStyle(
    input: string
): Promise<PersistedSnapshot> {
    const row = parsePortalBallotStyle(input)
    // Deep clone so we don't mutate the caller's parsed object.
    const cloned = JSON.parse(JSON.stringify(row)) as PortalBallotStyleRow
    const kp = await rekeyBallotStyle(cloned)
    if (!cloned.created_at) cloned.created_at = "1970-01-01T00:00:00Z"
    if (!cloned.last_updated_at)
        cloned.last_updated_at = "1970-01-01T00:00:00Z"
    const voter = makeVoter("voter")
    return assembleSnapshot({
        electionEvent: {
            id: cloned.election_event_id,
            tenant_id: cloned.tenant_id,
            name: "Imported election event",
            description: "Synthesized by portal ballot-style import.",
            elections: [cloned.election_id],
            status: {...DEFAULT_OPEN_STATUS},
        },
        election: {
            id: cloned.election_id,
            election_event_id: cloned.election_event_id,
            tenant_id: cloned.tenant_id,
            name:
                ((cloned.ballot_eml as {name?: string}).name as string) ??
                "Imported election",
            description: "Synthesized by portal ballot-style import.",
            contests: cloned.ballot_eml.contests ?? [],
            num_allowed_revotes: 0,
            status: {...DEFAULT_OPEN_STATUS},
        },
        ballotStyles: [cloned],
        keypairs: {[cloned.id]: kp},
        voters: [voter],
        assignments: {[voter.id]: [cloned.id]},
    })
}

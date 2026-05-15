// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Importer for a velvet `ElectionConfig` JSON document — the
// authoritative election bundle used by the velvet pipeline (see
// `velvet/src/pipe_inputs.rs::ElectionConfig`). The importer:
//
//   1. Generates a fresh workbench keypair per ballot style and
//      stamps each new public key into its `ballot_eml.public_key`.
//   2. Wraps every velvet `BallotStyle` (which carries `contests`
//      directly) into a portal `IBallotStyle` slice row by setting
//      `ballot_eml = <velvet BS payload>`.
//   3. Synthesizes one voter per `TreeNodeArea`, named
//      `voter (<area-short-id>)` (TreeNodeArea has no `name` field
//      so we use the first 4 chars of the area UUID).
//   4. Builds the eligibility overlay: each voter assigned to every
//      ballot style whose `area_id` matches their area.
//
// The result is a `PersistedSnapshot` ready to feed into
// `loadSnapshotViaReload(snap, null)`.

import type {PersistedSnapshot} from "../persistence"
import type {Voter} from "../workbenchStore"
import {
    assembleSnapshot,
    DEFAULT_OPEN_STATUS,
    makeVoter,
    rekeyBallotStyle,
    type PortalBallotStyleRow,
} from "./importHelpers"

/** A velvet `BallotStyle` is shaped like the contents of a portal
 *  `ballot_eml` field but with an explicit `area_id`. Loose typing
 *  so we don't have to keep this file in sync with the Rust struct. */
interface VelvetBallotStyle {
    id: string
    election_id: string
    election_event_id: string
    tenant_id: string
    area_id: string
    contests: Array<{id: string} & Record<string, unknown>>
    [k: string]: unknown
}

interface VelvetArea {
    id: string
    tenant_id: string
    election_event_id: string
    parent_id?: string | null
    [k: string]: unknown
}

interface VelvetElectionConfig {
    id: string
    name: string
    election_event_id: string
    tenant_id: string
    description?: string
    ballot_styles: VelvetBallotStyle[]
    areas: VelvetArea[]
    [k: string]: unknown
}

function parseVelvetConfig(input: string): VelvetElectionConfig {
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
    const required = [
        "id",
        "name",
        "election_event_id",
        "tenant_id",
    ] as const
    for (const key of required) {
        if (typeof obj[key] !== "string") {
            throw new Error(`Missing or non-string field: ${key}`)
        }
    }
    if (!Array.isArray(obj.ballot_styles) || obj.ballot_styles.length === 0) {
        throw new Error("ballot_styles must be a non-empty array")
    }
    if (!Array.isArray(obj.areas) || obj.areas.length === 0) {
        throw new Error("areas must be a non-empty array")
    }
    return obj as unknown as VelvetElectionConfig
}

/** Build a `PersistedSnapshot` from a velvet ElectionConfig JSON
 *  string. */
export async function importVelvetElection(
    input: string
): Promise<PersistedSnapshot> {
    const config = parseVelvetConfig(input)
    // 1. Wrap and re-key every ballot style.
    const wrappedRows: PortalBallotStyleRow[] = []
    const keypairs: Record<string, {pkB64: string; skB64: string}> = {}
    for (const bs of config.ballot_styles) {
        if (typeof bs.id !== "string" || typeof bs.area_id !== "string") {
            throw new Error(
                "Every ballot style must have string `id` and `area_id`."
            )
        }
        const row: PortalBallotStyleRow = {
            id: bs.id,
            election_id: config.id,
            election_event_id: config.election_event_id,
            tenant_id: config.tenant_id,
            area_id: bs.area_id,
            // Clone the BS minus the fields we now hold at the row
            // level — what's left becomes the `ballot_eml` payload.
            ballot_eml: JSON.parse(JSON.stringify(bs)) as Record<
                string,
                unknown
            >,
            created_at: "1970-01-01T00:00:00Z",
            last_updated_at: "1970-01-01T00:00:00Z",
        }
        const kp = await rekeyBallotStyle(row)
        keypairs[bs.id] = kp
        wrappedRows.push(row)
    }
    // 2. One voter per area; assignments by area_id.
    const voters: Voter[] = []
    const assignments: Record<string, string[]> = {}
    for (const area of config.areas) {
        if (typeof area.id !== "string") {
            throw new Error("Every area must have a string `id`.")
        }
        const shortId = area.id.slice(0, 4)
        const voter = makeVoter(`voter (${shortId})`)
        voters.push(voter)
        assignments[voter.id] = wrappedRows
            .filter((bs) => bs.area_id === area.id)
            .map((bs) => bs.id)
    }
    // 3. Election + electionEvent rows. Contests for the election
    // slice are unioned across all ballot styles, deduped by id, so
    // the booth has every contest definition the user might land on
    // after a voter swap.
    const seen = new Set<string>()
    const contests: Array<{id: string} & Record<string, unknown>> = []
    for (const row of wrappedRows) {
        for (const c of row.ballot_eml.contests ?? []) {
            if (!seen.has(c.id)) {
                seen.add(c.id)
                contests.push(c)
            }
        }
    }
    return assembleSnapshot({
        electionEvent: {
            id: config.election_event_id,
            tenant_id: config.tenant_id,
            name: config.name,
            description: config.description ?? "",
            elections: [config.id],
            status: {...DEFAULT_OPEN_STATUS},
        },
        election: {
            id: config.id,
            election_event_id: config.election_event_id,
            tenant_id: config.tenant_id,
            name: config.name,
            description: config.description ?? "",
            contests,
            num_allowed_revotes: 0,
            status: {...DEFAULT_OPEN_STATUS},
        },
        ballotStyles: wrappedRows,
        keypairs,
        voters,
        assignments,
    })
}

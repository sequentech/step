// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {ILogMessage} from "./logMessage"

const object = (value: unknown): value is Record<string, unknown> =>
    value !== null && typeof value === "object" && !Array.isArray(value)

/** Display metadata only; the proof panel separately verifies the canonical signed bytes. */
export const ballotLogDetails = (message: ILogMessage | null) => {
    const body = message?.statement?.body
    if (!object(body) || (!object(body.BallotReceived) && !object(body.PrivacyCastVote)))
        return null
    if (!message?.ballot_id || !message.election_id || !message.username) return null
    return {
        ballotId: message.ballot_id,
        electionId: message.election_id,
        pseudonym: message.username,
    }
}

export const pseudonymResolution = (message: ILogMessage | null) => {
    const body = message?.statement?.body
    if (!object(body) || !object(body.PseudonymResolved)) return null
    const {pseudonym, reason} = body.PseudonymResolved
    return typeof pseudonym === "string" && typeof reason === "string" ? {pseudonym, reason} : null
}

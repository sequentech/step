// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {IElectionStatus, EVotingStatus} from "@sequentech/ui-core"

export const getVotingStatus = (data?: IElectionStatus): EVotingStatus => {
    return data?.voting_status || EVotingStatus.NOT_STARTED
}

const VOTING_CHANNEL_STATUSES = [
    "voting_status",
    "kiosk_voting_status",
    "early_voting_status",
    "telephone_voting_status",
] as const

/**
 * Whether a voting status (an event's or an election's) shows voting has
 * opened on some channel. Mirrors the database's `trusted_voting_started`
 * over `trusted_voting_state`: only the four channels count; a missing or
 * null channel status is NOT_STARTED; a period date set to null is no date
 * (`ElectionStatus::default()` writes them all as null).
 */
export const votingStarted = (status: unknown): boolean => {
    if (!status || typeof status !== "object" || Array.isArray(status)) return false
    const record = status as Record<string, unknown>
    return VOTING_CHANNEL_STATUSES.some((channel) => {
        const value = record[channel]
        if (value !== undefined && value !== null && value !== EVotingStatus.NOT_STARTED) {
            return true
        }
        const dates = record[channel.replace("_status", "_period_dates")]
        if (dates === undefined || dates === null) return false
        if (typeof dates === "object" && !Array.isArray(dates)) {
            return Object.values(dates).some((instant) => instant !== null)
        }
        // A non-object value is kept as is by the database, so it counts.
        return true
    })
}

/**
 * Whether voting has ever opened in the event or any of its elections, on
 * any channel. Settings that are locked once voting has opened (the ballot
 * box seal policy) are disabled then, as the database refuses the change.
 */
export const hasVotingEverOpened = (
    elections: Array<{status?: unknown}> | undefined,
    event: {status?: unknown} | undefined
): boolean =>
    votingStarted(event?.status) ||
    (elections ?? []).some((election) => votingStarted(election.status))

/** Whether the ballot box seal policy may change, and why not. */
export enum ESealPolicyLock {
    /** Voting has never opened: it may change. */
    OPEN = "open",
    /** The elections are still loading. */
    LOADING = "loading",
    /** The elections could not be read: whether voting opened is unknown. */
    UNKNOWN = "unknown",
    /** Voting has opened (in `elections`, or on the event itself when empty). */
    LOCKED = "locked",
}

export interface ISealPolicyLock<Election> {
    state: ESealPolicyLock
    /** The elections where voting has opened. */
    elections: Election[]
}

/**
 * Whether the ballot box seal policy is locked, as the database's
 * `guard_ballot_box_seal_policy_update` decides, and which elections lock it.
 */
export const ballotBoxSealPolicyLock = <Election extends {status?: unknown}>(
    elections: Election[] | undefined,
    event: {status?: unknown} | undefined,
    loading = false
): ISealPolicyLock<Election> => {
    if (loading && !elections) return {state: ESealPolicyLock.LOADING, elections: []}
    if (!elections) return {state: ESealPolicyLock.UNKNOWN, elections: []}
    const opened = elections.filter((election) => votingStarted(election.status))
    return opened.length || votingStarted(event?.status)
        ? {state: ESealPolicyLock.LOCKED, elections: opened}
        : {state: ESealPolicyLock.OPEN, elections: []}
}

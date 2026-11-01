// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Ballots that share a short ID (design boards ShortIdMatches and
// ShortIdStates). A short ID finds ballots; it never proves one is yours. The
// browser asks a log server or a monitor of the policy for every ballot whose
// Tracking ID starts with the first 3 characters (about 30 in a million) and
// keeps the ones that share the typed groups itself, so no server learns which
// ballot was looked for. A server that has no such route (404) is left, and
// the existing lookup, which sends the typed ID, takes over.

import {
    BucketLookupError,
    lookupTrackingId,
    type BallotMatch,
    type BucketMatches,
    type Policy,
} from "@vitrine/verify"
import {
    ballotIdChars,
    ballotIdInput,
    BALLOT_ID_GROUP,
    EBallotIdInput,
    groupBallotId,
} from "./ballotId"

/** Up to this many ballots are listed; above it, only the third group is asked. */
export const MAX_LISTED = 5
/** The groups of a short ID. */
const SHORT_GROUPS = 2
const SHORT_LENGTH = SHORT_GROUPS * BALLOT_ID_GROUP

/** What the lookup came to. */
export enum EShortIdKind {
    /** One to five ballots, listed. */
    LISTED = "listed",
    /** More than five: nothing is listed, the third group is asked. */
    MANY = "many",
    /** No ballot starts so. */
    NONE = "none",
    /** No server of the policy has the bucket route: the existing lookup, which
     * sends the typed ID to the election's server, takes over. */
    UNSUPPORTED = "unsupported",
    /** The servers that have it didn't answer. */
    UNAVAILABLE = "unavailable",
}

export enum EShortIdStatus {
    /** Nothing to look up in a bucket: the ID isn't short, or the event has no
     * public policy to name a server. The routes that send the ID answer. */
    OFF = "off",
    LOADING = "loading",
    DONE = "done",
}

export interface IShortIdMatch {
    /** The ballot's 64 hex digits. */
    ballotId: string
    /** The full Tracking ID, 52 characters. */
    trackingId: string
    /** The third and fourth groups, to compare with a receipt. */
    third: string
    fourth: string
    /** Whether its last step is an audit: the ballot was never cast. */
    audited: boolean
    /** When the ballot box last took it, POSIX seconds. */
    at?: number
    event: string
    election: string
    /** The receipt this device keeps is for this ballot. */
    onDevice: boolean
}

export interface IShortIdResult {
    kind: EShortIdKind
    /** What was looked up, as characters. */
    typed: string
    /** The ballots to list, the one on this device first. */
    matches: IShortIdMatch[]
    /** All the ballots that start so, listed or not. */
    total: number
    /** The server stopped at its limit: there may be more. */
    truncated: boolean
    /** The ballot on this device, if its Tracking ID starts so. */
    mine?: string
}

/** A server of the policy that may list a bucket. */
export interface IBucketServer {
    server: string
    log: string
}

/** The log servers and monitors of a policy that may list the buckets of its
 * ballots log: the log's own server first, then the monitors in the policy's
 * order. The log's server is the log's URL without the log's path, which is
 * the origin without the domain. */
export const bucketServers = (policy: Policy): IBucketServer[] => {
    const servers: IBucketServer[] = []
    const ballots = policy.logs.get("ballots")
    if (ballots) {
        const slash = ballots.origin.indexOf("/")
        const name = slash < 0 ? "" : ballots.origin.slice(slash + 1)
        const url = ballots.url?.replace(/\/+$/, "")
        if (name && url?.endsWith(`/${name}`)) {
            servers.push({server: url.slice(0, url.length - name.length - 1), log: name})
        }
        for (const monitor of Array.from(policy.monitors.values())) {
            servers.push({server: monitor.url, log: ballots.origin})
        }
    }
    return servers
}

const shortLookupOf = (value: string): string | undefined => {
    const input = ballotIdInput(value)
    return input === EBallotIdInput.SHORT || input === EBallotIdInput.PARTIAL
        ? ballotIdChars(value)
        : undefined
}

/** The characters to look up in a bucket, for a short ID or the start of one;
 * undefined for a full ID, which the existing lookups take, or what isn't one. */
export const shortIdToLook = shortLookupOf

const matchOf = (match: BallotMatch, mine: string | undefined): IShortIdMatch => {
    const chars = ballotIdChars(match.trackingId)
    const last = match.entries[match.entries.length - 1]
    return {
        ballotId: match.ballotId,
        trackingId: chars,
        third: chars.slice(SHORT_LENGTH, SHORT_LENGTH + BALLOT_ID_GROUP),
        fourth: chars.slice(SHORT_LENGTH + BALLOT_ID_GROUP, SHORT_LENGTH + 2 * BALLOT_ID_GROUP),
        audited: last?.stage === "audited",
        ...(last ? {at: last.acceptedAt} : {}),
        event: match.event,
        election: match.election,
        onDevice: mine === chars,
    }
}

/** The result of a bucket answer, for what was typed: when the third group is
 * typed, only the ballots that share it; the receipt of this device, if its ID
 * starts as typed, comes first and is marked. */
export const shortIdResultOf = (
    typed: string,
    answer: BucketMatches,
    kept: readonly string[] = []
): IShortIdResult => {
    const chars = ballotIdChars(typed)
    const mine = kept.map(ballotIdChars).find((id) => id.startsWith(chars))
    const wanted = Math.floor(chars.length / BALLOT_ID_GROUP)
    // The answer lists the ballots that share at least the short ID's groups,
    // those that share more first: with more typed, only those that do.
    const listed = answer.matches.filter((match) => match.shared >= Math.max(wanted, SHORT_GROUPS))
    // The ones the answer doesn't list share no more than the last listed.
    const total =
        listed.length === answer.matches.length ? listed.length + answer.more : listed.length
    const matches = listed.map((match) => matchOf(match, mine))
    if (mine && !matches.some((match) => match.onDevice)) {
        // The device keeps this ballot: it is listed first even if the server's
        // bucket didn't.
        matches.unshift({
            ballotId: "",
            trackingId: mine,
            third: mine.slice(SHORT_LENGTH, SHORT_LENGTH + BALLOT_ID_GROUP),
            fourth: mine.slice(SHORT_LENGTH + BALLOT_ID_GROUP, SHORT_LENGTH + 2 * BALLOT_ID_GROUP),
            audited: false,
            event: "",
            election: "",
            onDevice: true,
        })
    } else if (mine) {
        matches.sort((a, b) => Number(b.onDevice) - Number(a.onDevice))
    }
    const count = Math.max(total, matches.length)
    const kind =
        count === 0
            ? EShortIdKind.NONE
            : count > MAX_LISTED && !mine
              ? EShortIdKind.MANY
              : EShortIdKind.LISTED
    return {
        kind,
        typed: chars,
        matches: kind === EShortIdKind.MANY ? [] : matches.slice(0, MAX_LISTED),
        total: count,
        truncated: answer.truncated,
        ...(mine ? {mine} : {}),
    }
}

export interface IShortIdLookupOptions {
    fetch?: typeof fetch
    signal?: AbortSignal
}

/** Looks the typed short ID up in the policy's servers, one after another until
 * one lists the bucket whole. */
export const lookupShortId = async (
    policy: Policy,
    typed: string,
    kept: readonly string[] = [],
    opts: IShortIdLookupOptions = {}
): Promise<IShortIdResult> => {
    const chars = shortLookupOf(typed)
    if (!chars) throw new TypeError("not a short Tracking ID")
    const servers = bucketServers(policy)
    let unavailable = false
    let partial: BucketMatches | undefined
    for (const {server, log} of servers) {
        try {
            const answer = await lookupTrackingId(server, log, groupBallotId(chars), {
                ...(opts.fetch ? {fetch: opts.fetch} : {}),
                ...(opts.signal ? {signal: opts.signal} : {}),
            })
            if (!answer.truncated) return shortIdResultOf(chars, answer, kept)
            // It stopped at its limit: another server may list the rest.
            partial ??= answer
        } catch (error) {
            if (opts.signal?.aborted) throw error
            if (error instanceof BucketLookupError && error.status === 404) continue
            unavailable = true
        }
    }
    if (partial) return shortIdResultOf(chars, partial, kept)
    return {
        kind: unavailable ? EShortIdKind.UNAVAILABLE : EShortIdKind.UNSUPPORTED,
        typed: chars,
        matches: [],
        total: 0,
        truncated: false,
    }
}

/** What to show when the voter typed the short ID and a third group (or more):
 * the ID the third group gives, or the start to type, as characters. */
export const withThirdGroup = (typed: string, third: string): string =>
    ballotIdChars(typed).slice(0, SHORT_LENGTH) + ballotIdChars(third).slice(0, BALLOT_ID_GROUP)

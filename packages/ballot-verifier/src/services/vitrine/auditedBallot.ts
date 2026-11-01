// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The audited ballot screen (design boards HubAudit and PhoneHubAudit): the
// ballot verifier that a transparency hub runs opens an audited ballot from
// the link the voting portal's audit screen gives,
// `<verifier>tenant/<t>/event/<e>/election/<el>/audited#id=<Tracking ID>&policy=sha256:<hex>`.
// The Tracking ID and the policy ID ride in the fragment, so the hub's server
// never sees them. The rest comes from the public record and is checked here:
// the election's policy must have the link's policy ID; the ballot's entries
// come from a bucket of the policy's ballots log, which names 3 characters of
// the Tracking ID and nothing more, and the audited one is proved in the log
// like a receipt; and the ballot itself, with the secret the portal
// published, comes from Step's public ballot locator, as for the voting
// portal's ballot tracker.

import type {BallotMatch, BallotStageEntry, Policy, ReceiptReference} from "@vitrine/verify"
import {ballotIdChars, ballotIdInput, EBallotIdInput, groupBallotId} from "./ballotId"
import {FetchLike, fetchJson, sdkFetch} from "./http"
import {examinePolicy, IPolicyView, pinnedPolicies, policyElections} from "./policies"
import type {IVitrineSdk} from "./sdk"

/** The route of the screen, under the app's base. */
export const AUDITED_ROUTE = "/tenant/:tenantId/event/:eventId/election/:electionId/audited"

/** The log whose entries are the ballots'. */
const BALLOTS_LOG = "ballots"

const POLICY_ID = /^sha256:[0-9a-f]{64}$/

/** What the link names: the ballot and the policy to check it with. */
export interface IAuditedLink {
    /** The full Tracking ID, in groups. */
    trackingId: string
    /** `sha256:<hex>`. */
    policyId: string
}

/** Where the ballot is: the route's IDs. */
export interface IAuditedIds {
    tenantId: string
    eventId: string
    electionId: string
}

/** The fragment's Tracking ID and policy ID, or nothing when it lacks either
 * or has a short Tracking ID: only the whole ID names one ballot. */
export const parseAuditedFragment = (hash: string): IAuditedLink | undefined => {
    const params = new URLSearchParams(hash.replace(/^#/, ""))
    const id = params.get("id") ?? ""
    const policyId = (params.get("policy") ?? "").trim().toLowerCase()
    if (ballotIdInput(id) !== EBallotIdInput.FULL || !POLICY_ID.test(policyId)) return undefined
    return {trackingId: groupBallotId(id), policyId}
}

/** The link to a hub's ballot verifier for an audited ballot (the voting
 * portal's side of the same contract). */
export const auditedBallotLink = (
    verifier: string,
    ids: IAuditedIds,
    link: IAuditedLink
): string => {
    const base = verifier.endsWith("/") ? verifier : `${verifier}/`
    const path = [
        "tenant",
        ids.tenantId,
        "event",
        ids.eventId,
        "election",
        ids.electionId,
        "audited",
    ]
        .map(encodeURIComponent)
        .join("/")
    return `${base}${path}#id=${groupBallotId(link.trackingId)}&policy=${link.policyId}`
}

// -- The policy ------------------------------------------------------------------

export enum EPolicyProblem {
    /** Neither this device nor the event's public config has the link's policy. */
    UNAVAILABLE = "unavailable",
    /** The policy is for another event or election than the link's. */
    OTHER_ELECTION = "other-election",
}

export type TPolicyLoad = {ok: true; view: IPolicyView} | {ok: false; problem: EPolicyProblem}

export interface IFetchOptions {
    fetch?: FetchLike
    signal?: AbortSignal
}

/** The URL of an election event's public config in the public bucket. */
export const publicConfigUrl = (publicBucketUrl: string, ids: IAuditedIds): string =>
    `${publicBucketUrl}tenant-${ids.tenantId}/event-${ids.eventId}/election_event_config.json`

const isObject = (value: unknown): value is Record<string, unknown> =>
    typeof value === "object" && value !== null && !Array.isArray(value)

/** The sealed policy text an event's public config carries, in its
 * presentation's Vitrine part, as the voting portal reads it. */
const configPolicy = (config: unknown, ids: IAuditedIds): string | undefined => {
    if (!isObject(config)) return undefined
    if (config.tenant_id !== ids.tenantId || config.election_event_id !== ids.eventId) {
        return undefined
    }
    const presentation = config.election_event_presentation
    const vitrine = isObject(presentation) ? presentation.vitrine : undefined
    const policy = isObject(vitrine) ? vitrine.policy : undefined
    return typeof policy === "string" && policy ? policy : undefined
}

/** Whether the policy's ballots log holds the ballots of the link's election. */
const coversElection = (policy: Policy, ids: IAuditedIds): boolean => {
    const {eventId, electionIds} = policyElections(policy)
    return eventId === ids.eventId && electionIds.includes(ids.electionId)
}

/** The policy the link names: one pinned on this device, else the one the
 * event's public config carries. Either way it is only taken when it is
 * sealed and its ID is the link's, so that neither a config nor a link can
 * pass another policy off; and it must cover the link's election. */
export const loadAuditedPolicy = async (
    sdk: IVitrineSdk,
    publicBucketUrl: string,
    ids: IAuditedIds,
    policyId: string,
    opts: IFetchOptions = {}
): Promise<TPolicyLoad> => {
    let view = pinnedPolicies(sdk).find((pinned) => pinned.policy.id === policyId)
    if (!view) {
        let config: unknown
        try {
            config = await fetchJson(publicConfigUrl(publicBucketUrl, ids), {}, opts)
        } catch (e) {
            if (opts.signal?.aborted) throw e
            return {ok: false, problem: EPolicyProblem.UNAVAILABLE}
        }
        const text = configPolicy(config, ids)
        const examined = text ? examinePolicy(sdk, text) : undefined
        if (examined?.ok && examined.view.policy.id === policyId) view = examined.view
    }
    if (!view) return {ok: false, problem: EPolicyProblem.UNAVAILABLE}
    if (!coversElection(view.policy, ids))
        return {ok: false, problem: EPolicyProblem.OTHER_ELECTION}
    return {ok: true, view}
}

// -- The hub ---------------------------------------------------------------------

/** The hub whose ballot verifier this is, as its header shows it. */
export interface IHubIdentity {
    /** The hub's name in the policy, or what the settings call it. */
    name?: string
    /** Who runs it, from the policy. */
    operator?: string
}

/** A policy hub's ballot verifier, from its `hub-ballot-verifier` line, once
 * the vendored @vitrine/verify reads it. */
const ballotVerifierOf = (hub: unknown): string | undefined => {
    const url = isObject(hub) ? hub.ballotVerifier : undefined
    return typeof url === "string" ? url : undefined
}

const withSlash = (url: string): string => (url.endsWith("/") ? url : `${url}/`)

/** Which hub this verifier is: the one the settings name, else the one whose
 * `hub-ballot-verifier` URL this page is under. Its operator comes from the
 * policy. Nothing when neither says. */
export const hubIdentity = (
    policy: Policy | undefined,
    configured: string | undefined,
    location: string
): IHubIdentity => {
    const hubs = policy ? Array.from(policy.hubs.values()) : []
    const named = configured?.trim()
    const hub = named
        ? hubs.find((h) => h.name === named)
        : hubs.find((h) => {
              const url = ballotVerifierOf(h)
              return url !== undefined && location.startsWith(withSlash(url))
          })
    const name = named || hub?.name
    return {...(name ? {name} : {}), ...(hub?.operator ? {operator: hub.operator} : {})}
}

// -- The ballot's entries ------------------------------------------------------------

/** A server of the policy that may list a bucket. */
export interface IBucketServer {
    server: string
    log: string
}

/** The log servers and monitors of a policy that may list the buckets of its
 * ballots log, as the voting portal's tracker asks them: the log's own server
 * first, then the monitors in the policy's order. The log's server is the
 * log's URL without the log's path, which is the origin without the domain. */
export const bucketServers = (policy: Policy): IBucketServer[] => {
    const servers: IBucketServer[] = []
    const ballots = policy.logs.get(BALLOTS_LOG)
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

export enum EEntriesKind {
    /** The bucket lists the ballot. */
    FOUND = "found",
    /** The bucket doesn't list it. */
    NONE = "none",
    /** No server of the policy lists buckets. */
    UNSUPPORTED = "unsupported",
    /** The servers that list them didn't answer. */
    UNAVAILABLE = "unavailable",
}

export type TEntries =
    | {kind: EEntriesKind.FOUND; match: BallotMatch}
    | {kind: EEntriesKind.NONE | EEntriesKind.UNSUPPORTED | EEntriesKind.UNAVAILABLE}

/** The ballot's entries in the policy's ballots log: the servers are asked
 * for the bucket, one after another until one lists it whole, and the ballot
 * whose whole Tracking ID is the link's, in the link's election, is kept. A
 * ballot the bucket doesn't list is not in the record, as far as the servers
 * that answered know. */
export const findBallotEntries = async (
    sdk: IVitrineSdk,
    policy: Policy,
    trackingId: string,
    ids: IAuditedIds,
    opts: IFetchOptions = {}
): Promise<TEntries> => {
    const wanted = ballotIdChars(trackingId)
    let unavailable = false
    let partial = false
    for (const {server, log} of bucketServers(policy)) {
        try {
            const answer = await sdk.verify.lookupTrackingId(server, log, groupBallotId(wanted), {
                fetch: sdkFetch(opts.fetch),
                ...(opts.signal ? {signal: opts.signal} : {}),
            })
            const match = answer.matches.find(
                (m) =>
                    m.exact &&
                    ballotIdChars(m.trackingId) === wanted &&
                    m.event === ids.eventId &&
                    m.election === ids.electionId
            )
            if (match) return {kind: EEntriesKind.FOUND, match}
            // A server that stopped at its limit may have left it out.
            if (!answer.truncated) return {kind: EEntriesKind.NONE}
            partial = true
        } catch (error) {
            if (opts.signal?.aborted) throw error
            if (error instanceof sdk.verify.BucketLookupError && error.status === 404) continue
            unavailable = true
        }
    }
    if (partial) return {kind: EEntriesKind.NONE}
    return {kind: unavailable ? EEntriesKind.UNAVAILABLE : EEntriesKind.UNSUPPORTED}
}

export enum EStageKind {
    /** Its last entry is an audit, and it was never cast. */
    AUDITED = "audited",
    /** It was cast: a cast ballot is never decoded. */
    CAST = "cast",
    /** The ballot box received it, and it is neither cast nor audited yet. */
    PENDING = "pending",
}

export type TStage =
    | {kind: EStageKind.AUDITED; entry: BallotStageEntry}
    | {kind: EStageKind.CAST | EStageKind.PENDING}

/** What the ballot's entries say it is. One that was cast is cast, whatever
 * else the log holds for it. */
export const ballotStage = (match: BallotMatch): TStage => {
    if (match.entries.some((entry) => entry.stage === "cast")) return {kind: EStageKind.CAST}
    const audited = match.entries.filter((entry) => entry.stage === "audited")
    const last = audited[audited.length - 1]
    return last ? {kind: EStageKind.AUDITED, entry: last} : {kind: EStageKind.PENDING}
}

/** The reference of the ballot's `audited` entry, which the log's receipt
 * route serves the proof of, as for a receipt's reference. The entry is the
 * one the bucket listed, rebuilt from its fields. */
export const auditedReference = (
    sdk: IVitrineSdk,
    view: IPolicyView,
    match: BallotMatch,
    entry: BallotStageEntry,
    verifier: string
): ReceiptReference => ({
    verifier,
    policyPrefix: sdk.pure.policyPrefix(view.policy.id),
    log: BALLOTS_LOG,
    index: BigInt(entry.index),
    entry: sdk.pure.entries.buildElectionEntry({
        eventId: match.event,
        electionId: match.election,
        stage: "audited",
        ballotId: match.ballotId,
        acceptedAt: entry.acceptedAt,
    }),
})

// -- The ballot ----------------------------------------------------------------------

/** The ballot locator's query: the public record of a ballot, by its whole
 * Tracking ID (the voting portal's queries/LocateBallot.ts). */
const LOCATE_BALLOT = `query LocateBallot($electionEventId: uuid!, $electionId: uuid!, $ballotId: String!) {
    locate_ballot(election_event_id: $electionEventId, election_id: $electionId, ballot_id: $ballotId) {
        status
        ballot_id
        content
    }
}`

export enum ELocateKind {
    FOUND = "found",
    NOT_FOUND = "not-found",
    /** The locator didn't answer, or refused. */
    UNAVAILABLE = "unavailable",
}

export type TLocate =
    | {kind: ELocateKind.FOUND; content: string}
    | {kind: ELocateKind.NOT_FOUND | ELocateKind.UNAVAILABLE}

/** Asks Step's public ballot locator for the ballot's record, with no
 * credentials: the audited ballot and the secret the portal published. What
 * it answers is checked against the Tracking ID before anything is shown. */
export const locateBallot = async (
    graphqlUrl: string,
    ids: IAuditedIds,
    trackingId: string,
    opts: IFetchOptions = {}
): Promise<TLocate> => {
    let answer: unknown
    try {
        answer = await fetchJson(
            graphqlUrl,
            {
                method: "POST",
                headers: {"Content-Type": "application/json"},
                body: JSON.stringify({
                    operationName: "LocateBallot",
                    query: LOCATE_BALLOT,
                    variables: {
                        electionEventId: ids.eventId,
                        electionId: ids.electionId,
                        ballotId: ballotIdChars(trackingId),
                    },
                }),
            },
            opts
        )
    } catch (e) {
        if (opts.signal?.aborted) throw e
        return {kind: ELocateKind.UNAVAILABLE}
    }
    const data = isObject(answer) && isObject(answer.data) ? answer.data : undefined
    const located = data && isObject(data.locate_ballot) ? data.locate_ballot : undefined
    if (!located) return {kind: ELocateKind.UNAVAILABLE}
    if (
        located.status === "found" &&
        typeof located.content === "string" &&
        typeof located.ballot_id === "string" &&
        ballotIdChars(located.ballot_id) === ballotIdChars(trackingId)
    ) {
        return {kind: ELocateKind.FOUND, content: located.content}
    }
    return {kind: ELocateKind.NOT_FOUND}
}

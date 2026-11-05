// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Runs the audited ballot screen's checks (services/vitrine/auditedBallot.ts):
// the link's policy, the ballot's entries in a bucket of the ballots log,
// then, for an audited ballot, the proof of its `audited` entry, asked of the
// log alone, and the ballot from the public locator, checked against its
// Tracking ID and decoded. Nothing is shown as the voter's until all of them
// pass; a cast ballot is never fetched, let alone decoded.

import {useCallback, useEffect, useRef, useState} from "react"
import type {BallotStageEntry} from "@vitrine/verify"
import {
    auditBallot,
    EAuditOutcome,
    provideAuditService,
    TAudit,
    TAuditedRecord,
    TAuditService,
} from "../../services/vitrine/ballotAudit"
import {ERecordKind, classifyRecord} from "../../services/vitrine/ballotRecord"
import {
    auditedReference,
    ballotStage,
    EEntriesKind,
    ELocateKind,
    EPolicyProblem,
    EStageKind,
    findBallotEntries,
    hubIdentity,
    IAuditedIds,
    IAuditedLink,
    IHubIdentity,
    loadAuditedPolicy,
    locateBallot,
} from "../../services/vitrine/auditedBallot"
import {EStepState, tiersState} from "../../services/vitrine/ballotStatus"
import {
    chainFromReference,
    checkRound,
    ECheckRound,
    expectFromForm,
} from "../../services/vitrine/check"
import type {FetchLike} from "../../services/vitrine/http"
import type {IPolicyView} from "../../services/vitrine/policies"
import {IVitrineSdk, loadVitrineSdk} from "../../services/vitrine/sdk"

/** Where the screen is. */
export enum EAuditedPhase {
    LOADING = "loading",
    /** The link lacks the full Tracking ID or the policy ID. */
    BAD_LINK = "bad-link",
    /** The link's policy can't be had: not pinned, and not the event's. */
    NO_POLICY = "no-policy",
    /** The policy is for another election than the link's. */
    OTHER_ELECTION = "other-election",
    /** The public record has no ballot with this Tracking ID. */
    NOT_FOUND = "not-found",
    /** The ballot was cast: it is never decoded. */
    CAST = "cast",
    /** The ballot box received it, and it is neither cast nor audited yet. */
    PENDING = "pending",
    /** The log has the audit, but the locator doesn't have the ballot. */
    NO_CONTENT = "no-content",
    /** The record's servers or the locator didn't answer. */
    UNAVAILABLE = "unavailable",
    /** The log's proof of the audited entry doesn't check out. */
    RECORD_FAILED = "record-failed",
    /** The ballot isn't the one the Tracking ID names, or its secret doesn't
     * reproduce it. */
    MISMATCH = "mismatch",
    /** The ballot couldn't be checked or decoded here. */
    UNCHECKED = "unchecked",
    /** All checks passed: the voter compares. */
    READY = "ready",
}

/** The state of one of the checks the aside lists. */
export enum ECheckState {
    WAITING = "waiting",
    DONE = "done",
    FAILED = "failed",
}

export interface IAuditedChecks {
    record: ECheckState
    secret: ECheckState
    decoded: ECheckState
}

export interface IAuditedBallot {
    phase: EAuditedPhase
    hub: IHubIdentity
    view?: IPolicyView
    /** The ballot's `audited` entry in the log. */
    entry?: BallotStageEntry
    record?: TAuditedRecord
    audit?: TAudit
    checks: IAuditedChecks
    retry: () => void
}

export interface IAuditedDeps {
    loadSdk: () => Promise<IVitrineSdk>
    fetch?: FetchLike
    /** The device clock, POSIX seconds. */
    now?: () => number
    auditService: () => TAuditService
    /** The page's address, which may say which hub this verifier is. */
    location: () => string
}

export interface IAuditedInput {
    ids: IAuditedIds
    link: IAuditedLink | undefined
    publicBucketUrl: string
    graphqlUrl: string
    /** The hub this verifier is, as the settings name it. */
    hubName?: string
}

interface IState {
    phase: EAuditedPhase
    hub: IHubIdentity
    view?: IPolicyView
    entry?: BallotStageEntry
    record?: TAuditedRecord
    audit?: TAudit
    checks: IAuditedChecks
}

const WAITING: IAuditedChecks = {
    record: ECheckState.WAITING,
    secret: ECheckState.WAITING,
    decoded: ECheckState.WAITING,
}

const DEFAULT_DEPS: IAuditedDeps = {
    loadSdk: loadVitrineSdk,
    auditService: provideAuditService,
    location: () => window.location.href,
}

enum ERecordProof {
    DONE = "done",
    FAILED = "failed",
    UNAVAILABLE = "unavailable",
}

/** Whether the ballot's audit is the record's: its entry is proved in the
 * log against a checkpoint the log signed, by the log only. */
const proveAudited = async (
    sdk: IVitrineSdk,
    view: IPolicyView,
    ids: IAuditedIds,
    link: IAuditedLink,
    reference: ReturnType<typeof auditedReference>,
    opts: {fetch?: FetchLike; now?: () => number; signal: AbortSignal}
): Promise<ERecordProof> => {
    const expect = expectFromForm(sdk, {
        eventId: ids.eventId,
        electionId: ids.electionId,
        ballotId: link.trackingId,
        stage: "audited",
    })
    if (!expect) return ERecordProof.FAILED
    const history = new sdk.verify.PolicyHistory(view.policy)
    try {
        const chain = await chainFromReference(sdk, reference, view, history, opts)
        const {result} = await checkRound(
            sdk,
            {chain, view, expect, history},
            ECheckRound.LOG,
            opts
        )
        const state = tiersState(result, ["record"])
        if (state === EStepState.DONE) return ERecordProof.DONE
        return state === EStepState.FAILED || state === EStepState.DAMAGED
            ? ERecordProof.FAILED
            : ERecordProof.UNAVAILABLE
    } catch (e) {
        if (opts.signal.aborted) throw e
        // The log has no receipt for the entry, is quiet, or didn't answer.
        if (e instanceof sdk.verify.UpgradeError && e.code === "entry-mismatch") {
            return ERecordProof.FAILED
        }
        return ERecordProof.UNAVAILABLE
    }
}

export const useAuditedBallot = (
    input: IAuditedInput,
    deps: Partial<IAuditedDeps> = {}
): IAuditedBallot => {
    const [state, setState] = useState<IState>({
        phase: EAuditedPhase.LOADING,
        hub: input.hubName ? {name: input.hubName} : {},
        checks: WAITING,
    })
    const depsRef = useRef<IAuditedDeps>({...DEFAULT_DEPS, ...deps})
    depsRef.current = {...DEFAULT_DEPS, ...deps}
    const [attempt, setAttempt] = useState(0)
    const {ids, link, publicBucketUrl, graphqlUrl, hubName} = input
    const {tenantId, eventId, electionId} = ids
    const trackingId = link?.trackingId
    const policyId = link?.policyId

    useEffect(() => {
        const controller = new AbortController()
        const {signal} = controller
        const {loadSdk, fetch, now, auditService, location} = depsRef.current
        const route: IAuditedIds = {tenantId, eventId, electionId}
        const named: IHubIdentity = hubName ? {name: hubName} : {}
        const opts = {...(fetch ? {fetch} : {}), signal}
        const show = (next: Partial<IState> & {phase: EAuditedPhase}) => {
            if (!signal.aborted) setState((s) => ({...s, ...next}))
        }
        setState({phase: EAuditedPhase.LOADING, hub: named, checks: WAITING})
        if (!trackingId || !policyId) {
            show({phase: EAuditedPhase.BAD_LINK})
            return () => controller.abort()
        }
        const target: IAuditedLink = {trackingId, policyId}
        void (async () => {
            try {
                const sdk = await loadSdk()
                const loaded = await loadAuditedPolicy(sdk, publicBucketUrl, route, policyId, opts)
                if (!loaded.ok) {
                    show({
                        phase:
                            loaded.problem === EPolicyProblem.OTHER_ELECTION
                                ? EAuditedPhase.OTHER_ELECTION
                                : EAuditedPhase.NO_POLICY,
                    })
                    return
                }
                const {view} = loaded
                const hub = hubIdentity(view.policy, hubName, location())
                show({phase: EAuditedPhase.LOADING, view, hub})

                const found = await findBallotEntries(sdk, view.policy, trackingId, route, opts)
                if (found.kind !== EEntriesKind.FOUND) {
                    show({
                        phase:
                            found.kind === EEntriesKind.NONE
                                ? EAuditedPhase.NOT_FOUND
                                : EAuditedPhase.UNAVAILABLE,
                    })
                    return
                }
                const stage = ballotStage(found.match)
                if (stage.kind !== EStageKind.AUDITED) {
                    show({
                        phase:
                            stage.kind === EStageKind.CAST
                                ? EAuditedPhase.CAST
                                : EAuditedPhase.PENDING,
                    })
                    return
                }
                const {entry} = stage
                show({phase: EAuditedPhase.LOADING, entry})

                const page = location().split("#")[0]
                const reference = auditedReference(sdk, view, found.match, entry, page)
                const [proof, located] = await Promise.all([
                    proveAudited(sdk, view, route, target, reference, {
                        ...opts,
                        ...(now ? {now} : {}),
                    }),
                    locateBallot(graphqlUrl, route, trackingId, opts),
                ])
                if (proof === ERecordProof.FAILED) {
                    show({
                        phase: EAuditedPhase.RECORD_FAILED,
                        checks: {...WAITING, record: ECheckState.FAILED},
                    })
                    return
                }
                if (proof === ERecordProof.UNAVAILABLE) {
                    show({phase: EAuditedPhase.UNAVAILABLE})
                    return
                }
                const recordDone = {...WAITING, record: ECheckState.DONE}
                if (located.kind !== ELocateKind.FOUND) {
                    show({
                        phase:
                            located.kind === ELocateKind.NOT_FOUND
                                ? EAuditedPhase.NO_CONTENT
                                : EAuditedPhase.UNAVAILABLE,
                        checks: recordDone,
                    })
                    return
                }
                const record = classifyRecord(located.content)
                if (record.kind !== ERecordKind.AUDITED) {
                    show({phase: EAuditedPhase.UNCHECKED, checks: recordDone})
                    return
                }
                const audit = auditBallot(auditService(), record, trackingId)
                if (audit.outcome === EAuditOutcome.MISMATCH) {
                    show({
                        phase: EAuditedPhase.MISMATCH,
                        record,
                        audit,
                        checks: {...recordDone, secret: ECheckState.FAILED},
                    })
                    return
                }
                if (audit.outcome === EAuditOutcome.NOT_VERIFIABLE) {
                    show({phase: EAuditedPhase.UNCHECKED, record, audit, checks: recordDone})
                    return
                }
                show({
                    phase: EAuditedPhase.READY,
                    record,
                    audit,
                    checks: {
                        record: ECheckState.DONE,
                        secret: ECheckState.DONE,
                        decoded: ECheckState.DONE,
                    },
                })
            } catch (e) {
                if (signal.aborted) return
                console.error("Failed to check the audited ballot:", e)
                show({phase: EAuditedPhase.UNAVAILABLE})
            }
        })()
        return () => controller.abort()
    }, [
        tenantId,
        eventId,
        electionId,
        trackingId,
        policyId,
        publicBucketUrl,
        graphqlUrl,
        hubName,
        attempt,
    ])

    const retry = useCallback(() => setAttempt((n) => n + 1), [])
    return {...state, retry}
}

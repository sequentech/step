// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The overview at the top of the Transparency tab: the record's health, its
// figures and the policy's witnesses, hubs and monitors, from the sealed
// policy and what the sidecar reports. A figure Harvest doesn't report stays
// undefined, so the tab says so instead of guessing.

import type {Policy} from "@vitrine/verify"
import {ETransparencyMode, ILogView} from "./Transparency"

export enum EHealth {
    DISABLED = "disabled",
    /** The verifier is still loading, so the policy isn't checked yet. */
    CHECKING = "checking",
    /** The verifier couldn't start in this browser. */
    UNCHECKED = "unchecked",
    NO_POLICY = "noPolicy",
    UNREPORTED = "unreported",
    ATTENTION = "attention",
    HEALTHY = "healthy",
}

/** Where the tab is with the event's stored policy. */
export enum EPolicyCheck {
    CHECKING = "checking",
    UNCHECKED = "unchecked",
    NONE = "none",
    INVALID = "invalid",
    SEALED = "sealed",
}

export enum EHealthIssue {
    STORED_POLICY_INVALID = "storedPolicyInvalid",
    POLICY_ID_MISMATCH = "policyIdMismatch",
    LOG_OTHER_POLICY = "logOtherPolicy",
    LOG_ALERT = "logAlert",
    WITNESS_ALERT = "witnessAlert",
}

export interface IHealthIssue {
    issue: EHealthIssue
    log?: string
    witness?: string
    alert?: string
}

export interface IHealth {
    health: EHealth
    issues: IHealthIssue[]
}

export interface IHealthInput {
    mode: ETransparencyMode
    policy: EPolicyCheck
    /** The sealed policy's ID, as lowercase hex. */
    policyHex?: string
    /** The `policy_id` Harvest returned, if any, and its hex. */
    loggedPolicyId?: string | null
    loggedPolicyHex?: string
    logs: readonly ILogView[]
}

/** The record's health: what keeps it from being checked, the alerts the
 * logs and their witnesses report, or healthy. */
export const recordHealth = (input: IHealthInput): IHealth => {
    const only = (health: EHealth): IHealth => ({health, issues: []})
    if (input.mode === ETransparencyMode.DISABLED) return only(EHealth.DISABLED)
    if (input.policy === EPolicyCheck.CHECKING) return only(EHealth.CHECKING)
    if (input.policy === EPolicyCheck.UNCHECKED) return only(EHealth.UNCHECKED)
    if (input.policy === EPolicyCheck.NONE) return only(EHealth.NO_POLICY)

    const issues: IHealthIssue[] = []
    if (input.policy === EPolicyCheck.INVALID) {
        issues.push({issue: EHealthIssue.STORED_POLICY_INVALID})
    }
    if (input.loggedPolicyId && input.loggedPolicyHex !== input.policyHex) {
        issues.push({issue: EHealthIssue.POLICY_ID_MISMATCH})
    }
    for (const log of input.logs) {
        if (log.policyHex && input.policyHex && log.policyHex !== input.policyHex) {
            issues.push({issue: EHealthIssue.LOG_OTHER_POLICY, log: log.key})
        }
        if (log.alert) issues.push({issue: EHealthIssue.LOG_ALERT, log: log.key, alert: log.alert})
        for (const w of log.witnesses) {
            if (w.alert) {
                issues.push({issue: EHealthIssue.WITNESS_ALERT, log: log.key, witness: w.name})
            }
        }
    }
    if (issues.length > 0) return {health: EHealth.ATTENTION, issues}
    if (input.logs.length === 0) return only(EHealth.UNREPORTED)
    return only(EHealth.HEALTHY)
}

// -- The policy -----------------------------------------------------------------

export interface IQuorum {
    k: number
    n: number
}

export interface INamedParty {
    name: string
    /** Who runs it, when the policy says. */
    party?: string
}

export interface IPolicyFacts {
    visibility: ETransparencyMode
    operator: string
    /** The `platform` line: the party whose software runs the election. */
    platform?: string
    quorum: IQuorum
    appHubs: number
    /** App hubs run by neither the operator nor the platform. */
    outsideHubs: number
    /** The policy's `anchor-hubs` rule, when it has one: `min` hubs, at least
     * `outside` of them run by neither the operator nor the platform. */
    hubRule?: {min: number; outside: number}
    bootstrapHubs: string[]
    mmd: number
    anchorPeriod: number
    browserPeriod: number
    monitors: INamedParty[]
    monitorQuorum: number
    redaction?: {k: number; names: string[]}
}

/** The witness quorum as k of n: the quorum's group, or the one witness it
 * names. */
export const witnessQuorum = (set: Pick<Policy["witnesses"], "groups" | "quorum">): IQuorum => {
    const group = set.groups.get(set.quorum)
    return group ? {k: group.k, n: group.members.length} : {k: 1, n: 1}
}

/** True when `party` is neither the policy's operator nor its platform. */
const runByOthers = (policy: Pick<Policy, "operator" | "platform">, party: string): boolean =>
    party !== policy.operator && party !== policy.platform

export const policyFacts = (policy: Policy): IPolicyFacts => {
    const hubs = Array.from(policy.hubs.values())
    const appHubs = hubs.filter((h) => !h.bootstrap)
    const facts: IPolicyFacts = {
        visibility:
            policy.visibility === "private" ? ETransparencyMode.PRIVATE : ETransparencyMode.PUBLIC,
        operator: policy.operator,
        quorum: witnessQuorum(policy.witnesses),
        appHubs: appHubs.length,
        outsideHubs: appHubs.filter((h) => runByOthers(policy, h.operator)).length,
        bootstrapHubs: hubs.filter((h) => h.bootstrap).map((h) => h.name),
        mmd: policy.mmd,
        anchorPeriod: policy.anchorPeriod,
        browserPeriod: policy.browserPeriod,
        monitors: Array.from(policy.monitors.values()).map((m) =>
            m.party && m.party !== m.name ? {name: m.name, party: m.party} : {name: m.name}
        ),
        monitorQuorum: policy.monitorQuorum,
    }
    if (policy.platform !== undefined) facts.platform = policy.platform
    if (policy.anchorHubs) {
        facts.hubRule = {min: policy.anchorHubs.min, outside: policy.anchorHubs.minIndependent}
    }
    if (policy.redaction) {
        facts.redaction = {k: policy.redaction.k, names: [...policy.redaction.names]}
    }
    return facts
}

// -- Witnesses and hubs -----------------------------------------------------------

export enum EWitnessState {
    SIGNING = "signing",
    LAGGING = "lagging",
    ALERT = "alert",
    UNREPORTED = "unreported",
}

const WITNESS_RANK: Record<EWitnessState, number> = {
    [EWitnessState.UNREPORTED]: 0,
    [EWitnessState.SIGNING]: 1,
    [EWitnessState.LAGGING]: 2,
    [EWitnessState.ALERT]: 3,
}

export interface IWitnessRow extends INamedParty {
    state: EWitnessState
    /** How long ago it last cosigned, when it is behind. */
    lagSeconds?: number
}

/** The policy's witnesses, then any other a log reports, each in its worst
 * state across the event's logs. */
export const witnessRows = (
    policy: Policy | undefined,
    logs: readonly ILogView[]
): IWitnessRow[] => {
    const rows = new Map<string, IWitnessRow>()
    policy?.witnesses.witnesses.forEach((w) => {
        rows.set(
            w.name,
            w.partyGiven
                ? {name: w.name, party: w.party, state: EWitnessState.UNREPORTED}
                : {name: w.name, state: EWitnessState.UNREPORTED}
        )
    })
    for (const log of logs) {
        for (const w of log.witnesses) {
            const row = rows.get(w.name) ?? {name: w.name, state: EWitnessState.UNREPORTED}
            const state = w.alert
                ? EWitnessState.ALERT
                : w.current
                  ? EWitnessState.SIGNING
                  : EWitnessState.LAGGING
            if (WITNESS_RANK[state] > WITNESS_RANK[row.state]) row.state = state
            if (!w.current && w.cosignedAgeSeconds !== undefined) {
                row.lagSeconds = Math.max(row.lagSeconds ?? 0, w.cosignedAgeSeconds)
            }
            rows.set(w.name, row)
        }
    }
    return Array.from(rows.values())
}

export interface IHubRow {
    name: string
    operator: string
    /** When the hub last received an anchor of any of the logs, in POSIX
     * seconds. */
    lastAnchor?: number
}

const lastAnchorOf = (name: string, logs: readonly ILogView[]): number | undefined => {
    const times = logs.flatMap((log) =>
        log.hubs.flatMap((h) =>
            h.name === name && h.anchor?.time !== undefined ? [h.anchor.time] : []
        )
    )
    return times.length > 0 ? Math.max(...times) : undefined
}

/** The policy's app hubs, with when each last received an anchor. */
export const hubRows = (policy: Policy, logs: readonly ILogView[]): IHubRow[] =>
    Array.from(policy.hubs.values())
        .filter((h) => !h.bootstrap)
        .map((h) => {
            const row: IHubRow = {
                name: h.name,
                operator: h.operator,
            }
            const lastAnchor = lastAnchorOf(h.name, logs)
            if (lastAnchor !== undefined) row.lastAnchor = lastAnchor
            return row
        })

// -- Figures --------------------------------------------------------------------

export interface IRecordFigures {
    /** Entries in the logs' witnessed checkpoints. */
    entries?: number
    witnessedLogs: number
    queueRows?: number
    /** How long the oldest entry waiting to be sequenced has waited. */
    queueOldestSeconds?: number
    /** The newest anchor any app hub received. */
    lastAnchor?: number
    anchoredHubs: number
}

export const recordFigures = (
    hubs: readonly IHubRow[],
    logs: readonly ILogView[]
): IRecordFigures => {
    const witnessed = logs.flatMap((l) => (l.checkpoint ? [l.checkpoint.size] : []))
    const queues = logs.flatMap((l) => (l.queue ? [l.queue] : []))
    const waits = queues.flatMap((q) => (q.oldestSeconds !== undefined ? [q.oldestSeconds] : []))
    const anchors = hubs.flatMap((h) => (h.lastAnchor !== undefined ? [h.lastAnchor] : []))
    const figures: IRecordFigures = {
        witnessedLogs: witnessed.length,
        anchoredHubs: anchors.length,
    }
    if (witnessed.length > 0) figures.entries = witnessed.reduce((a, b) => a + b, 0)
    if (queues.length > 0) figures.queueRows = queues.reduce((a, q) => a + q.rows, 0)
    if (waits.length > 0) figures.queueOldestSeconds = Math.max(...waits)
    if (anchors.length > 0) figures.lastAnchor = Math.max(...anchors)
    return figures
}

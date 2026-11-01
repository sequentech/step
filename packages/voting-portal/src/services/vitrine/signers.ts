// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Who signed and which hubs (design board WitnessList): the lists that open
// under the "Confirmed by witnesses" and "Anchored" steps, for the status
// dialog and the ballot tracker. The names and who runs each come from the
// policy; a time shows only for a signature this browser verified, read from
// the receipt chain; whether a signer is on time comes from the check's tiers.
// No row is called independent, and none carries a badge: the guarantee is in
// the rules the footnote states, as far as the policy backs them.

import {explorersOf, type ChainResult, type Policy, type PolicyHub} from "@vitrine/verify"
import {witnessCount} from "./ballotTracker"
import type {TVitrinePure} from "./sdk"
import type {ITranslation} from "./statusRows"
import {EVitrineLog} from "./types"

const KEYS = "vitrine.signers"

export enum ESignerKind {
    WITNESSES = "witnesses",
    HUBS = "hubs",
}

/** Where a signer stands: signed (or anchored), still to come, or past its
 * deadline. Never a failure: a signer that has not signed is not at fault. */
export enum ESignerState {
    DONE = "done",
    PENDING = "pending",
    LATE = "late",
}

/** The party that provides the voting platform: the policy's `platform` line,
 * or none when the policy has no such line. */
export const platformOf = (policy: Policy): string | undefined => policy.platform

/** Whether the election authority (the policy's operator) or the platform
 * runs a party. Their sites get no "Check on its site" link. */
export const isOperatorOrPlatform = (
    policy: Policy,
    party: string,
    platform: string | undefined = platformOf(policy)
): boolean => party === policy.operator || (platform !== undefined && party === platform)

/** The site that the policy names for a party: the explorer of a monitor or a
 * hub it runs. The operator's own explorers are not a party's site. */
const siteOf = (policy: Policy, party: string, own?: string): string | undefined =>
    own ??
    explorersOf(policy).find((explorer) => explorer.kind !== "operator" && explorer.party === party)
        ?.url

/** When each signer signed what this browser checked, in POSIX seconds, and
 * the size of the checkpoint they signed. */
export interface ISignerTimes {
    /** By witness name: its earliest cosignature of the record's checkpoint. */
    witnesses: ReadonlyMap<string, number>
    /** By hub name: the hub quorum's time on the link that anchors the record. */
    hubs: ReadonlyMap<string, number>
    /** The size of the checkpoint that the record's proof is against. */
    size: number
}

export interface ISignerTimesInput {
    policy: Policy
    /** The receipt chain, as text. */
    chain: string
    /** The signed time of the check, in POSIX seconds. */
    now: number
}

const witnessTimes = (cosigs: readonly {witness: string; timestamp: number}[]) => {
    const times = new Map<string, number>()
    for (const {witness, timestamp} of cosigs) {
        times.set(witness, Math.min(times.get(witness) ?? timestamp, timestamp))
    }
    return times
}

/** The times that the receipt chain gives, as far as it can be read and
 * verified. Nothing throws: a chain that cannot be read gives no times, and
 * the lists then stay shut.
 *
 * The witnesses are those that cosigned the receipt that met the quorum, the
 * later of the chain's `record` and `issued` receipts as the verifier takes
 * it; while neither does, those of the latest receipt, so far. */
export const signerTimes = (
    pure: TVitrinePure,
    {policy, chain: text, now}: ISignerTimesInput
): ISignerTimes | undefined => {
    try {
        const chain = pure.parseChain(text)
        const log = policy.logs.get(chain.log)
        if (!log) return undefined
        const checked = [chain.record, chain.issued].flatMap((body) => {
            if (!body) return []
            const proof = pure.proofs.parseTlogProof(body)
            return [
                pure.checkpoints.verifyCheckpoint(
                    proof.checkpoint,
                    log.vkey,
                    policy.witnesses,
                    now,
                    {
                        logTime: log.logTime,
                    }
                ),
            ]
        })
        const receipt = checked.find((c) => policy.witnesses.meets(c.cosigs)) ?? checked[0]
        if (!receipt) return undefined
        const hubs = new Map<string, number>()
        const subject = pure.hubs.logSubject(policy, chain.log)
        for (const [name, blocks] of Array.from(chain.hubs)) {
            const hub = policy.hubs.get(name)
            if (!hub || hub.bootstrap || !blocks.anchor) continue
            try {
                const link = pure.hubs.verifyHubLink(blocks.anchor, hub, subject, now)
                if (link.hubQuorum && link.hubQuorumTime !== undefined) {
                    hubs.set(name, link.hubQuorumTime)
                }
            } catch {
                // A link that does not verify tells no time; the check's tiers say the rest.
            }
        }
        return {witnesses: witnessTimes(receipt.cosigs), hubs, size: Number(receipt.cp.size)}
    } catch {
        return undefined
    }
}

export interface ISignerRow {
    /** The signer's name in the policy; none for the stand-in hub. */
    name?: string
    /** Who runs it, as the policy states; none for the stand-in hub. */
    party?: string
    /** The hubs that its party also runs, for a witness. */
    alsoRuns: string[]
    state: ESignerState
    /** POSIX seconds: when it signed or anchored what this browser checked. */
    at?: number
    /** A link into the signer's own site, which checks it there. */
    site?: string
    /** The hub that stands in where browsers check, until browsers check anchors. */
    standIn?: boolean
}

export interface ISignerList {
    kind: ESignerKind
    rows: ISignerRow[]
    /** What the footnote says the policy needs. */
    rule: ITranslation
}

export interface ISoleHub {
    name: string
    party: string
    /** Whether the platform runs it. */
    platform: boolean
}

const appHubsOf = (policy: Policy): PolicyHub[] =>
    Array.from(policy.hubs.values()).filter((hub) => !hub.bootstrap)

/** The hub that anchors the record, when the policy has only one. */
export const soleHubOf = (
    policy: Policy,
    platform: string | undefined = platformOf(policy)
): ISoleHub | undefined => {
    const hubs = appHubsOf(policy)
    if (hubs.length !== 1) return undefined
    const [hub] = hubs
    return {name: hub.name, party: hub.operator, platform: hub.operator === platform}
}

const tierOf = (result: ChainResult, name: "witnesses" | "hubs" | "browser") =>
    result.tiers.find((tier) => tier.tier === name)

const text = (key: string, values?: ITranslation["values"]): ITranslation => ({
    key: `${KEYS}.${key}`,
    ...(values ? {values} : {}),
})

interface ILinkParts {
    policy: Policy
    result: ChainResult
    size: number
    /** The query that names the signer: `cosigner` or `anchor`. */
    signer: [key: string, name: string]
    site: string
}

/** A link into an explorer, at the policy and the checkpoint this browser
 * checked, naming the signer to check there. */
const signerLink = ({policy, result, size, signer, site}: ILinkParts): string => {
    const query = new URLSearchParams({
        policy: policy.id,
        log: result.entry?.log ?? EVitrineLog.BALLOTS,
        checkpoint: String(size),
    })
    query.set(...signer)
    const hash = site.indexOf("#")
    return `${hash < 0 ? site : site.slice(0, hash)}#${query.toString()}`
}

const partiesOf = (policy: Policy): Map<string, number> => {
    const parties = new Map<string, number>()
    for (const witness of Array.from(policy.witnesses.witnesses.values())) {
        parties.set(witness.party, (parties.get(witness.party) ?? 0) + 1)
    }
    return parties
}

/** The footnote of the witnesses: how many signed and what the policy needs.
 * "No single party can confirm alone" only when the policy's own quorum and
 * parties say so: no party runs as many witnesses as the quorum needs. */
const witnessRule = (policy: Policy, signed: number): ITranslation => {
    const count = witnessCount(policy, signed)
    const values = {signed, total: count?.total ?? signed}
    if (count?.needed === undefined) return text("rule.witnessesCount", values)
    const most = Math.max(0, ...Array.from(partiesOf(policy).values()))
    return text(most < count.needed ? "rule.witnessesAlone" : "rule.witnesses", {
        ...values,
        needed: count.needed,
    })
}

/** The footnote of the hubs: the count the policy needs and, when it asks, how
 * many are run by someone other than the election authority. */
const hubRule = (policy: Policy, platform: string | undefined): ITranslation => {
    const sole = soleHubOf(policy, platform)
    if (sole) {
        return text(sole.platform ? "rule.oneHubPlatform" : "rule.oneHub", {party: sole.party})
    }
    const count = appHubsOf(policy).length
    const independent = policy.anchorHubs?.minIndependent ?? 0
    if (independent < 1) return text("rule.hubsAll", {count})
    return text(platform === undefined ? "rule.hubs" : "rule.hubsPlatform", {count, independent})
}

export interface ISignerListInput {
    policy?: Policy
    result?: ChainResult
    times?: ISignerTimes
    /** Whether the hubs include where browsers check, as the dialog's step does. */
    standIn: boolean
    /** The party that runs the platform, when the policy names it. */
    platform?: string
}

export interface ISignerLists {
    witnesses?: ISignerList
    hubs?: ISignerList
}

const withLink = (
    row: ISignerRow,
    party: string,
    own: string | undefined,
    parts: Omit<ILinkParts, "site">,
    platform: string | undefined
): ISignerRow => {
    if (row.state !== ESignerState.DONE || isOperatorOrPlatform(parts.policy, party, platform)) {
        return row
    }
    const site = siteOf(parts.policy, party, own)
    return site ? {...row, site: signerLink({...parts, site})} : row
}

const witnessRows = ({
    policy,
    result,
    times,
    platform,
}: Required<Pick<ISignerListInput, "policy" | "result" | "times">> &
    Pick<ISignerListInput, "platform">): ISignerRow[] => {
    const late = tierOf(result, "witnesses")?.status === "late"
    return Array.from(policy.witnesses.witnesses.values()).map((witness) => {
        const at = times.witnesses.get(witness.name)
        const row: ISignerRow = {
            name: witness.name,
            party: witness.party,
            alsoRuns: appHubsOf(policy)
                .filter((hub) => hub.operator === witness.party)
                .map((hub) => hub.name),
            state:
                at !== undefined
                    ? ESignerState.DONE
                    : late
                      ? ESignerState.LATE
                      : ESignerState.PENDING,
            ...(at === undefined ? {} : {at}),
        }
        return withLink(
            row,
            witness.party,
            undefined,
            {policy, result, size: times.size, signer: ["cosigner", witness.name]},
            platform
        )
    })
}

const stateOfTier = (status: string | undefined): ESignerState => {
    if (status === "verified") return ESignerState.DONE
    return status === "late" ? ESignerState.LATE : ESignerState.PENDING
}

const hubRows = ({
    policy,
    result,
    times,
    standIn,
    platform,
}: Required<Pick<ISignerListInput, "policy" | "result" | "times" | "standIn">> &
    Pick<ISignerListInput, "platform">): ISignerRow[] => {
    const tier = tierOf(result, "hubs")
    const rows = appHubsOf(policy).map((hub) => {
        const state = stateOfTier(tier?.hubs?.find((detail) => detail.hub === hub.name)?.status)
        const at = times.hubs.get(hub.name)
        const row: ISignerRow = {
            name: hub.name,
            party: hub.operator,
            alsoRuns: [],
            state,
            ...(state === ESignerState.DONE && at !== undefined ? {at} : {}),
        }
        return withLink(
            row,
            hub.operator,
            hub.explorer,
            {policy, result, size: times.size, signer: ["anchor", hub.name]},
            platform
        )
    })
    const browser = tierOf(result, "browser")
    if (standIn && result.policy.browser === "bootstrap" && browser?.status !== "unused") {
        rows.push({alsoRuns: [], state: stateOfTier(browser?.status), standIn: true})
    }
    return rows
}

/** The lists that open under the witnesses and anchors steps. A list is left
 * out when the policy, the check or the receipt's times are missing, or when
 * the policy has no signers of its kind. */
export const signerLists = ({
    policy,
    result,
    times,
    standIn,
    platform = policy ? platformOf(policy) : undefined,
}: ISignerListInput): ISignerLists => {
    if (!policy || !result || !times) return {}
    const input = {policy, result, times, standIn, platform}
    const lists: ISignerLists = {}
    const witnesses = witnessRows(input)
    if (witnesses.length > 0) {
        const signed = witnesses.filter((row) => row.state === ESignerState.DONE).length
        lists.witnesses = {
            kind: ESignerKind.WITNESSES,
            rows: witnesses,
            rule: witnessRule(policy, signed),
        }
    }
    const hubs = hubRows(input)
    if (appHubsOf(policy).length > 0 && tierOf(result, "hubs")?.status !== "unused") {
        lists.hubs = {kind: ESignerKind.HUBS, rows: hubs, rule: hubRule(policy, platform)}
    }
    return lists
}

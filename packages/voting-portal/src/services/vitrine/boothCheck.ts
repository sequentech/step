// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The check before encrypting (step-integration.md, section 2.6; Vitrine's
// specs/entry.md, section 5.2). The booth needs the election's config
// statement and the event's `voting-opened` milestone, each in the events log
// with a receipt that meets the quorum, and the key, styles, policy and
// release they name must be the ones the booth uses.

import type {Policy} from "@vitrine/verify"
import {electionKeyDigest, styleDigestHex, stylesDigest} from "./digests"
import {policyHex} from "./presentation"
import type {IVitrineSdk} from "./sdk"
import {
    IConfigStatement,
    parseConfigStatement,
    parseMilestoneStatement,
    VOTING_OPENED,
} from "./statements"
import {
    ETransparencyMode,
    EVitrineLog,
    IVitrineLoggedStatement,
    IVitrinePresentation,
    IVitrineVoterContext,
    VOTING_PORTAL_PACKAGE,
} from "./types"

const EVENTS_TAG = "vitrine/events/v1"
const CONFIG_KIND = "config"

// The booth needs the quorum's signatures on the checkpoint, not their time,
// so cosignatures dated after this device's clock count too: a clock that runs
// slow must not keep anyone from voting.
const ANY_TIME = Number.MAX_SAFE_INTEGER

/** Why the booth refused to encrypt. */
export enum EBoothCheckReason {
    VERIFIER_UNAVAILABLE = "verifier-unavailable",
    CONTEXT_MISSING = "context-missing",
    MODE_MISMATCH = "mode-mismatch",
    POLICY_MISSING = "policy-missing",
    POLICY_INVALID = "policy-invalid",
    POLICY_ID_MISMATCH = "policy-id-mismatch",
    POLICY_VISIBILITY_MISMATCH = "policy-visibility-mismatch",
    CONFIG_MISSING = "config-missing",
    CONFIG_INVALID = "config-invalid",
    CONFIG_UNVERIFIED = "config-unverified",
    OPENED_MISSING = "opened-missing",
    OPENED_INVALID = "opened-invalid",
    OPENED_UNVERIFIED = "opened-unverified",
    OPENED_CONFIG_MISMATCH = "opened-config-mismatch",
    EVENT_MISMATCH = "event-mismatch",
    ELECTION_MISMATCH = "election-mismatch",
    KEY_MISMATCH = "key-mismatch",
    STYLES_MISMATCH = "styles-mismatch",
    POLICY_MISMATCH = "policy-mismatch",
    RELEASE_MISMATCH = "release-mismatch",
    TRANSPARENCY_MISMATCH = "transparency-mismatch",
}

/** How `<vitrine-status>` shows a refusal: `pending` while something isn't
 * there yet, `invalid` for data that doesn't parse or verify, and `failed`
 * when verified statements contradict what the booth was given. */
export enum EBoothRefusalState {
    PENDING = "pending",
    INVALID = "invalid",
    FAILED = "failed",
}

export type TPinnedPolicy = {ok: true; policy: Policy} | {ok: false; reason: EBoothCheckReason}

/** The event's pinned policy: from the presentation for public events, from
 * Harvest after login for private ones. Either way its ID must be the one
 * the presentation pins, and its visibility the event's mode. */
export const pinnedPolicy = (
    sdk: IVitrineSdk,
    mode: ETransparencyMode,
    presentation: IVitrinePresentation | undefined,
    contextPolicy: string | null | undefined
): TPinnedPolicy => {
    const text = mode === ETransparencyMode.PUBLIC ? presentation?.policy : contextPolicy
    const pinned = policyHex(presentation?.policy_id)
    if (!text || !pinned) return {ok: false, reason: EBoothCheckReason.POLICY_MISSING}
    let policy: Policy
    try {
        policy = sdk.verify.parsePolicy(text)
    } catch {
        return {ok: false, reason: EBoothCheckReason.POLICY_INVALID}
    }
    if (policyHex(policy.id) !== pinned) {
        return {ok: false, reason: EBoothCheckReason.POLICY_ID_MISMATCH}
    }
    if (policy.visibility !== mode) {
        return {ok: false, reason: EBoothCheckReason.POLICY_VISIBILITY_MISMATCH}
    }
    return {ok: true, policy}
}

export interface IBoothCheckInput {
    mode: ETransparencyMode
    presentation?: IVitrinePresentation
    /** `vitrine_voter_context`, or undefined when it couldn't be fetched. */
    context?: IVitrineVoterContext
    eventId: string
    electionId: string
    /** The election public key the booth encrypts to, as the ballot style has it. */
    publicKey?: string
    /** The exact ballot-style text the portal received. */
    styleText?: string
    /** This portal's version. */
    release: string
}

export interface IBoothCheckPassed {
    ok: true
    policy: Policy
    config: IConfigStatement
}

export interface IBoothCheckRefused {
    ok: false
    reason: EBoothCheckReason
    state: EBoothRefusalState
    /** The pinned policy, when the refusal came after it was pinned: its
     * monitors get the observation. */
    policy?: Policy
}

export type TBoothCheckResult = IBoothCheckPassed | IBoothCheckRefused

type TLogged = {ok: true; text: string; entry: Uint8Array} | {ok: false; state: EBoothRefusalState}

/** A statement Harvest carried: its entry is an events/v1 entry of `kind`
 * holding exactly the statement, an authority signed it, and the chain proves
 * the entry is in the events log with a receipt that meets the quorum. */
const verifyLogged = (
    sdk: IVitrineSdk,
    policy: Policy,
    logged: IVitrineLoggedStatement,
    kind: string
): TLogged => {
    const {encoding, entries, notes} = sdk.pure
    const invalid: TLogged = {ok: false, state: EBoothRefusalState.INVALID}
    let entryBytes: Uint8Array
    let text: string
    try {
        entryBytes = encoding.b64decode(logged.entry)
        const entry = entries.parseEntry(entryBytes)
        if (entry.tag !== EVENTS_TAG) return invalid
        entries.checkLayout(entry)
        if (entries.fieldText(entry, "kind") !== kind) return invalid
        if (entries.fieldText(entry, "statement") !== logged.statement) return invalid
        const note = notes.parseNote(logged.statement)
        const authorities = Array.from(policy.authorities.values()).map((a) => a.vkey)
        if (notes.verifyNote(note, authorities).length === 0) return invalid
        text = note.text
    } catch {
        return invalid
    }
    const result = sdk.verify.verifyChain(logged.chain, policy, {
        expect: {entry: entryBytes},
        now: ANY_TIME,
    })
    const tiers = result.tiers.filter((t) => t.tier === "record" || t.tier === "witnesses")
    if (result.entry?.log === EVitrineLog.EVENTS && tiers.every((t) => t.status === "verified")) {
        return {ok: true, text, entry: entryBytes}
    }
    const statuses = tiers.map((t) => t.status)
    if (statuses.includes("failed")) return {ok: false, state: EBoothRefusalState.FAILED}
    if (statuses.includes("invalid") || result.entry?.log !== EVitrineLog.EVENTS) return invalid
    return {ok: false, state: EBoothRefusalState.PENDING}
}

/** Runs the whole check. It never throws: anything it can't check refuses. */
export const checkBooth = (sdk: IVitrineSdk, input: IBoothCheckInput): TBoothCheckResult => {
    const refuse = (
        reason: EBoothCheckReason,
        state: EBoothRefusalState,
        policy?: Policy
    ): IBoothCheckRefused => ({ok: false, reason, state, ...(policy ? {policy} : {})})
    const {context} = input
    if (!context) return refuse(EBoothCheckReason.CONTEXT_MISSING, EBoothRefusalState.PENDING)
    if (context.mode !== input.mode) {
        return refuse(EBoothCheckReason.MODE_MISMATCH, EBoothRefusalState.INVALID)
    }
    const pinned = pinnedPolicy(sdk, input.mode, input.presentation, context.policy)
    if (!pinned.ok) {
        const state =
            pinned.reason === EBoothCheckReason.POLICY_MISSING
                ? EBoothRefusalState.PENDING
                : EBoothRefusalState.INVALID
        return refuse(pinned.reason, state)
    }
    const {policy} = pinned
    const {pure} = sdk

    if (!context.config) {
        return refuse(EBoothCheckReason.CONFIG_MISSING, EBoothRefusalState.PENDING, policy)
    }
    const config = verifyLogged(sdk, policy, context.config, CONFIG_KIND)
    if (!config.ok) {
        const reason =
            config.state === EBoothRefusalState.INVALID
                ? EBoothCheckReason.CONFIG_INVALID
                : EBoothCheckReason.CONFIG_UNVERIFIED
        return refuse(reason, config.state, policy)
    }
    if (!context.opened) {
        return refuse(EBoothCheckReason.OPENED_MISSING, EBoothRefusalState.PENDING, policy)
    }
    const opened = verifyLogged(sdk, policy, context.opened, VOTING_OPENED)
    if (!opened.ok) {
        const reason =
            opened.state === EBoothRefusalState.INVALID
                ? EBoothCheckReason.OPENED_INVALID
                : EBoothCheckReason.OPENED_UNVERIFIED
        return refuse(reason, opened.state, policy)
    }

    const statement = parseConfigStatement(config.text)
    if (!statement) {
        return refuse(EBoothCheckReason.CONFIG_INVALID, EBoothRefusalState.INVALID, policy)
    }
    const milestone = parseMilestoneStatement(opened.text)
    if (!milestone || milestone.kind !== VOTING_OPENED) {
        return refuse(EBoothCheckReason.OPENED_INVALID, EBoothRefusalState.INVALID, policy)
    }
    const failed = (reason: EBoothCheckReason) => refuse(reason, EBoothRefusalState.FAILED, policy)

    if (statement.event !== input.eventId || milestone.event !== input.eventId) {
        return failed(EBoothCheckReason.EVENT_MISMATCH)
    }
    if (statement.election !== input.electionId) {
        return failed(EBoothCheckReason.ELECTION_MISMATCH)
    }
    const leaf = pure.encoding.b64encode(pure.hashes.leafHash(config.entry))
    if (milestone.configs[input.electionId] !== leaf) {
        return failed(EBoothCheckReason.OPENED_CONFIG_MISMATCH)
    }
    if (statement.policy !== policy.id) return failed(EBoothCheckReason.POLICY_MISMATCH)
    if (statement.transparency !== input.mode) {
        return failed(EBoothCheckReason.TRANSPARENCY_MISMATCH)
    }
    const key = electionKeyDigest(pure, input.publicKey)
    if (!key || key !== statement.key) return failed(EBoothCheckReason.KEY_MISMATCH)
    const hashes = context.style_hashes ?? []
    const own = styleDigestHex(pure, input.styleText)
    if (!own || !hashes.includes(own) || stylesDigest(pure, hashes) !== statement.styles) {
        return failed(EBoothCheckReason.STYLES_MISMATCH)
    }
    const listed = statement.releases.some(
        (r) => r.package === VOTING_PORTAL_PACKAGE && r.version === input.release
    )
    if (!listed) return failed(EBoothCheckReason.RELEASE_MISMATCH)
    return {ok: true, policy, config: statement}
}

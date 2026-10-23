// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The check before encrypting (step-integration.md, section 2.6; Vitrine's
// specs/entry.md, section 5.2). The booth needs the election's config
// statement and the event's `voting-opened` milestone, each in the events log
// with a receipt that meets the quorum, and the key, styles, policy and
// release they name must be the ones the booth uses. @vitrine/verify's
// checkBoothConfig does the check; this module pins the policy, feeds it what
// Harvest served and turns its answer into a reason the voter is shown.

import type {ConfigStatement, Policy} from "@vitrine/verify"
import {electionKeyBytes, isAscendingDigestList} from "./digests"
import {policyHex} from "./presentation"
import type {IVitrineSdk} from "./sdk"
import {
    ETransparencyMode,
    IVitrinePresentation,
    IVitrineVoterContext,
    VOTING_PORTAL_PACKAGE,
} from "./types"

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
    /** The device clock, POSIX seconds. */
    now?: number
}

export interface IBoothCheckPassed {
    ok: true
    policy: Policy
    config: ConfigStatement
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

// checkBoothConfig's `config-mismatch` fields.
const MISMATCHES: Record<string, EBoothCheckReason> = {
    event: EBoothCheckReason.EVENT_MISMATCH,
    election: EBoothCheckReason.ELECTION_MISMATCH,
    opened: EBoothCheckReason.OPENED_CONFIG_MISMATCH,
    key: EBoothCheckReason.KEY_MISMATCH,
    styles: EBoothCheckReason.STYLES_MISMATCH,
    policy: EBoothCheckReason.POLICY_MISMATCH,
    release: EBoothCheckReason.RELEASE_MISMATCH,
}

const QUORUM_NOT_MET = "quorum-not-met"

/** The receipt in a statement's receipt chain: the record, or the receipt
 * the log issued. Anything else goes to the check as it is, which refuses it. */
const receiptOf = (sdk: IVitrineSdk, chain: string): string | Uint8Array => {
    try {
        const parsed = sdk.pure.parseChain(chain)
        return parsed.record ?? parsed.issued ?? chain
    } catch {
        return chain
    }
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
    const failed = (reason: EBoothCheckReason) => refuse(reason, EBoothRefusalState.FAILED, policy)

    if (!context.config) {
        return refuse(EBoothCheckReason.CONFIG_MISSING, EBoothRefusalState.PENDING, policy)
    }
    if (!context.opened) {
        return refuse(EBoothCheckReason.OPENED_MISSING, EBoothRefusalState.PENDING, policy)
    }
    const publicKey = electionKeyBytes(sdk.pure, input.publicKey)
    if (!publicKey) return failed(EBoothCheckReason.KEY_MISMATCH)
    const styles = context.style_hashes ?? []
    if (input.styleText === undefined || !isAscendingDigestList(styles)) {
        return failed(EBoothCheckReason.STYLES_MISMATCH)
    }

    let statement: ConfigStatement
    try {
        statement = sdk.pure.checkBoothConfig(policy, {
            config: receiptOf(sdk, context.config.chain),
            opened: receiptOf(sdk, context.opened.chain),
            event: input.eventId,
            election: input.electionId,
            publicKey,
            styles,
            style: sdk.pure.bytes.utf8(input.styleText),
            release: {package: VOTING_PORTAL_PACKAGE, version: input.release},
            now: input.now,
        }).statement
    } catch (e) {
        if (!(e instanceof sdk.pure.VerifyError)) {
            return refuse(EBoothCheckReason.CONFIG_INVALID, EBoothRefusalState.INVALID, policy)
        }
        const params = e.params ?? {}
        if (e.code === "config-mismatch") {
            return failed(MISMATCHES[String(params.field)] ?? EBoothCheckReason.CONFIG_INVALID)
        }
        const opened = params.part === "opened"
        if (params.detail === QUORUM_NOT_MET) {
            const reason = opened
                ? EBoothCheckReason.OPENED_UNVERIFIED
                : EBoothCheckReason.CONFIG_UNVERIFIED
            return refuse(reason, EBoothRefusalState.PENDING, policy)
        }
        const reason = opened ? EBoothCheckReason.OPENED_INVALID : EBoothCheckReason.CONFIG_INVALID
        return refuse(reason, EBoothRefusalState.INVALID, policy)
    }
    if (statement.transparency !== input.mode) {
        return failed(EBoothCheckReason.TRANSPARENCY_MISMATCH)
    }
    return {ok: true, policy, config: statement}
}

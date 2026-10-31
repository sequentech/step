// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Trust policies pinned on this device. A receipt is checked only against a
// policy the user pinned after comparing its ID and six words with the
// official election notice (Vitrine's specs/receipt.md, section 3.1). A draft
// is never pinned: its eight draft words are shown, so the user can tell it
// from the sealed policy. Pins live in localStorage, a convenience of this
// browser only, and are checked again whenever they are read.

import type {Policy, PolicySummary, Reason} from "@vitrine/verify"
import {EReceiptInput, TReceiptInput} from "./input"
import type {IVitrineSdk} from "./sdk"

export const PINNED_POLICIES_KEY = "vitrine-pinned-policies"

/** The log whose profile parameters name the event and its elections. */
const BALLOTS_LOG = "ballots"

/** A valid trust policy, as the screen shows it. */
export interface IPolicyView {
    policy: Policy
    /** The signed policy note. */
    text: string
    /** Its ID, words, event and witnesses. */
    summary: PolicySummary
}

export enum EPolicyRefusal {
    DRAFT = "draft",
    INVALID = "invalid",
}

export type TPolicyExamination =
    | {ok: true; view: IPolicyView}
    | {ok: false; refusal: EPolicyRefusal.DRAFT; draftWords: string[]}
    | {ok: false; refusal: EPolicyRefusal.INVALID; reason: Reason}

const draftOf = (sdk: IVitrineSdk, note: string): Policy | undefined => {
    try {
        const p = sdk.verify.parsePolicy(note, {allowDraft: true, requireSignatures: false})
        return p.draft ? p : undefined
    } catch {
        return undefined
    }
}

/** Whether `text` is a valid, sealed trust policy, and if not, why. */
export const examinePolicy = (sdk: IVitrineSdk, text: string): TPolicyExamination => {
    try {
        const policy = sdk.verify.pinPolicy(text)
        return {ok: true, view: {policy, text, summary: sdk.verify.policySummary(policy)}}
    } catch (e) {
        const draft = draftOf(sdk, text)
        if (draft) {
            return {
                ok: false,
                refusal: EPolicyRefusal.DRAFT,
                draftWords: sdk.pure.policies.draftWords(draft),
            }
        }
        return {
            ok: false,
            refusal: EPolicyRefusal.INVALID,
            reason: e instanceof sdk.verify.VerifyError ? e.toReason() : {code: "policy-invalid"},
        }
    }
}

const readPinned = (): string[] => {
    try {
        const list: unknown = JSON.parse(localStorage.getItem(PINNED_POLICIES_KEY) ?? "[]")
        return Array.isArray(list) ? list.filter((t): t is string => typeof t === "string") : []
    } catch {
        return []
    }
}

const writePinned = (texts: readonly string[]): void => {
    try {
        localStorage.setItem(PINNED_POLICIES_KEY, JSON.stringify(texts))
    } catch {
        // The pin then lasts for this visit only.
    }
}

/** The pinned policies that are still valid, one per policy ID. */
export const pinnedPolicies = (sdk: IVitrineSdk): IPolicyView[] => {
    const views: IPolicyView[] = []
    for (const text of readPinned()) {
        const examined = examinePolicy(sdk, text)
        if (examined.ok && !views.some((v) => v.policy.id === examined.view.policy.id)) {
            views.push(examined.view)
        }
    }
    return views
}

/** Pins a policy the user confirmed. */
export const pinPolicy = (view: IPolicyView): void => {
    const texts = readPinned()
    if (!texts.includes(view.text)) writePinned([...texts, view.text])
}

/** Forgets the pinned policy with this ID. */
export const unpinPolicy = (sdk: IVitrineSdk, policyId: string): void => {
    writePinned(
        readPinned().filter((text) => {
            const examined = examinePolicy(sdk, text)
            return !examined.ok || examined.view.policy.id !== policyId
        })
    )
}

export enum EPinMatch {
    FOUND = "found",
    NONE = "none",
    /** A reference whose locator matches more than one pinned policy. */
    AMBIGUOUS = "ambiguous",
}

export type TPinMatch =
    | {match: EPinMatch.FOUND; view: IPolicyView}
    | {match: EPinMatch.NONE}
    | {match: EPinMatch.AMBIGUOUS}

/** The pinned policy a receipt is checked against: the one whose ID its chain
 * names, or the one its reference locates (Vitrine's specs/receipt.md,
 * section 4). */
export const findPinned = (
    sdk: IVitrineSdk,
    receipt: TReceiptInput,
    pinned: readonly IPolicyView[]
): TPinMatch => {
    let id: string
    if (receipt.kind === EReceiptInput.CHAIN) {
        id = receipt.policyId
    } else {
        try {
            id = sdk.verify.resolveReference(
                receipt.reference,
                pinned.map((v) => v.policy)
            ).id
        } catch (e) {
            return e instanceof sdk.verify.VerifyError && e.code === "reference-ambiguous"
                ? {match: EPinMatch.AMBIGUOUS}
                : {match: EPinMatch.NONE}
        }
    }
    const view = pinned.find((v) => v.policy.id === id)
    return view ? {match: EPinMatch.FOUND, view} : {match: EPinMatch.NONE}
}

/** True when the receipt is for this policy, so that it may be pinned for it. */
export const policyFitsReceipt = (
    sdk: IVitrineSdk,
    receipt: TReceiptInput,
    view: IPolicyView
): boolean =>
    receipt.kind === EReceiptInput.CHAIN
        ? view.policy.id === receipt.policyId
        : sdk.verify.referenceMatches(receipt.reference, view.policy)

/** The event and the elections whose ballots the policy's ballots log holds,
 * from its profile parameters (`param ballots event-id` and `election-ids`). */
export const policyElections = (policy: Policy): {eventId?: string; electionIds: string[]} => {
    const params = policy.logs.get(BALLOTS_LOG)?.params
    const eventId = params?.get("event-id")?.[0]
    return {...(eventId ? {eventId} : {}), electionIds: params?.get("election-ids") ?? []}
}

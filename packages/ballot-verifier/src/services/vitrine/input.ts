// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What a voter or auditor gives the receipt screen: a receipt chain, as its
// file or its text (Vitrine's specs/receipt.md, section 3); the link the
// voting portal's "Check independently" opens, which carries the chain, the
// entry the portal built and where the policy is,
// `#vitrine:chain?c=<chain>&e=<entry>&u=<policy URL>` or `&policy=<note>`
// (base64url); or a receipt reference, from a QR code or a printout,
// `#vitrine:receipt?p=&l=&i=&e=` (section 4). All of it is data: the screen
// shows it as text and checks it, and fetches a policy only from an https URL.

import type {ReceiptReference} from "@vitrine/verify"
import {isFetchableUrl} from "./http"
import type {IVitrineSdk} from "./sdk"

export const CHAIN_LINK_PREFIX = "#vitrine:chain?"
export const REFERENCE_PREFIX = "#vitrine:receipt?"
/** What receipt chain files end in, and their media type. */
export const CHAIN_FILE_EXTENSION = ".vitrine-receipt"
export const CHAIN_MEDIA_TYPE = "application/vnd.vitrine.receipt-chain"
const CHAIN_HEADER = "vitrine/receipt-chain/v1"
const POLICY_HEADER = "vitrine/policy/v1"

/** A receipt chain is at most 4 MiB; its link holds it in base64url. */
export const MAX_RECEIPT_TEXT = 6 << 20

export enum EReceiptInput {
    CHAIN = "chain",
    REFERENCE = "reference",
}

export enum EInputProblem {
    EMPTY = "empty",
    TOO_LARGE = "too-large",
    BAD_LINK = "bad-link",
    BAD_REFERENCE = "bad-reference",
    BAD_CHAIN = "bad-chain",
    NOT_HTTPS = "not-https",
    /** A trust policy where a receipt was expected, or the other way round. */
    WRONG_BOX = "wrong-box",
    UNRECOGNISED = "unrecognised",
    /** The file couldn't be read. */
    UNREADABLE = "unreadable",
}

/** Where a trust policy comes from: its URL, or its text. */
export type TPolicySource = {url: string} | {text: string}

export interface IChainInput {
    kind: EReceiptInput.CHAIN
    /** The receipt chain document. */
    chain: string
    /** The policy ID the chain names. */
    policyId: string
    log: string
    /** The entry the voting portal built, from its link. */
    entry?: Uint8Array
    /** Where the portal's link says the policy is. */
    policySource?: TPolicySource
}

export interface IReferenceInput {
    kind: EReceiptInput.REFERENCE
    reference: ReceiptReference
}

export type TReceiptInput = IChainInput | IReferenceInput

export type TParsed<T> = {ok: true; value: T} | {ok: false; problem: EInputProblem}

const problem = <T>(p: EInputProblem): TParsed<T> => ({ok: false, problem: p})

/** True when a URL's fragment holds a receipt link or reference. */
export const hasReceiptFragment = (url: string): boolean =>
    url.includes(CHAIN_LINK_PREFIX) || url.includes(REFERENCE_PREFIX)

/** The first of `candidates` that parses as a receipt chain. */
const chainInput = (
    sdk: IVitrineSdk,
    candidates: readonly string[],
    extra: Pick<IChainInput, "entry" | "policySource"> = {}
): TParsed<IChainInput> => {
    for (const chain of candidates) {
        try {
            const c = sdk.pure.parseChain(chain)
            return {
                ok: true,
                value: {
                    kind: EReceiptInput.CHAIN,
                    chain,
                    policyId: c.policyId,
                    log: c.log,
                    ...extra,
                },
            }
        } catch {
            // Try the next form, or report the chain as damaged.
        }
    }
    return problem(EInputProblem.BAD_CHAIN)
}

const parseChainLink = (sdk: IVitrineSdk, text: string): TParsed<IChainInput> => {
    const {b64urlDecode} = sdk.pure.encoding
    const {fromUtf8} = sdk.pure.bytes
    const params = new URLSearchParams(
        text.slice(text.indexOf(CHAIN_LINK_PREFIX) + CHAIN_LINK_PREFIX.length)
    )
    const c = params.get("c")
    if (!c) return problem(EInputProblem.BAD_LINK)
    let chain: string
    const extra: Pick<IChainInput, "entry" | "policySource"> = {}
    try {
        chain = fromUtf8(b64urlDecode(c))
        const e = params.get("e")
        if (e !== null) extra.entry = b64urlDecode(e)
        const u = params.get("u")
        const p = params.get("policy")
        if (u !== null) {
            if (!isFetchableUrl(u)) return problem(EInputProblem.NOT_HTTPS)
            extra.policySource = {url: u}
        } else if (p !== null) {
            extra.policySource = {text: fromUtf8(b64urlDecode(p))}
        }
    } catch {
        return problem(EInputProblem.BAD_LINK)
    }
    if (extra.entry?.length === 0) return problem(EInputProblem.BAD_LINK)
    const parsed = chainInput(sdk, [chain], extra)
    return parsed.ok ? parsed : problem(EInputProblem.BAD_LINK)
}

/** Reads what was given as a receipt. `base` is the page's own URL, for a
 * reference pasted without its verifier part; its query (the language) is not
 * part of the verifier's address. */
export const parseReceiptInput = (
    sdk: IVitrineSdk,
    raw: string,
    base?: string
): TParsed<TReceiptInput> => {
    if (raw.length > MAX_RECEIPT_TEXT) return problem(EInputProblem.TOO_LARGE)
    const text = raw.replace(/\r\n/g, "\n").trim()
    if (text === "") return problem(EInputProblem.EMPTY)
    if (text.includes(CHAIN_LINK_PREFIX)) return parseChainLink(sdk, text)
    if (text.includes(REFERENCE_PREFIX)) {
        const url = text.startsWith("#") && base ? `${base.split(/[?#]/)[0]}${text}` : text
        try {
            return {
                ok: true,
                value: {
                    kind: EReceiptInput.REFERENCE,
                    reference: sdk.verify.parseReceiptReference(url),
                },
            }
        } catch {
            return problem(EInputProblem.BAD_REFERENCE)
        }
    }
    if (text.startsWith(CHAIN_HEADER)) {
        // As given, from a file, or as it was before a paste changed its line
        // ends or its final newlines: a chain ends with its last block's
        // newline, after contents that usually end with one too.
        const lf = raw.replace(/\r\n/g, "\n").replace(/^\s+/, "")
        return chainInput(sdk, [raw, lf, `${text}\n`, `${text}\n\n`])
    }
    if (text.startsWith(POLICY_HEADER)) return problem(EInputProblem.WRONG_BOX)
    return problem(EInputProblem.UNRECOGNISED)
}

/** Reads what was given as a trust policy: its text, or its https URL. */
export const parsePolicyInput = (raw: string): TParsed<TPolicySource> => {
    if (raw.length > MAX_RECEIPT_TEXT) return problem(EInputProblem.TOO_LARGE)
    const text = raw.replace(/\r\n/g, "\n").trim()
    if (text === "") return problem(EInputProblem.EMPTY)
    if (text.startsWith(POLICY_HEADER)) return {ok: true, value: {text: `${text}\n`}}
    if (text.startsWith(CHAIN_HEADER) || hasReceiptFragment(text)) {
        return problem(EInputProblem.WRONG_BOX)
    }
    if (/^https?:\/\/\S+$/i.test(text)) {
        return isFetchableUrl(text)
            ? {ok: true, value: {url: text}}
            : problem(EInputProblem.NOT_HTTPS)
    }
    return problem(EInputProblem.UNRECOGNISED)
}

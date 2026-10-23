// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Check independently" (step-integration.md, section 2.6): the deployment's
// ballot verifier, with the receipt chain in the link's fragment, which
// browsers never send to a server. The link has the form of Vitrine's
// verifier chain links, `#vitrine:chain?c=<chain>&e=<entry>&u=<policy URL>`
// (base64url), so Vitrine's own verifier opens it too. A private event's log
// doesn't serve its policy to the public, so its link carries the policy note
// instead, as `policy=`, for the voter to pin after comparing its words with
// the election notice.

import type {Policy} from "@vitrine/verify"
import {joinUrl} from "./http"
import type {TVitrinePure} from "./sdk"
import {ETransparencyMode, EVitrineLog} from "./types"

export const CHAIN_LINK_PREFIX = "#vitrine:chain?"
/** The ballot verifier's receipt screen. */
export const VERIFIER_RECEIPT_PATH = "receipt"

export interface IHandOff {
    /** The deployment's ballot verifier. */
    verifierUrl: string
    chain: string
    /** The voter's entry, for the verifier's expectation. */
    entry: Uint8Array
    mode: ETransparencyMode
    policy: Policy
    /** The pinned policy's signed note. */
    policyText?: string
}

/** The link that opens the receipt in the ballot verifier. */
export const checkIndependentlyUrl = (pure: TVitrinePure, h: IHandOff): string => {
    const {b64urlEncode} = pure.encoding
    const {utf8} = pure.bytes
    const at = h.verifierUrl.indexOf("#")
    const base = joinUrl(at < 0 ? h.verifierUrl : h.verifierUrl.slice(0, at), VERIFIER_RECEIPT_PATH)
    const params = [`c=${b64urlEncode(utf8(h.chain))}`, `e=${b64urlEncode(h.entry)}`]
    const log = h.policy.logs.get(EVitrineLog.BALLOTS)?.url
    if (h.mode === ETransparencyMode.PUBLIC && log) {
        params.push(`u=${encodeURIComponent(joinUrl(log, `policy/${h.policy.id}`))}`)
    } else if (h.policyText) {
        params.push(`policy=${b64urlEncode(utf8(h.policyText))}`)
    }
    return `${base}${CHAIN_LINK_PREFIX}${params.join("&")}`
}

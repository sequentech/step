// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as pure from "@vitrine/verify/pure"
import {ballotEntry, chainText, KEYS, LOG_URLS, scenario} from "../../fixtures/vitrine"
import {checkIndependentlyUrl, receiptReferenceUrl} from "./handoff"
import {ETransparencyMode} from "./types"

const VERIFIER = "https://verifier.example/app/#home"

const setup = (mode: ETransparencyMode.PUBLIC | ETransparencyMode.PRIVATE) => {
    const s = scenario({mode})
    const entry = ballotEntry()
    const index = s.ballots.add(entry)
    s.ballots.add(ballotEntry("ab".repeat(32)))
    const receipt = s.ballots.receipt(index, 2, KEYS.witnesses.slice(0, 2))
    const chain = chainText(s.policyId, "ballots", {issued: pure.bytes.utf8(receipt)})
    return {s, entry, index, receipt, chain, policy: pure.parsePolicy(s.policy)}
}

const fragment = (url: string, prefix: string) => {
    const at = url.indexOf(prefix)
    expect(at).toBeGreaterThan(0)
    return {base: url.slice(0, at), params: new URLSearchParams(url.slice(at + prefix.length))}
}

describe("checkIndependentlyUrl", () => {
    it("carries a public event's chain, entry and policy URL in the fragment", () => {
        const {s, entry, chain, policy} = setup(ETransparencyMode.PUBLIC)
        const url = checkIndependentlyUrl(pure, {
            verifierUrl: VERIFIER,
            chain,
            entry,
            mode: ETransparencyMode.PUBLIC,
            policy,
            policyText: s.policy,
        })
        const {base, params} = fragment(url, "#vitrine:chain?")
        expect(base).toBe("https://verifier.example/app/receipt")
        expect(pure.bytes.fromUtf8(pure.encoding.b64urlDecode(params.get("c") ?? ""))).toBe(chain)
        expect(pure.encoding.b64urlDecode(params.get("e") ?? "")).toEqual(entry)
        expect(params.get("u")).toBe(`${LOG_URLS.ballots}policy/${s.policyId}`)
        expect(params.has("policy")).toBe(false)
    })

    it("carries a private event's policy note, whose log serves no one", () => {
        const {s, entry, chain, policy} = setup(ETransparencyMode.PRIVATE)
        const url = checkIndependentlyUrl(pure, {
            verifierUrl: VERIFIER,
            chain,
            entry,
            mode: ETransparencyMode.PRIVATE,
            policy,
            policyText: s.policy,
        })
        const {params} = fragment(url, "#vitrine:chain?")
        expect(params.has("u")).toBe(false)
        expect(pure.bytes.fromUtf8(pure.encoding.b64urlDecode(params.get("policy") ?? ""))).toBe(
            s.policy
        )
    })
})

describe("receiptReferenceUrl", () => {
    it("is the receipt's reference on the verifier's receipt screen", () => {
        const {s, entry, index, receipt, policy} = setup(ETransparencyMode.PUBLIC)
        const url = receiptReferenceUrl(pure, VERIFIER, policy, receipt, entry)
        const reference = pure.parseReceiptReference(url ?? "")
        expect(url?.startsWith("https://verifier.example/app/receipt/#vitrine:receipt?")).toBe(true)
        expect(reference).toMatchObject({
            policyPrefix: s.policyId.slice("sha256:".length, "sha256:".length + 32),
            log: "ballots",
            index: BigInt(index),
        })
        expect(reference.entry).toEqual(entry)
    })

    it("is undefined for anything but a receipt", () => {
        const {entry, policy} = setup(ETransparencyMode.PUBLIC)
        expect(receiptReferenceUrl(pure, VERIFIER, policy, "<b>not a receipt</b>", entry)).toBe(
            undefined
        )
    })
})

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {chainLink, policyUrlOf, receiptScenario, referenceOf} from "../../fixtures/vitrine"
import {ETransparencyMode} from "./presentation"
import {
    EInputProblem,
    EReceiptInput,
    hasReceiptFragment,
    IChainInput,
    MAX_RECEIPT_TEXT,
    parsePolicyInput,
    parseReceiptInput,
    TParsed,
    TReceiptInput,
} from "./input"
import type {IVitrineSdk} from "./sdk"

const sdk: IVitrineSdk = {verify, pure}
const s = receiptScenario()
const refused = (problem: EInputProblem) => ({ok: false, problem})

beforeAll(async () => {
    await verify.init()
})

const chainOf = (parsed: TParsed<TReceiptInput>): IChainInput => {
    if (!parsed.ok || parsed.value.kind !== EReceiptInput.CHAIN) {
        throw new Error(`not a chain: ${JSON.stringify(parsed)}`)
    }
    return parsed.value
}

describe("parseReceiptInput", () => {
    it("reads a receipt chain file", () => {
        expect(parseReceiptInput(sdk, s.chain)).toEqual({
            ok: true,
            value: {
                kind: EReceiptInput.CHAIN,
                chain: s.chain,
                policyId: s.policyId,
                log: "ballots",
            },
        })
    })

    it("reads a chain whose line ends or final newline a paste changed", () => {
        const pasted = `\n  ${s.chain.replace(/\n/g, "\r\n").trimEnd()}  \n`
        expect(chainOf(parseReceiptInput(sdk, pasted)).chain).toBe(s.chain)
    })

    it("reads the voting portal's link: the chain, the entry and where the policy is", () => {
        const link = `https://verifier.example/receipt${chainLink(s.chain, s.entry, {url: policyUrlOf(s)})}`
        const chain = chainOf(parseReceiptInput(sdk, link))
        expect(chain.chain).toBe(s.chain)
        expect(chain.policyId).toBe(s.policyId)
        expect(chain.entry).toEqual(s.entry)
        expect(chain.policySource).toEqual({url: policyUrlOf(s)})
    })

    it("reads a private event's link, which carries the policy itself", () => {
        const p = receiptScenario({visibility: ETransparencyMode.PRIVATE})
        const chain = chainOf(parseReceiptInput(sdk, chainLink(p.chain, p.entry, {text: p.policy})))
        expect(chain.policySource).toEqual({text: p.policy})
    })

    it.each([
        "http://logs.acme.example/e/ballots/policy/x",
        "javascript:alert(1)",
        "https://user:secret@logs.acme.example/policy",
    ])("refuses a link whose policy is at %s", (url) => {
        expect(parseReceiptInput(sdk, chainLink(s.chain, s.entry, {url}))).toEqual(
            refused(EInputProblem.NOT_HTTPS)
        )
    })

    it.each([
        ["without a chain", "#vitrine:chain?e=AAAA"],
        ["whose chain isn't base64url", "#vitrine:chain?c=***"],
        ["whose chain is damaged", chainLink("vitrine/receipt-chain/v1\nnonsense\n")],
        ["with an empty entry", `${chainLink(s.chain)}&e=`],
    ])("refuses a link %s", (_what, link) => {
        expect(parseReceiptInput(sdk, link)).toEqual(refused(EInputProblem.BAD_LINK))
    })

    it("reads a receipt reference, with or without the verifier's address", () => {
        const ref = referenceOf(s)
        const full = parseReceiptInput(sdk, ref)
        if (!full.ok || full.value.kind !== EReceiptInput.REFERENCE) throw new Error("no reference")
        expect(full.value.reference.index).toBe(BigInt(s.index))
        expect(full.value.reference.entry).toEqual(s.entry)
        const fragment = ref.slice(ref.indexOf("#"))
        expect(parseReceiptInput(sdk, fragment, "https://verifier.example/receipt/#old")).toEqual(
            full
        )
    })

    it("refuses a damaged reference", () => {
        expect(parseReceiptInput(sdk, "https://verifier.example/#vitrine:receipt?p=zz")).toEqual(
            refused(EInputProblem.BAD_REFERENCE)
        )
    })

    it("tells a trust policy from a receipt", () => {
        expect(parseReceiptInput(sdk, s.policy)).toEqual(refused(EInputProblem.WRONG_BOX))
    })

    it.each([
        ["markup", "<img src=x onerror=alert(1)>", EInputProblem.UNRECOGNISED],
        ["nothing", "  \n", EInputProblem.EMPTY],
        ["too much", "x".repeat(MAX_RECEIPT_TEXT + 1), EInputProblem.TOO_LARGE],
        ["a damaged chain", "vitrine/receipt-chain/v1\ngarbage", EInputProblem.BAD_CHAIN],
    ])("refuses %s", (_what, raw, problem) => {
        expect(parseReceiptInput(sdk, raw)).toEqual(refused(problem))
    })
})

describe("parsePolicyInput", () => {
    it("takes a policy's text, however it was pasted", () => {
        expect(parsePolicyInput(`\r\n${s.policy.replace(/\n/g, "\r\n")}  `)).toEqual({
            ok: true,
            value: {text: s.policy},
        })
    })

    it("takes a policy's https address", () => {
        expect(parsePolicyInput(` ${policyUrlOf(s)} `)).toEqual({
            ok: true,
            value: {url: policyUrlOf(s)},
        })
    })

    it.each([
        ["plain http", "http://logs.acme.example/policy", EInputProblem.NOT_HTTPS],
        ["credentials", "https://user:pw@logs.acme.example/policy", EInputProblem.NOT_HTTPS],
        ["a receipt chain", s.chain, EInputProblem.WRONG_BOX],
        ["a receipt link", chainLink(s.chain), EInputProblem.WRONG_BOX],
        ["a script address", "javascript:alert(1)", EInputProblem.UNRECOGNISED],
        ["nothing", "", EInputProblem.EMPTY],
    ])("refuses %s", (_what, raw, problem) => {
        expect(parsePolicyInput(raw)).toEqual(refused(problem))
    })
})

describe("hasReceiptFragment", () => {
    it("finds a receipt link or reference in a URL", () => {
        expect(hasReceiptFragment(`https://v.example/receipt${chainLink(s.chain)}`)).toBe(true)
        expect(hasReceiptFragment(referenceOf(s))).toBe(true)
        expect(hasReceiptFragment("https://v.example/receipt#section")).toBe(false)
    })
})

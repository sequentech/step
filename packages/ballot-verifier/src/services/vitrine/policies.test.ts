// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import * as verify from "@vitrine/verify"
import * as pure from "@vitrine/verify/pure"
import {
    draftPolicyText,
    ELECTION_ID,
    EVENT_ID,
    OTHER_ELECTION_ID,
    policyIdOf,
    policyText,
    receiptScenario,
    referenceOf,
} from "../../fixtures/vitrine"
import {EReceiptInput, TReceiptInput} from "./input"
import {
    EPinMatch,
    EPolicyRefusal,
    examinePolicy,
    findPinned,
    IPolicyView,
    pinnedPolicies,
    PINNED_POLICIES_KEY,
    pinPolicy,
    policyElections,
    policyFitsReceipt,
    unpinPolicy,
} from "./policies"
import type {IVitrineSdk} from "./sdk"

const sdk: IVitrineSdk = {verify, pure}

beforeAll(async () => {
    await verify.init()
})

beforeEach(() => {
    localStorage.clear()
})

const viewOf = (text: string): IPolicyView => {
    const examined = examinePolicy(sdk, text)
    if (!examined.ok) throw new Error(`not a valid policy: ${JSON.stringify(examined)}`)
    return examined.view
}

const s = receiptScenario()
const chainInput: TReceiptInput = {
    kind: EReceiptInput.CHAIN,
    chain: s.chain,
    policyId: s.policyId,
    log: "ballots",
}
const referenceInput: TReceiptInput = {
    kind: EReceiptInput.REFERENCE,
    reference: verify.parseReceiptReference(referenceOf(s)),
}

describe("examinePolicy", () => {
    it("accepts a sealed policy, with what the user compares", () => {
        const view = viewOf(s.policy)
        expect(view.text).toBe(s.policy)
        expect(view.policy.id).toBe(s.policyId)
        expect(view.summary.id).toBe(policyIdOf(s.policy))
        expect(view.idDisplay).toBe(verify.policyIdDisplay(view.policy.id))
        expect(view.idDisplay.split("-")).toHaveLength(13)
        expect(view.shortId).toBe(view.idDisplay.split("-").slice(0, 4).join("-"))
        expect(view.summary.event).toBe("acme-2026")
        expect(view.summary.witnesses).toContainEqual(
            expect.objectContaining({name: "w2", party: "observers"})
        )
    })

    it("refuses a draft, with its draft hash in the groups of a policy ID", () => {
        const draft = draftPolicyText()
        const examined = examinePolicy(sdk, draft)
        const parsed = pure.parsePolicy(draft, {allowDraft: true, requireSignatures: false})
        const draftId = verify.policyIdDisplay(parsed.id)
        expect(examined).toEqual({ok: false, refusal: EPolicyRefusal.DRAFT, draftId})
        expect(draftId.split("-")).toHaveLength(13)
        // It isn't the sealed policy's ID, so the two can't be mistaken.
        expect(draftId).not.toBe(verify.policyIdDisplay(s.policyId))
    })

    it.each([
        ["a policy changed after it was signed", s.policy.replace("mmd 60", "mmd 61")],
        ["markup", "<b>vitrine/policy/v1</b>"],
        ["nothing", ""],
    ])("refuses %s, with the verifier's reason", (_what, text) => {
        const examined = examinePolicy(sdk, text)
        expect(examined).toMatchObject({ok: false, refusal: EPolicyRefusal.INVALID})
        if (examined.ok || examined.refusal !== EPolicyRefusal.INVALID) return
        expect(typeof examined.reason.code).toBe("string")
    })
})

describe("pinned policies", () => {
    it("keeps the policies pinned in this browser, once each", () => {
        const view = viewOf(s.policy)
        pinPolicy(view)
        pinPolicy(view)
        expect(JSON.parse(localStorage.getItem(PINNED_POLICIES_KEY) ?? "null")).toEqual([s.policy])
        expect(pinnedPolicies(sdk).map((v) => v.policy.id)).toEqual([s.policyId])
    })

    it("ignores what isn't a valid sealed policy when it reads them", () => {
        localStorage.setItem(
            PINNED_POLICIES_KEY,
            JSON.stringify([draftPolicyText(), 42, s.policy, "garbage", s.policy])
        )
        expect(pinnedPolicies(sdk).map((v) => v.text)).toEqual([s.policy])
        localStorage.setItem(PINNED_POLICIES_KEY, "{not json")
        expect(pinnedPolicies(sdk)).toEqual([])
        localStorage.setItem(PINNED_POLICIES_KEY, JSON.stringify({policies: [s.policy]}))
        expect(pinnedPolicies(sdk)).toEqual([])
    })

    it("forgets a policy by its ID", () => {
        const other = viewOf(policyText({name: "other-2026"}))
        pinPolicy(viewOf(s.policy))
        pinPolicy(other)
        unpinPolicy(sdk, s.policyId)
        expect(pinnedPolicies(sdk).map((v) => v.policy.id)).toEqual([other.policy.id])
    })
})

describe("findPinned", () => {
    const view = viewOf(s.policy)
    const other = viewOf(policyText({name: "other-2026"}))

    it("finds the pinned policy a chain names", () => {
        expect(findPinned(sdk, chainInput, [other, view])).toEqual({match: EPinMatch.FOUND, view})
        expect(findPinned(sdk, chainInput, [other])).toEqual({match: EPinMatch.NONE})
        expect(findPinned(sdk, chainInput, [])).toEqual({match: EPinMatch.NONE})
    })

    it("finds the pinned policy a reference locates", () => {
        expect(findPinned(sdk, referenceInput, [other, view])).toEqual({
            match: EPinMatch.FOUND,
            view,
        })
        expect(findPinned(sdk, referenceInput, [other])).toEqual({match: EPinMatch.NONE})
    })

    it("refuses a reference that locates more than one pinned policy", () => {
        const twoMatches: IVitrineSdk = {
            pure,
            verify: {
                ...verify,
                resolveReference: () => {
                    throw new verify.VerifyError("reference-ambiguous")
                },
            },
        }
        expect(findPinned(twoMatches, referenceInput, [view, other])).toEqual({
            match: EPinMatch.AMBIGUOUS,
        })
    })
})

describe("policyFitsReceipt", () => {
    it("lets a policy be pinned only for its own receipts", () => {
        const view = viewOf(s.policy)
        const other = viewOf(policyText({name: "other-2026"}))
        expect(policyFitsReceipt(sdk, chainInput, view)).toBe(true)
        expect(policyFitsReceipt(sdk, chainInput, other)).toBe(false)
        expect(policyFitsReceipt(sdk, referenceInput, view)).toBe(true)
        expect(policyFitsReceipt(sdk, referenceInput, other)).toBe(false)
    })
})

describe("policyElections", () => {
    it("reads the event and the elections from the ballots log's parameters", () => {
        expect(policyElections(viewOf(s.policy).policy)).toEqual({
            eventId: EVENT_ID,
            electionIds: [ELECTION_ID, OTHER_ELECTION_ID],
        })
        expect(policyElections(viewOf(policyText({noParams: true})).policy)).toEqual({
            electionIds: [],
        })
    })
})

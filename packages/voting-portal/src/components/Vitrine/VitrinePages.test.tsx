/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The pages around Vitrine's receipt, for each state the verifier's result
// can put them in (the design boards Main, Revisit and Alert). The receipt
// element itself is the kit's and is covered by the confirmation screen's tests.

import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import type {ChainResult, TierName, TierResult, TierStatus} from "@vitrine/verify"
import {EReceiptNotice, IReceiptView} from "../../services/vitrine/castReceipt"
import {EReceiptKeeping} from "../../services/vitrine/storage"
import {ETransparencyMode} from "../../services/vitrine/types"
import type {IConfirmationVitrine} from "../../store/castVotes/confirmationScreenDataSlice"
import {CastReceipt} from "./CastReceipt"
import {KeptReceipts} from "./KeptReceipts"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        ...jest.requireActual(
            "../../../../ui-essentials/src/components/ConfirmationActions/ConfirmationActions"
        ),
        VisuallyHidden: jest.requireActual(
            "../../../../ui-essentials/src/components/VisuallyHidden/VisuallyHidden"
        ).default,
        QRCode: () => null,
        theme: jest.requireActual("../../../../ui-essentials/src/services/theme").default,
    }),
    {virtual: true}
)
jest.mock("../../providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual<typeof React>("react").createContext({
        isKiosk: () => false,
        userId: "voter-1",
    }),
}))
jest.mock("../../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {BALLOT_VERIFIER_URL: "https://verifier.example/"},
    }),
}))
jest.mock("./VitrineElements", () => ({
    VitrineReceiptView: () => <div className="kit-receipt" />,
    VitrineStatusView: () => <div className="kit-status" />,
}))

const mockCast = jest.fn()
jest.mock("../../hooks/useCastReceipt", () => ({
    useCastReceipt: () => mockCast(),
    useKeptReceipts: () => mockKept(),
}))
const mockKept = jest.fn()

const tier = (name: TierName, status: TierStatus, extra: Partial<TierResult> = {}): TierResult => ({
    tier: name,
    status,
    reasons: [],
    ...extra,
})

const resultOf = (status: ChainResult["status"], tiers: TierResult[]): ChainResult =>
    ({
        ok: status !== "failed" && status !== "invalid",
        status,
        tiers,
        facts: {included: "done", unchanged: "done", sameForEveryone: "pending"},
        reasons: [],
        steps: [],
        where: "page",
        entry: {log: "ballots", leafHash: "ab", index: 18204331, promised: false},
        words: ["amber", "falcon", "river", "seven"],
        wordsLang: "en",
        now: 0,
        clockWarning: false,
        profileMetadataResolved: true,
        policy: {id: "x", words: [], lang: "en", event: "e", witnesses: [], hubs: []},
    }) as unknown as ChainResult

const PENDING = resultOf("pending", [
    tier("record", "verified"),
    tier("witnesses", "pending"),
    tier("hubs", "pending"),
    tier("browser", "pending"),
])
const HOLDS = resultOf("pending", [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "verified", {
        hubs: [
            {hub: "a", status: "verified", reasons: []},
            {hub: "b", status: "verified", reasons: []},
        ],
    }),
    tier("browser", "verified"),
])
const SPLIT = resultOf("failed", [
    tier("record", "verified"),
    tier("witnesses", "failed", {
        reasons: [{code: "split-view", objects: ["issued", "record"]}],
    }),
])

const view = (v: Partial<IReceiptView>): IReceiptView => ({notice: EReceiptNotice.NONE, ...v})

const answer = (v: IReceiptView | undefined, kept = true) =>
    mockCast.mockReturnValue({view: v, keeping: EReceiptKeeping.KEEP, kept, setKeeping})

const setKeeping = jest.fn()

const publicEvent: IConfirmationVitrine = {
    cast: null,
    transparency: {mode: ETransparencyMode.PUBLIC},
}
const privateEvent: IConfirmationVitrine = {
    cast: null,
    transparency: {mode: ETransparencyMode.PRIVATE},
}

const renderPage = (vitrine: IConfirmationVitrine = publicEvent) =>
    render(
        <CastReceipt
            eventId="e"
            electionId="el"
            ballotId="c0ffee00c0ffee00"
            vitrine={vitrine}
            progress={<div className="progress" />}
            renderActions={(receipt) => (
                <button
                    className="actions"
                    data-save={receipt.save ? "shown" : "hidden"}
                    disabled={!receipt.save?.onClick}
                >
                    actions
                </button>
            )}
        />
    )

beforeEach(() => {
    jest.clearAllMocks()
})

describe("the page around the receipt", () => {
    it("says the ballot is cast while the witnesses haven't confirmed it", () => {
        answer(view({chain: "chain", result: PENDING}))
        const {container} = renderPage()
        expect(container.querySelector(".vitrine-confirmation-title")?.textContent).toBe(
            "vitrine.confirmation.cast"
        )
        expect(container.querySelector(".vitrine-confirmation")?.getAttribute("data-phase")).toBe(
            "cast"
        )
    })

    it("says the ballot is in the public record once the witnesses confirm it", () => {
        answer(view({chain: "chain", result: HOLDS}))
        const {container} = renderPage()
        expect(container.querySelector(".vitrine-confirmation-title")?.textContent).toBe(
            "vitrine.confirmation.recordedPublic"
        )
        expect(container.querySelector(".vitrine-confirmation-status")?.textContent).toBe(
            "vitrine.confirmation.recordedPublic"
        )
    })

    it("calls a private event's record the election record", () => {
        answer(view({chain: "chain", result: HOLDS}))
        const {container} = renderPage(privateEvent)
        expect(container.querySelector(".vitrine-confirmation-title")?.textContent).toBe(
            "vitrine.confirmation.recordedPrivate"
        )
    })

    it("offers to save the receipt only once it has a chain", () => {
        answer(view({notice: EReceiptNotice.WAITING}))
        const waiting = renderPage()
        expect(waiting.container.querySelector<HTMLButtonElement>(".actions")?.disabled).toBe(true)
        waiting.unmount()

        answer(view({chain: "chain", result: PENDING}))
        const ready = renderPage()
        expect(ready.container.querySelector<HTMLButtonElement>(".actions")?.disabled).toBe(false)
    })

    it("lets the voter stop keeping receipts on this device", () => {
        answer(view({chain: "chain", result: PENDING}))
        const {container} = renderPage()
        expect(container.querySelector(".vitrine-receipt-storage-text")?.textContent).toBe(
            "vitrine.receipt.stored"
        )
        fireEvent.click(container.querySelector(".vitrine-receipt-keeping-button")!)
        expect(setKeeping).toHaveBeenCalledWith(EReceiptKeeping.FORGET)
    })
})

describe("the page when the ballot can't be confirmed", () => {
    it("lists the receipt's entry and the proofs that contradict each other", () => {
        answer(view({chain: "chain", result: SPLIT}))
        const {container} = renderPage()
        const failure = container.querySelector<HTMLElement>(".vitrine-failure")!
        expect(failure.querySelector(".vitrine-failure-title")?.textContent).toBe(
            "vitrine.failure.titlePublic"
        )
        expect(failure.querySelector(".vitrine-failure-explanation")?.textContent).toBe(
            "vitrine.failure.contradiction"
        )
        expect(failure.querySelector(".vitrine-failure-evidence-count")?.textContent).toContain(
            '"count":2'
        )
        expect(failure.querySelector(".vitrine-failure-receipt")?.textContent).toContain(
            "18,204,331"
        )
        const problem = failure.querySelector<HTMLElement>(".vitrine-failure-problem")!
        expect(problem.getAttribute("data-tier")).toBe("witnesses")
        expect(problem.querySelector(".vitrine-failure-row-title")?.textContent).toBe(
            "Witnesses signed a record that contradicts yours."
        )
        expect(problem.querySelector(".vitrine-failure-row-reason")?.textContent).toContain(
            "issued, record"
        )
    })

    it("names the election record for a private event, and keeps the voter able to finish", () => {
        answer(view({chain: "chain", result: SPLIT}))
        const {container} = renderPage(privateEvent)
        expect(container.querySelector(".vitrine-failure-title")?.textContent).toBe(
            "vitrine.failure.titlePrivate"
        )
        expect(container.querySelector(".actions")).not.toBeNull()
    })

    it("offers the receipt file on the page itself, not twice, and shows no steps", () => {
        answer(view({chain: "chain", result: SPLIT}))
        const {container} = renderPage()
        expect(container.querySelector(".actions")?.getAttribute("data-save")).toBe("hidden")
        expect(container.querySelector(".vitrine-failure-save-button")).not.toBeNull()
        expect(container.querySelector(".progress")).toBeNull()
    })

    it("tells each reason once, whichever tiers share it", () => {
        const reason = {code: "split-view", objects: ["issued", "record"]}
        answer(
            view({
                chain: "chain",
                result: resultOf("failed", [
                    tier("record", "failed", {reasons: [reason]}),
                    tier("witnesses", "failed", {reasons: [reason]}),
                ]),
            })
        )
        const {container} = renderPage()
        expect(container.querySelectorAll(".vitrine-failure-problem")).toHaveLength(2)
        expect(container.querySelectorAll(".vitrine-failure-row-reason")).toHaveLength(1)
    })

    it("shows no evidence when no receipt came, and says so", () => {
        answer(view({notice: EReceiptNotice.MISSING}))
        const {container} = renderPage()
        expect(container.querySelector(".vitrine-failure")?.getAttribute("data-notice")).toBe(
            "missing"
        )
        expect(container.querySelector(".vitrine-failure-explanation")?.textContent).toBe(
            "vitrine.receipt.missing"
        )
        expect(container.querySelector(".vitrine-failure-evidence")).toBeNull()
        expect(container.querySelector(".vitrine-failure-save-button")).toBeNull()
    })

    it("doesn't call damaged data a failure", () => {
        answer(
            view({
                chain: "chain",
                result: resultOf("invalid", [
                    tier("record", "invalid"),
                    tier("witnesses", "pending"),
                ]),
            })
        )
        const {container} = renderPage()
        expect(container.querySelector(".vitrine-failure")).toBeNull()
        expect(container.querySelector(".vitrine-confirmation-title")?.textContent).toBe(
            "vitrine.confirmation.cast"
        )
    })
})

describe("the receipts kept on this device", () => {
    const keptFor = (result: ChainResult | undefined) =>
        mockKept.mockReturnValue({
            receipts: [
                {kept: {electionId: "el"}, view: result ? view({chain: "c", result}) : undefined},
            ],
            forgetAll: jest.fn(),
        })
    const presentation = {transparency: "public"}
    const renderKept = () =>
        render(
            <KeptReceipts eventId="e" presentation={presentation} electionNames={{el: "Municipal"}}>
                <div className="elections">elections</div>
            </KeptReceipts>
        )

    it("says the ballot is still in the record, tier by tier", () => {
        keptFor(HOLDS)
        const {container} = renderKept()
        const card = container.querySelector<HTMLElement>(".vitrine-kept-receipt")!
        expect(card.getAttribute("data-phase")).toBe("holds")
        expect(card.querySelector(".vitrine-kept-title")?.textContent).toBe(
            "vitrine.storedReceipts.holdsPublic"
        )
        const tiers = Array.from(card.querySelectorAll(".vitrine-kept-tier")).map((t) => [
            t.getAttribute("data-tier"),
            t.querySelector(".vitrine-kept-tier-value")?.textContent,
        ])
        expect(tiers).toEqual([
            ["witnesses", "Verified"],
            ["hubs", 'vitrine.storedReceipts.hubsAgree {"agree":2,"total":2}'],
            ["browser", "vitrine.storedReceipts.anchored"],
        ])
        expect(card.querySelector(".vitrine-kept-words")?.textContent).toBe(
            "amber · falcon · river · seven"
        )
    })

    it("keeps the page's elections between the receipts and how they are checked", () => {
        keptFor(HOLDS)
        const {container} = renderKept()
        const order = [".vitrine-kept-receipts", ".elections", ".vitrine-how"].map((selector) =>
            container.querySelector(selector)
        )
        expect(order[0]?.compareDocumentPosition(order[1]!)).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
        expect(order[1]?.compareDocumentPosition(order[2]!)).toBe(Node.DOCUMENT_POSITION_FOLLOWING)
    })

    it("waits for proofs that are on their way, without calling it a problem", () => {
        keptFor(PENDING)
        const {container} = renderKept()
        const card = container.querySelector(".vitrine-kept-receipt")
        expect(card?.getAttribute("data-phase")).toBe("waiting")
        expect(card?.querySelector(".vitrine-kept-title")?.textContent).toBe(
            "vitrine.storedReceipts.waiting"
        )
    })

    it("reports contradicting proofs, and a receipt it couldn't check", () => {
        keptFor(SPLIT)
        const problem = renderKept()
        expect(
            problem.container.querySelector(".vitrine-kept-receipt")?.getAttribute("data-phase")
        ).toBe("problem")
        expect(problem.container.querySelector(".vitrine-kept-title")?.textContent).toBe(
            "vitrine.failure.titlePublic"
        )
        problem.unmount()

        keptFor(undefined)
        const unchecked = renderKept()
        expect(
            unchecked.container.querySelector(".vitrine-kept-receipt")?.getAttribute("data-phase")
        ).toBe("unchecked")
    })

    it("opens the kit's receipt on demand, and forgets the receipts when asked", () => {
        keptFor(HOLDS)
        const forgetAll = jest.fn()
        mockKept.mockReturnValue({
            receipts: [{kept: {electionId: "el"}, view: view({chain: "c", result: HOLDS})}],
            forgetAll,
        })
        const {container} = renderKept()
        const toggle = container.querySelector<HTMLElement>(".vitrine-kept-view-button")!
        const panel = container.querySelector<HTMLElement>(".vitrine-kept-receipt-panel")!
        expect(panel.hidden).toBe(true)
        fireEvent.click(toggle)
        expect(panel.hidden).toBe(false)
        expect(toggle.getAttribute("aria-expanded")).toBe("true")
        fireEvent.click(container.querySelector(".vitrine-kept-receipts-forget-button")!)
        expect(forgetAll).toHaveBeenCalledTimes(1)
    })

    it("shows only the page's elections without receipts, or for an event without transparency", () => {
        mockKept.mockReturnValue({receipts: [], forgetAll: jest.fn()})
        const none = renderKept()
        expect(none.container.querySelector(".vitrine-kept-receipts")).toBeNull()
        expect(none.container.querySelector(".elections")).not.toBeNull()
        none.unmount()

        keptFor(HOLDS)
        const off = render(
            <KeptReceipts eventId="e" presentation={{transparency: "disabled"}}>
                <div className="elections">elections</div>
            </KeptReceipts>
        )
        expect(off.container.querySelector(".vitrine-kept-receipts")).toBeNull()
        expect(within(off.container).getByText("elections")).toBeTruthy()
        expect(screen.queryByText("vitrine.storedReceipts.howTitle")).toBeNull()
    })
})

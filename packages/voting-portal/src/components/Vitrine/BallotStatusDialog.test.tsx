/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot status dialog (design board ReceiptDialog), in each state the
// receipt's check can put it in.

import React from "react"
import {act, fireEvent, render, screen, within} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import axe from "axe-core"
import type {ChainResult, TierName, TierResult, TierStatus} from "@vitrine/verify"
import theme from "../../../../ui-essentials/src/services/theme"
import {EBallotStatus} from "../../services/vitrine/ballotStatus"
import {EReceiptNotice, IReceiptView} from "../../services/vitrine/castReceipt"
import {clockMinutes} from "../../services/vitrine/statusRows"
import {BallotStatusDialog, IBallotStatusDialogProps} from "./BallotStatusDialog"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))

const mockSaveTextFile = jest.fn()
jest.mock("../../services/vitrine/file", () => ({
    ...jest.requireActual("../../services/vitrine/file"),
    saveTextFile: (...args: unknown[]) => mockSaveTextFile(...args),
}))

const GROUPED = "30ff d314 4238 6472 fd6d bdb4 df36 9559 0f7a 414d 02ab 2499 5bef c5a9 40b8 9da1"
const BALLOT_ID = GROUPED.replace(/ /g, "")
const DEADLINE = 1_800_000_600

const tier = (name: TierName, status: TierStatus, extra: Partial<TierResult> = {}): TierResult => ({
    tier: name,
    status,
    reasons: [],
    ...extra,
})

const resultOf = (
    status: ChainResult["status"],
    tiers: TierResult[],
    policy: Partial<ChainResult["policy"]> = {}
): ChainResult =>
    ({
        ok: status !== "failed" && status !== "invalid",
        status,
        tiers,
        facts: {included: "done", unchanged: "done", sameForEveryone: "pending"},
        reasons: [],
        steps: [],
        where: "page",
        entry: {log: "ballots", leafHash: "ab", index: 21, promised: false},
        words: [],
        wordsLang: "en",
        now: 0,
        clockWarning: false,
        profileMetadataResolved: true,
        policy: {
            id: "x",
            words: [],
            lang: "en",
            event: "e",
            witnesses: [{name: "a"}, {name: "b"}, {name: "c"}],
            hubs: ["h1", "h2"],
            browser: "bootstrap",
            ...policy,
        },
    }) as unknown as ChainResult

const view = (
    result: ChainResult | undefined,
    extra: Partial<IReceiptView> = {}
): IReceiptView => ({
    notice: EReceiptNotice.NONE,
    chain: "the chain",
    result,
    ...extra,
})

const VERIFIED = resultOf("verified", [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "verified"),
    tier("browser", "verified"),
])
const WITNESSED = resultOf("pending", [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "verified"),
    tier("browser", "pending"),
])
const RECORDED = resultOf("pending", [
    tier("record", "verified"),
    tier("witnesses", "pending"),
    tier("hubs", "pending"),
    tier("browser", "pending"),
])
const LATE = resultOf("late", [
    tier("record", "verified"),
    tier("witnesses", "verified"),
    tier("hubs", "late", {deadline: DEADLINE}),
    tier("browser", "pending"),
])
const MISMATCH = resultOf("failed", [
    tier("record", "failed"),
    tier("witnesses", "pending"),
    tier("hubs", "pending"),
    tier("browser", "pending"),
])

const renderDialog = (props: Partial<IBallotStatusDialogProps> & {status: EBallotStatus}) => {
    const onClose = jest.fn()
    const utils = render(
        <ThemeProvider theme={theme}>
            <BallotStatusDialog
                open
                onClose={onClose}
                ballotId={BALLOT_ID}
                electionName="Municipal council election"
                castAt="29/10/2026, 10:42"
                {...props}
            />
        </ThemeProvider>
    )
    return {...utils, onClose}
}

const dialog = () => screen.getByRole("dialog", {name: "vitrine.status.title"})
const head = () => dialog().querySelector(".ballot-status-head")?.textContent
const sub = () => dialog().querySelector(".ballot-status-sub")?.textContent
const stepStates = () =>
    Array.from(dialog().querySelectorAll(".ballot-status-step")).map((s) =>
        s.getAttribute("data-state")
    )
const stepWords = () =>
    Array.from(dialog().querySelectorAll(".ballot-status-step-word")).map((w) => w.textContent)

beforeEach(() => {
    jest.clearAllMocks()
})

describe("the headline of the ballot status dialog", () => {
    it.each([
        [
            EBallotStatus.NOT_CAST,
            undefined,
            "vitrine.status.state.notcast",
            "vitrine.status.sub.notcast",
        ],
        [
            EBallotStatus.CHECKING,
            undefined,
            "vitrine.status.state.checking",
            "vitrine.status.sub.checking",
        ],
        [
            EBallotStatus.RECORDED,
            RECORDED,
            "vitrine.status.state.recorded",
            "vitrine.status.sub.recorded",
        ],
        [
            EBallotStatus.WITNESSED,
            WITNESSED,
            "vitrine.status.state.witnessed",
            'vitrine.status.sub.witnessed {"minutes":10}',
        ],
        [
            EBallotStatus.VERIFIED,
            VERIFIED,
            "vitrine.status.state.verified",
            "vitrine.status.sub.verified",
        ],
        [EBallotStatus.LATE, LATE, "vitrine.status.head.late", "vitrine.status.sub.late"],
        [
            EBallotStatus.PROBLEM,
            MISMATCH,
            "vitrine.status.head.mismatch",
            'vitrine.status.sub.mismatch {"index":21}',
        ],
        [
            EBallotStatus.AUDITABLE,
            undefined,
            "vitrine.status.state.auditable",
            "vitrine.status.sub.auditable",
        ],
        [
            EBallotStatus.AUDIT_FAILED,
            undefined,
            "vitrine.status.state.auditfail",
            "vitrine.status.sub.auditfail",
        ],
    ] as const)("says what %s means", (status, result, expectedHead, expectedSub) => {
        renderDialog({status, view: result ? view(result) : undefined})
        expect(head()).toBe(expectedHead)
        expect(sub()).toBe(expectedSub)
    })

    it.each([
        ["no receipt", {notice: EReceiptNotice.MISSING}, "noReceipt"],
        ["a receipt that can't be checked", {notice: EReceiptNotice.UNCHECKED}, "unchecked"],
        ["damaged data", {result: resultOf("invalid", [tier("record", "invalid")])}, "damaged"],
    ] as const)("tells the voter why a ballot shows a problem: %s", (_name, extra, problem) => {
        renderDialog({status: EBallotStatus.PROBLEM, view: view(undefined, extra)})
        expect(head()).toBe(`vitrine.status.head.${problem}`)
        expect(sub()).toBe(`vitrine.status.sub.${problem}`)
        expect(
            screen.queryByRole("button", {name: "vitrine.status.callout.saveEvidence"})
        ).toBeNull()
    })

    it("says which step is late when it isn't the anchors", () => {
        const late = resultOf("late", [tier("record", "verified"), tier("witnesses", "late")])
        renderDialog({status: EBallotStatus.LATE, view: view(late)})
        expect(sub()).toBe("vitrine.status.sub.lateStep")
    })
})

describe("the steps of the ballot status dialog", () => {
    it.each([
        [EBallotStatus.NOT_CAST, undefined, ["upcoming", "upcoming", "upcoming"]],
        [EBallotStatus.CHECKING, undefined, ["active", "waiting", "waiting"]],
        [EBallotStatus.RECORDED, RECORDED, ["done", "active", "waiting"]],
        [EBallotStatus.WITNESSED, WITNESSED, ["done", "done", "active"]],
        [EBallotStatus.VERIFIED, VERIFIED, ["done", "done", "done"]],
        [EBallotStatus.LATE, LATE, ["done", "done", "late"]],
        [EBallotStatus.PROBLEM, MISMATCH, ["failed", "skipped", "skipped"]],
        [EBallotStatus.AUDITABLE, undefined, ["done", "done", "done"]],
        [EBallotStatus.AUDIT_FAILED, undefined, ["done", "done", "done"]],
    ] as const)("lists them as %s says", (status, result, states) => {
        renderDialog({status, view: result ? view(result) : undefined})
        expect(stepStates()).toEqual(states)
    })

    it("uses the receipt's facts where it has them", () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        const details = Array.from(dialog().querySelectorAll(".ballot-status-step-detail")).map(
            (d) => d.textContent
        )
        expect(details).toEqual([
            'vitrine.status.row.recordedEntry {"index":21}',
            'vitrine.status.row.witnessed {"count":3}',
            'vitrine.status.row.anchoredBootstrap {"count":2}',
        ])
    })

    it("says when a step should have been done", () => {
        renderDialog({status: EBallotStatus.LATE, view: view(LATE)})
        const details = dialog().querySelectorAll(".ballot-status-step-detail")
        expect(details[2].textContent).toBe(
            `vitrine.status.row.lateAnchors ${JSON.stringify({time: clockMinutes(DEADLINE)})}`
        )
        expect(stepWords()[2]).toBe("vitrine.status.word.late")
        expect(dialog().querySelector(".ballot-status-late")?.textContent).toContain(
            "vitrine.status.callout.late"
        )
    })

    it("shows when this page saw a step done, and nothing when it didn't", () => {
        const at = new Date(2026, 9, 29, 10, 42, 8).getTime()
        renderDialog({
            status: EBallotStatus.WITNESSED,
            view: view(WITNESSED),
            times: {record: at},
        })
        expect(stepWords()).toEqual([
            "10:42:08",
            "",
            'vitrine.status.word.withinMinutes {"minutes":10}',
        ])
    })

    it("says what the mismatch is", () => {
        renderDialog({status: EBallotStatus.PROBLEM, view: view(MISMATCH)})
        expect(stepWords()).toEqual([
            "vitrine.status.word.mismatch",
            "vitrine.status.word.skipped",
            "vitrine.status.word.skipped",
        ])
        expect(dialog().querySelector(".ballot-status-step-detail")?.textContent).toBe(
            'vitrine.status.row.mismatch {"index":21}'
        )
    })
})

describe("the Tracking ID in the ballot status dialog", () => {
    const writeText = jest.fn(async (_text: string) => undefined)
    const savedMatchMedia = window.matchMedia

    beforeEach(() => {
        Object.defineProperty(navigator, "clipboard", {value: {writeText}, configurable: true})
    })

    afterEach(() => {
        window.matchMedia = savedMatchMedia
    })

    it("shows the short ID, and the full ID in two lines of eight groups", () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        expect(dialog().querySelector(".ballot-status-short-id")?.textContent).toBe("30ff d314")
        const lines = Array.from(dialog().querySelectorAll(".ballot-status-full-id > div"))
        expect(lines.map((l) => l.textContent)).toEqual([
            GROUPED.split(" ").slice(0, 8).join(" "),
            GROUPED.split(" ").slice(8).join(" "),
        ])
        expect(lines[0].querySelector("b")?.textContent).toBe("30ff d314")
    })

    it("breaks the full ID into lines of four groups on a phone", () => {
        window.matchMedia = ((query: string) => ({
            matches: true,
            media: query,
            addEventListener: jest.fn(),
            removeEventListener: jest.fn(),
        })) as unknown as typeof window.matchMedia
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        const lines = Array.from(dialog().querySelectorAll(".ballot-status-full-id > div"))
        expect(lines).toHaveLength(4)
        expect(lines[3].textContent).toBe("5bef c5a9 40b8 9da1")
    })

    it("copies the full ID in groups of four, and says so", async () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(writeText).toHaveBeenCalledWith(GROUPED)
        expect(screen.getByRole("button", {name: "vitrine.ballotId.copied"})).toBeTruthy()
        expect(within(dialog()).getByRole("status").textContent).toBe("vitrine.ballotId.copied")
    })

    it("swaps the copy icon for a check of the same size when it copies, and writes no text", async () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        const copy = screen.getByRole("button", {name: "vitrine.ballotId.copy"})
        const icon = copy.querySelector(".ballot-status-copy-icon")
        expect(icon).not.toBeNull()
        expect(copy.querySelector(".ballot-status-copied-icon")).toBeNull()
        expect(copy.textContent).toBe("")
        const size = [icon?.getAttribute("width"), icon?.getAttribute("height")]
        await act(async () => {
            fireEvent.click(copy)
        })
        const done = screen.getByRole("button", {name: "vitrine.ballotId.copied"})
        const check = done.querySelector(".ballot-status-copied-icon")
        expect(done.querySelector(".ballot-status-copy-icon")).toBeNull()
        expect(check).not.toBeNull()
        expect([check?.getAttribute("width"), check?.getAttribute("height")]).toEqual(size)
        expect(done.textContent).toBe("")
        expect(getComputedStyle(done).color).toBe("rgb(4, 120, 87)")
    })

    it("keeps the copy button a 28 px square, before and after it copies", async () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        const square = (button: HTMLElement) => {
            const style = getComputedStyle(button)
            return [style.width, style.height]
        }
        expect(square(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))).toEqual([
            "28px",
            "28px",
        ])
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(square(screen.getByRole("button", {name: "vitrine.ballotId.copied"}))).toEqual([
            "28px",
            "28px",
        ])
    })

    it("has an 8 px radius, and the Tracking ID label above the short ID", () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        const box = dialog().querySelector<HTMLElement>(".ballot-status-id")
        expect(getComputedStyle(box!).borderRadius).toBe("8px")
        expect(box?.querySelector(".ballot-status-id-label")?.textContent).toBe(
            "vitrine.ballotId.label"
        )
    })

    it.each([
        [EBallotStatus.NOT_CAST, "notCast", "neutral", undefined],
        [EBallotStatus.CHECKING, "cast", "success", undefined],
        [EBallotStatus.RECORDED, "cast", "success", RECORDED],
        [EBallotStatus.WITNESSED, "cast", "success", WITNESSED],
        [EBallotStatus.VERIFIED, "cast", "success", VERIFIED],
        [EBallotStatus.LATE, "cast", "success", LATE],
        [EBallotStatus.PROBLEM, "cast", "success", MISMATCH],
        [EBallotStatus.AUDITABLE, "auditable", "success", undefined],
        [EBallotStatus.AUDIT_FAILED, "auditable", "success", undefined],
    ] as const)(
        "tags a ballot that is %s with the word %s in the %s tone, at the right of the label",
        (status, word, tone, result) => {
            renderDialog({status, view: result ? view(result) : undefined})
            const head = dialog().querySelector(".ballot-status-id-head")
            const tags = Array.from(dialog().querySelectorAll(".status-tag"))
            expect(tags).toHaveLength(1)
            expect(head?.contains(tags[0])).toBe(true)
            expect(tags[0].textContent).toBe(`vitrine.status.tag.${word}`)
            expect(tags[0].getAttribute("data-tone")).toBe(tone)
            expect(head?.firstElementChild?.className).toContain("ballot-status-id-label")
            expect(head?.lastElementChild).toBe(tags[0])
        }
    )

    it("says nothing was copied when the clipboard refuses", async () => {
        writeText.mockRejectedValueOnce(new Error("denied"))
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(screen.queryByRole("button", {name: "vitrine.ballotId.copied"})).toBeNull()
    })
})

describe("what the ballot status dialog offers", () => {
    const tracker = {href: "/tenant/t/event/e/election/el/ballot-locator/abc", target: "_blank"}

    it("names the election and when the ballot was cast", () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        expect(dialog().querySelector(".ballot-status-meta")?.textContent).toBe(
            'Municipal council election · vitrine.status.cast {"when":"29/10/2026, 10:42"}'
        )
    })

    it("says when an audited ballot was audited, and not that it was cast", () => {
        renderDialog({status: EBallotStatus.AUDITABLE, castAt: "29/10/2026, 10:40"})
        expect(dialog().querySelector(".ballot-status-meta")?.textContent).toBe(
            'Municipal council election · vitrine.status.audited {"when":"29/10/2026, 10:40"}'
        )
    })

    it("opens an audited ballot in the ballot tracker, with the caption of its state", () => {
        renderDialog({status: EBallotStatus.AUDITABLE, tracker})
        const caption = dialog().querySelector(".ballot-status-caption")
        expect(caption?.textContent).toContain("vitrine.status.caption.auditable")
        expect(caption?.textContent).not.toContain("vitrine.status.caption.kept")
        expect(
            screen.getByRole("link", {name: "vitrine.status.tracker"}).getAttribute("href")
        ).toBe(tracker.href)
    })

    it("colours the head of a failed audit as it does a problem", () => {
        renderDialog({status: EBallotStatus.AUDIT_FAILED})
        expect(getComputedStyle(dialog().querySelector(".ballot-status-head")!).color).toBe(
            "rgb(185, 28, 28)"
        )
        expect(dialog().querySelector(".ballot-status-caption")?.textContent).toContain(
            "vitrine.status.caption.problem"
        )
    })

    it("offers the evidence of a failed audit only where a receipt was kept", () => {
        const kept = renderDialog({status: EBallotStatus.AUDIT_FAILED, view: view(undefined)})
        fireEvent.click(screen.getByRole("button", {name: "vitrine.status.callout.saveEvidence"}))
        expect(mockSaveTextFile).toHaveBeenCalledTimes(1)
        kept.unmount()
        renderDialog({
            status: EBallotStatus.AUDIT_FAILED,
            view: view(undefined, {chain: undefined}),
        })
        expect(
            screen.queryByRole("button", {name: "vitrine.status.callout.saveEvidence"})
        ).toBeNull()
    })

    it("offers no evidence for an audit that did not fail", () => {
        renderDialog({status: EBallotStatus.AUDITABLE, view: view(undefined)})
        expect(
            screen.queryByRole("button", {name: "vitrine.status.callout.saveEvidence"})
        ).toBeNull()
    })

    it("offers neither the receipt nor the tracker before the ballot is cast", () => {
        renderDialog({status: EBallotStatus.NOT_CAST, castAt: undefined, tracker})
        expect(dialog().querySelector(".ballot-status-meta")?.textContent).toBe(
            "Municipal council election"
        )
        expect(screen.queryByRole("button", {name: "vitrine.status.save"})).toBeNull()
        expect(screen.queryByRole("link", {name: "vitrine.status.tracker"})).toBeNull()
        expect(screen.getByRole("button", {name: "vitrine.status.ok"})).toBeTruthy()
    })

    it("opens the ballot in the ballot tracker", () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED), tracker})
        const link = screen.getByRole("link", {name: "vitrine.status.tracker"})
        expect(link.getAttribute("href")).toBe(tracker.href)
        expect(link.getAttribute("target")).toBe("_blank")
    })

    it("saves the receipt under the short ID's name", () => {
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        fireEvent.click(screen.getByRole("button", {name: "vitrine.status.save"}))
        expect(mockSaveTextFile).toHaveBeenCalledWith(
            "ballot-30ff-d314.vitrine-receipt",
            "the chain",
            "application/vnd.vitrine.receipt-chain"
        )
    })

    it("can't save a receipt that hasn't come", () => {
        renderDialog({status: EBallotStatus.CHECKING, view: view(undefined, {chain: undefined})})
        const save = screen.getByRole("button", {name: "vitrine.status.save"})
        expect(save.hasAttribute("disabled")).toBe(true)
    })

    it("offers to save the evidence of a mismatch", () => {
        renderDialog({status: EBallotStatus.PROBLEM, view: view(MISMATCH)})
        fireEvent.click(screen.getByRole("button", {name: "vitrine.status.callout.saveEvidence"}))
        expect(mockSaveTextFile).toHaveBeenCalledTimes(1)
        expect(mockSaveTextFile.mock.calls[0][1]).toBe("the chain")
    })

    it("tells a device that doesn't keep the receipt not to expect another check", () => {
        const kept = renderDialog({
            status: EBallotStatus.VERIFIED,
            view: view(VERIFIED),
            kept: true,
        })
        expect(dialog().querySelector(".ballot-status-caption")?.textContent).toContain(
            "vitrine.status.caption.kept"
        )
        kept.unmount()
        renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED), kept: false})
        expect(dialog().querySelector(".ballot-status-caption")?.textContent).not.toContain(
            "vitrine.status.caption.kept"
        )
    })

    it("closes with OK and with the close button", () => {
        const {onClose} = renderDialog({status: EBallotStatus.VERIFIED, view: view(VERIFIED)})
        fireEvent.click(screen.getByRole("button", {name: "vitrine.status.ok"}))
        fireEvent.click(screen.getByRole("button", {name: "a11y.closeDialog"}))
        expect(onClose).toHaveBeenCalledTimes(2)
    })
})

describe("the ballot status dialog's accessibility", () => {
    // Colour contrast needs a layout engine; the board's colours are checked on the page.
    it.each([
        [EBallotStatus.NOT_CAST, undefined],
        [EBallotStatus.VERIFIED, VERIFIED],
        [EBallotStatus.LATE, LATE],
        [EBallotStatus.PROBLEM, MISMATCH],
        [EBallotStatus.AUDITABLE, undefined],
        [EBallotStatus.AUDIT_FAILED, undefined],
    ] as const)("has no violations when %s", async (status, result) => {
        renderDialog({
            status,
            view: result ? view(result) : undefined,
            tracker: {href: "/tracker"},
        })
        const {violations} = await axe.run(document.body, {
            rules: {"color-contrast": {enabled: false}},
        })
        expect(
            violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)
        ).toEqual([])
    })
})

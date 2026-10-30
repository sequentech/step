/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// What the portal shows around the ballot status dialog (the design boards
// Review, Confirmation and BallotList): the check mark that opens it, and the
// Ballot ID box of the review screen.

import React from "react"
import {act, fireEvent, render, screen, waitFor, within} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import axe from "axe-core"
import type {ChainResult, TierName, TierResult, TierStatus} from "@vitrine/verify"
import theme from "../../../../ui-essentials/src/services/theme"
import {EBallotStatus} from "../../services/vitrine/ballotStatus"
import {EReceiptNotice, IReceiptView} from "../../services/vitrine/castReceipt"
import {clockSeconds} from "../../services/vitrine/statusRows"
import {BallotStatus, EBallotStatusTrigger, IBallotStatusProps} from "./BallotStatus"
import {ReviewBallotId} from "./ReviewBallotId"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
    }),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        BallotHashCopyButton: jest.requireActual(
            "../../../../ui-essentials/src/components/BallotHash/BallotHash"
        ).BallotHashCopyButton,
        IconButton: jest.requireActual(
            "../../../../ui-essentials/src/components/IconButton/IconButton"
        ).default,
        theme: jest.requireActual("../../../../ui-essentials/src/services/theme").default,
    }),
    {virtual: true}
)
jest.mock("../../providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual<typeof React>("react").createContext({
        isKiosk: () => mockKiosk,
    }),
}))

let mockKiosk = false

const GROUPED = "30ff d314 4238 6472 fd6d bdb4 df36 9559 0f7a 414d 02ab 2499 5bef c5a9 40b8 9da1"
const BALLOT_ID = GROUPED.replace(/ /g, "")
const TRACKER = "/tenant/t/event/e/election/el/ballot-locator/abc"

const tier = (name: TierName, status: TierStatus): TierResult => ({
    tier: name,
    status,
    reasons: [],
})

const resultOf = (status: ChainResult["status"], tiers: TierResult[]): ChainResult =>
    ({
        ok: true,
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
        policy: {id: "x", words: [], lang: "en", event: "e", witnesses: [], hubs: []},
    }) as unknown as ChainResult

const RECORDED: IReceiptView = {
    notice: EReceiptNotice.NONE,
    chain: "chain",
    result: resultOf("pending", [
        tier("record", "verified"),
        tier("witnesses", "pending"),
        tier("hubs", "pending"),
        tier("browser", "pending"),
    ]),
}
const WITNESSED: IReceiptView = {
    ...RECORDED,
    result: resultOf("pending", [
        tier("record", "verified"),
        tier("witnesses", "verified"),
        tier("hubs", "pending"),
        tier("browser", "pending"),
    ]),
}

const renderStatus = (props: Partial<IBallotStatusProps> = {}) => {
    const element = (p: Partial<IBallotStatusProps>) => (
        <ThemeProvider theme={theme}>
            <MemoryRouter initialEntries={["/start"]}>
                <Routes>
                    <Route
                        path="/start"
                        element={
                            <BallotStatus
                                trigger={EBallotStatusTrigger.ICON}
                                ballotId={BALLOT_ID}
                                electionName="Municipal council election"
                                trackerUrl={TRACKER}
                                {...p}
                            />
                        }
                    />
                    <Route path="/tenant/*" element={<div>the ballot tracker</div>} />
                </Routes>
            </MemoryRouter>
        </ThemeProvider>
    )
    const utils = render(element(props))
    return {...utils, update: (p: Partial<IBallotStatusProps>) => utils.rerender(element(p))}
}

const stateWords = (status: EBallotStatus): string => `vitrine.status.state.${status}`

beforeEach(() => {
    mockKiosk = false
    jest.restoreAllMocks()
})

describe("the check mark next to a Ballot ID", () => {
    it.each(Object.values(EBallotStatus))("names the state when it is %s", (status) => {
        renderStatus({status})
        const button = screen.getByRole("button", {
            name: `vitrine.status.aria ${JSON.stringify({state: stateWords(status)})}`,
        })
        expect(button.getAttribute("data-status")).toBe(status)
        expect(button.getAttribute("aria-haspopup")).toBe("dialog")
    })

    it("shows the state in its tooltip", async () => {
        renderStatus({status: EBallotStatus.VERIFIED})
        fireEvent.focus(screen.getByRole("button", {name: /^vitrine.status.aria/}))
        fireEvent.mouseOver(screen.getByRole("button", {name: /^vitrine.status.aria/}))
        expect(
            await screen.findByRole("tooltip", {
                name: `vitrine.status.tooltip ${JSON.stringify({state: stateWords(EBallotStatus.VERIFIED)})}`,
            })
        ).toBeTruthy()
    })

    it("takes the status from the receipt", () => {
        renderStatus({view: RECORDED})
        expect(
            screen.getByRole("button", {name: /^vitrine.status.aria/}).getAttribute("data-status")
        ).toBe(EBallotStatus.RECORDED)
    })

    it("opens the dialog, and closes it again", async () => {
        renderStatus({status: EBallotStatus.NOT_CAST})
        expect(screen.queryByRole("dialog")).toBeNull()
        fireEvent.click(screen.getByRole("button", {name: /^vitrine.status.aria/}))
        const dialog = screen.getByRole("dialog", {name: "vitrine.status.title"})
        expect(within(dialog).getByText(stateWords(EBallotStatus.NOT_CAST))).toBeTruthy()
        fireEvent.click(within(dialog).getByRole("button", {name: "vitrine.status.ok"}))
        await waitFor(() =>
            expect(screen.queryByRole("dialog", {name: "vitrine.status.title"})).toBeNull()
        )
    })

    it("shows when this page saw each step done", () => {
        const at = new Date(2026, 9, 29, 10, 42, 8).getTime()
        const spy = jest.spyOn(Date, "now").mockReturnValue(at)
        const {update} = renderStatus({view: RECORDED})
        act(() => {
            spy.mockReturnValue(at + 1000)
            update({view: WITNESSED})
        })
        fireEvent.click(screen.getByRole("button", {name: /^vitrine.status.aria/}))
        const words = Array.from(
            screen.getByRole("dialog").querySelectorAll(".ballot-status-step-word")
        ).map((w) => w.textContent)
        expect(words[0]).toBe(clockSeconds(at))
        expect(words[1]).toBe(clockSeconds(at + 1000))
    })

    it("opens the ballot tracker in a new tab", () => {
        renderStatus({view: WITNESSED})
        fireEvent.click(screen.getByRole("button", {name: /^vitrine.status.aria/}))
        const link = screen.getByRole("link", {name: "vitrine.status.tracker"})
        expect(link.getAttribute("href")).toBe(TRACKER)
        expect(link.getAttribute("target")).toBe("_blank")
        expect(link.getAttribute("rel")).toContain("noopener")
    })

    it("opens the ballot tracker in the same tab on a kiosk, which keeps its session in memory", () => {
        mockKiosk = true
        renderStatus({view: WITNESSED})
        fireEvent.click(screen.getByRole("button", {name: /^vitrine.status.aria/}))
        const link = screen.getByRole("link", {name: "vitrine.status.tracker"})
        expect(link.getAttribute("target")).toBeNull()
        fireEvent.click(link)
        expect(screen.getByText("the ballot tracker")).toBeTruthy()
    })
})

describe("the ballot cast label of the ballot list", () => {
    it("doesn't start the vote of the card around it when the dialog is clicked", () => {
        const onCardClick = jest.fn()
        render(
            <ThemeProvider theme={theme}>
                <MemoryRouter>
                    <div className="election-card" onClick={onCardClick}>
                        <BallotStatus
                            trigger={EBallotStatusTrigger.LABEL}
                            view={WITNESSED}
                            ballotId={BALLOT_ID}
                        />
                    </div>
                </MemoryRouter>
            </ThemeProvider>
        )
        fireEvent.click(screen.getByRole("button", {name: /^vitrine.status.ariaCast/}))
        const dialog = screen.getByRole("dialog", {name: "vitrine.status.title"})
        fireEvent.click(dialog.querySelector(".ballot-status-sub")!)
        fireEvent.click(within(dialog).getByRole("button", {name: "vitrine.status.ok"}))
        expect(onCardClick).toHaveBeenCalledTimes(1)
    })

    it("is the button that opens the dialog", () => {
        renderStatus({trigger: EBallotStatusTrigger.LABEL, view: WITNESSED, kept: true})
        const button = screen.getByRole("button", {
            name: `vitrine.status.ariaCast ${JSON.stringify({state: stateWords(EBallotStatus.WITNESSED)})}`,
        })
        expect(within(button).getByText("vitrine.status.ballotCast")).toBeTruthy()
        fireEvent.click(button)
        expect(screen.getByRole("dialog", {name: "vitrine.status.title"})).toBeTruthy()
    })

    it("shows a ballot that is checking until its receipt is there", () => {
        renderStatus({trigger: EBallotStatusTrigger.LABEL, view: undefined})
        expect(
            screen
                .getByRole("button", {name: /^vitrine.status.ariaCast/})
                .getAttribute("data-status")
        ).toBe(EBallotStatus.CHECKING)
    })
})

describe("the Ballot ID box of the review screen", () => {
    const writeText = jest.fn(async (_text: string) => undefined)
    const onHelpClick = jest.fn()

    const renderBox = () =>
        render(
            <ThemeProvider theme={theme}>
                <MemoryRouter>
                    <ReviewBallotId
                        ballotId={BALLOT_ID}
                        electionName="Municipal council election"
                        helpLabel="About your Ballot ID"
                        onHelpClick={onHelpClick}
                    />
                </MemoryRouter>
            </ThemeProvider>
        )

    beforeEach(() => {
        onHelpClick.mockClear()
        Object.defineProperty(navigator, "clipboard", {value: {writeText}, configurable: true})
    })

    it("shows the short ID after the label", () => {
        const {container} = renderBox()
        expect(container.querySelector(".hash-text")?.textContent).toBe(
            "vitrine.ballotId.yours 30ff d314"
        )
        expect(container.querySelector(".vitrine-short-ballot-id")?.textContent).toBe("30ff d314")
    })

    it("shows the check mark of a ballot not cast yet, and opens its dialog", () => {
        renderBox()
        const button = screen.getByRole("button", {name: /^vitrine.status.aria/})
        expect(button.getAttribute("data-status")).toBe(EBallotStatus.NOT_CAST)
        fireEvent.click(button)
        const dialog = screen.getByRole("dialog", {name: "vitrine.status.title"})
        expect(within(dialog).queryByRole("button", {name: "vitrine.status.save"})).toBeNull()
        expect(within(dialog).queryByRole("link")).toBeNull()
    })

    it("copies the full ID in groups of four", async () => {
        renderBox()
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(writeText).toHaveBeenCalledWith(GROUPED)
    })

    it("opens the help about the Ballot ID", () => {
        renderBox()
        fireEvent.click(screen.getByRole("button", {name: "About your Ballot ID"}))
        expect(onHelpClick).toHaveBeenCalledTimes(1)
    })
})

describe("the status check mark's accessibility", () => {
    // Colour contrast needs a layout engine, and the fragments aren't whole pages.
    it("has no violations in the review screen's box, the ballot list, or with the dialog open", async () => {
        const box = render(
            <ThemeProvider theme={theme}>
                <MemoryRouter>
                    <ReviewBallotId
                        ballotId={BALLOT_ID}
                        helpLabel="About your Ballot ID"
                        onHelpClick={jest.fn()}
                    />
                    <BallotStatus
                        trigger={EBallotStatusTrigger.LABEL}
                        view={WITNESSED}
                        ballotId={BALLOT_ID}
                    />
                </MemoryRouter>
            </ThemeProvider>
        )
        const check = async () =>
            (
                await axe.run(document.body, {
                    rules: {"color-contrast": {enabled: false}, "region": {enabled: false}},
                })
            ).violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target.join(" ")).join(", ")}`)
        expect(await check()).toEqual([])
        fireEvent.click(box.getAllByRole("button", {name: /^vitrine.status.ariaCast/})[0])
        expect(await check()).toEqual([])
    })
})

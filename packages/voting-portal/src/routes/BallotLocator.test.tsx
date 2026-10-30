/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot tracker (design boards TrackerSearch, TrackerResult and
// PhoneTracker): the search field, the list of ballots that share a short ID,
// and the result in each state of the receipt's check.

import React from "react"
import {render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import type {ChainResult, TierName, TierResult, TierStatus} from "@vitrine/verify"
import theme from "../../../ui-essentials/src/services/theme"
import {EReceiptNotice, IReceiptView} from "../services/vitrine/castReceipt"
import type {ITrackerReceipt} from "../services/vitrine/ballotTracker"
import BallotLocator from "./BallotLocator"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: Record<string, unknown>) =>
            values ? `${key} ${JSON.stringify(values)}` : key,
        i18n: {language: "en", resolvedLanguage: "en"},
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    stringToHtml: (value: string) => value,
    formatVotingPortalDateTime: (time: unknown) => `at ${String(time)}`,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        theme: jest.requireActual("../../../ui-essentials/src/services/theme").default,
        PageLimit: ({children}: {children: React.ReactNode}) => <div>{children}</div>,
        InfoDataBox: ({children}: {children: React.ReactNode}) => <pre>{children}</pre>,
        BreadCrumbSteps: ({selected}: {selected: number}) => (
            <div data-testid="steps">{selected}</div>
        ),
        Icon: () => null,
        IconButton: () => null,
        Dialog: () => null,
        ExpandableText: () => null,
    }),
    {virtual: true}
)
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {DISABLE_AUTH: false, BALLOT_VERIFIER_URL: "https://verifier.example/"},
    }),
}))
jest.mock("../store/hooks", () => ({
    useAppDispatch: () => jest.fn(),
    useAppSelector: () => undefined,
}))

const mockTracked = jest.fn()
jest.mock("../hooks/useTrackedReceipt", () => ({
    useTrackedReceipt: (...args: unknown[]) => mockTracked(...args),
}))

interface ICastVote {
    ballot_id: string
    created_at: string
    content: string
}

let mockPresentation: Record<string, unknown> = {}
let mockTelephone = false
let mockVotes: ICastVote[] = []
const mockQueries: {name: string; variables: Record<string, unknown>}[] = []

jest.mock("@apollo/client/react", () => ({
    useQuery: (
        query: {definitions: {kind: string; name?: {value: string}}[]},
        options: {variables: Record<string, unknown>}
    ) => {
        const name = query.definitions.find((d) => d.kind === "OperationDefinition")?.name?.value
        mockQueries.push({name: name ?? "", variables: options.variables})
        const data =
            name === "GetElectionEvent"
                ? {sequent_backend_election_event: [{id: "event", presentation: mockPresentation}]}
                : name === "GetElections"
                  ? {
                        sequent_backend_election: [
                            {id: "election", voting_channels: {telephone: mockTelephone}},
                        ],
                    }
                  : name === "GetCastVote"
                    ? {cast_votes: mockVotes}
                    : undefined
        return {data, loading: false, refetch: jest.fn()}
    },
}))

const FULL = "30ffd31442386472fd6dbdb4df3695590f7a414d02ab24995befc5a940b89da1"
const OTHER = "30ffd3149c071be27a50e3d946b108fcd25e6f13b7a4c9013e885d27a0c6f41b"
const SHORT = "30ffd314"
const GROUPED = "30ff d314 4238 6472 fd6d bdb4 df36 9559 0f7a 414d 02ab 2499 5bef c5a9 40b8 9da1"
const NOW = 1_793_270_528
const PATH = "/tenant/tenant/event/event/election/election/ballot-locator"

const vote = (ballotId: string, createdAt = "2026-10-29T10:42:08.000000+00:00"): ICastVote => ({
    ballot_id: ballotId,
    created_at: createdAt,
    content: `content of ${ballotId.slice(0, 8)}`,
})

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
        steps: [],
        entry: {
            log: "ballots",
            leafHash: `b1d8c4756132${"0".repeat(48)}5a0d`,
            index: 21,
            promised: false,
        },
        policy: {hubs: ["hub-a", "hub-b"]},
        now: NOW,
    }) as unknown as ChainResult

const viewOf = (result: ChainResult): IReceiptView => ({
    notice: EReceiptNotice.NONE,
    chain: "chain",
    own: {entry: new Uint8Array(), acceptedAt: NOW},
    result,
})

const VERIFIED = viewOf(
    resultOf("verified", [
        tier("record", "verified"),
        tier("witnesses", "verified"),
        tier("hubs", "verified"),
        tier("browser", "verified"),
    ])
)
const LATE = viewOf(
    resultOf("late", [
        tier("record", "verified"),
        tier("witnesses", "verified"),
        tier("hubs", "late", {deadline: NOW - 60}),
        tier("browser", "pending"),
    ])
)
const PROBLEM = viewOf(
    resultOf("failed", [
        tier("record", "failed"),
        tier("witnesses", "pending"),
        tier("hubs", "pending"),
        tier("browser", "pending"),
    ])
)

const tracked = (patch: Partial<ITrackerReceipt>): ITrackerReceipt => ({
    kept: true,
    settled: true,
    ...patch,
})

let router: ReturnType<typeof createMemoryRouter>

const renderAt = (suffix = "") => {
    router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/ballot-locator/:ballotId?",
                element: <BallotLocator />,
            },
        ],
        {
            initialEntries: [`${PATH}${suffix}`],
        }
    )
    return render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
}

const field = () => screen.getByLabelText("a11y.ballotIdLabel") as HTMLInputElement
const matchLinks = () => Array.from(document.querySelectorAll<HTMLAnchorElement>("a.ballot-match"))
const findButton = () => screen.getByRole("button", {name: "ballotLocator.locate"})

beforeEach(() => {
    mockPresentation = {}
    mockTelephone = false
    mockVotes = []
    mockQueries.length = 0
    mockTracked.mockReset()
    mockTracked.mockReturnValue(tracked({view: VERIFIED}))
})

describe("the search", () => {
    it("groups the ID in fours as the voter types", async () => {
        renderAt()
        await userEvent.type(field(), "30FFd3144238")
        expect(field().value).toBe("30ff d314 4238")
    })

    it("takes the short ID and the full one, with or without spaces", async () => {
        renderAt()
        expect(findButton()).toBeDisabled()
        await userEvent.type(field(), "30ff d31")
        expect(findButton()).toBeDisabled()
        await userEvent.type(field(), "4")
        expect(findButton()).toBeEnabled()
        await userEvent.type(field(), "4238")
        expect(findButton()).toBeDisabled()
        await userEvent.clear(field())
        await userEvent.paste(GROUPED)
        expect(field().value).toBe(GROUPED)
        expect(findButton()).toBeEnabled()
    })

    it("explains what the ID can be, and says when a character can't be part of it", async () => {
        renderAt()
        expect(screen.getByText("ballotLocator.idHelp")).toBeVisible()
        expect(screen.getByRole("alert")).toBeEmptyDOMElement()
        await userEvent.type(field(), "30ffg")
        expect(field().value).toBe("30ff")
        expect(screen.getByRole("alert")).toHaveTextContent("ballotLocator.wrongFormatBallotId")
        expect(screen.queryByText("ballotLocator.idHelp")).toBeNull()
        await userEvent.type(field(), "d")
        expect(screen.getByRole("alert")).toBeEmptyDOMElement()
    })

    it("finds the ballot by the ID without its spaces", async () => {
        renderAt()
        await userEvent.type(field(), "30ff d314")
        await userEvent.click(findButton())
        expect(router.state.location.pathname).toBe(`${PATH}/${SHORT}`)
    })

    it("finds the ballot when the voter presses Enter, but only for an ID", async () => {
        renderAt()
        await userEvent.type(field(), "30ff{Enter}")
        expect(router.state.location.pathname).toBe(PATH)
        await userEvent.type(field(), "d314{Enter}")
        expect(router.state.location.pathname).toBe(`${PATH}/${SHORT}`)
    })

    it("takes the first four characters for events with telephone voting", async () => {
        mockTelephone = true
        renderAt()
        await userEvent.type(field(), "30ff")
        expect(findButton()).toBeEnabled()
    })
})

describe("ballots that share a short ID", () => {
    beforeEach(() => {
        mockVotes = [
            vote(FULL, "2026-10-29T10:42:08.000000+00:00"),
            vote(OTHER, "2026-10-28T18:05:00.000000+00:00"),
        ]
    })

    it("lists each with its full ID and when it was cast, and asks for the prefix", () => {
        renderAt(`/${SHORT}`)
        expect(mockQueries.find((q) => q.name === "GetCastVote")?.variables).toEqual({
            electionEventId: "event",
            electionId: "election",
            ballotIdPrefix: SHORT,
        })
        expect(screen.getByText(/ballotLocator\.matches\.title/)).toHaveTextContent(
            '{"total":2,"shortId":"30ff d314"}'
        )
        const links = matchLinks()
        expect(links).toHaveLength(2)
        expect(links[0]).toHaveAttribute("href", `${PATH}/${FULL}`)
        expect(links[0]).toHaveTextContent(GROUPED)
        expect(links[0]).toHaveTextContent("ballotLocator.matches.cast")
        expect(links[0]).toHaveTextContent("at 2026-10-29T10:42:08.000000+00:00")
        expect(links[1]).toHaveAttribute("href", `${PATH}/${OTHER}`)
    })

    it("keeps the search step with the short ID in the field", () => {
        renderAt(`/${SHORT}`)
        expect(screen.getByTestId("steps")).toHaveTextContent("0")
        expect(field().value).toBe("30ff d314")
        expect(findButton()).toBeEnabled()
        expect(screen.queryByRole("button", {name: "ballotLocator.locateAgain"})).toBeNull()
    })

    it("opens the ballot that the voter chooses", async () => {
        renderAt(`/${SHORT}`)
        await userEvent.click(matchLinks()[1])
        expect(router.state.location.pathname).toBe(`${PATH}/${OTHER}`)
    })
})

describe("the result for an event with transparency", () => {
    beforeEach(() => {
        mockPresentation = {transparency: "public"}
        mockVotes = [vote(FULL)]
    })

    const section = () => screen.getByRole("region", {name: "ballotLocator.result.receipt"})

    it("names the ballot by its short ID and shows both IDs", () => {
        renderAt(`/${FULL}`)
        expect(screen.getByTestId("steps")).toHaveTextContent("1")
        expect(screen.getByText(/ballotLocator\.found /)).toHaveTextContent(
            '"ballotId":"30ff d314"'
        )
        expect(screen.queryByText("ballotLocator.foundShort")).toBeNull()
        const ids = within(section())
        expect(ids.getByText("30ff d314", {selector: ".tracker-short-id"})).toBeVisible()
        expect(ids.getByText("ballotLocator.result.idComment")).toBeVisible()
        const full = section().querySelector(".tracker-full-id")
        expect(full?.textContent).toBe(GROUPED)
        expect(full?.querySelector("b")?.textContent).toBe("30ff d314")
        expect(ids.getByText("b1d8 c475 6132 ··· 5a0d")).toBeVisible()
        expect(ids.getByRole("button", {name: "ballotLocator.result.save"})).toBeVisible()
    })

    it("tells the voter who found the ballot by its short ID to compare the full one", () => {
        renderAt(`/${SHORT}`)
        expect(screen.getByText("ballotLocator.foundShort")).toBeVisible()
        expect(mockTracked.mock.calls.at(-1)?.[0]).toEqual({
            eventId: "event",
            electionId: "election",
            ballotId: FULL,
            acceptedAt: Date.UTC(2026, 9, 29, 10, 42, 8) / 1000,
        })
    })

    it("copies the full ID, grouped", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        Object.defineProperty(navigator, "clipboard", {value: {writeText}, configurable: true})
        renderAt(`/${FULL}`)
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.result.copy"}))
        expect(writeText).toHaveBeenCalledWith(GROUPED)
        expect(screen.getByRole("button", {name: "ballotLocator.result.copied"})).toBeVisible()
    })

    it("shows a verified ballot with its four steps done", () => {
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.status.verified.head")).toBeVisible()
        expect(screen.getByRole("img", {name: /ballotLocator\.result\.sealLabel/})).toBeVisible()
        const steps = section().querySelectorAll(".tracker-step")
        expect(steps).toHaveLength(4)
        steps.forEach((step) => expect(step).toHaveClass("tracker-step-done"))
        expect(screen.getByText("ballotLocator.result.captionVerified")).toBeVisible()
        expect(screen.getByRole("link", {name: "ballotLocator.result.check"})).toBeVisible()
        expect(screen.queryByText("ballotLocator.result.late")).toBeNull()
        expect(screen.queryByText("ballotLocator.result.problem")).toBeNull()
    })

    it("shows anchors that are due later as scheduled", () => {
        mockTracked.mockReturnValue(
            tracked({
                view: viewOf(
                    resultOf("pending", [
                        tier("record", "verified"),
                        tier("witnesses", "verified"),
                        tier("hubs", "verified"),
                        tier("browser", "pending", {deadline: NOW + 600}),
                    ])
                ),
            })
        )
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.status.witnessed.head")).toBeVisible()
        expect(
            screen.getByText(/ballotLocator\.result\.status\.witnessed\.subAnchored/)
        ).toHaveTextContent('"count":10')
        expect(section().querySelector(".tracker-step-browser")).toHaveClass(
            "tracker-step-scheduled"
        )
    })

    it("shows a late step and says it isn't evidence of tampering", () => {
        mockTracked.mockReturnValue(tracked({view: LATE}))
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.status.late.head")).toBeVisible()
        expect(section().querySelector(".tracker-step-hubs")).toHaveClass("tracker-step-late")
        expect(section().querySelector(".tracker-step-browser")).toHaveClass("tracker-step-waiting")
        expect(screen.getByText("ballotLocator.result.late")).toBeVisible()
    })

    it("shows a ballot missing from the record as a problem, with the evidence to save", () => {
        mockTracked.mockReturnValue(tracked({view: PROBLEM}))
        renderAt(`/${FULL}`)
        expect(
            screen.getAllByText("ballotLocator.result.status.problem.head").length
        ).toBeGreaterThan(0)
        expect(section().querySelector(".tracker-step-record")).toHaveClass("tracker-step-failed")
        expect(section().querySelector(".tracker-step-witnesses")).toHaveClass(
            "tracker-step-skipped"
        )
        expect(screen.getByRole("alert")).toHaveTextContent("ballotLocator.result.problem")
        expect(
            screen.getByRole("button", {name: "ballotLocator.result.saveEvidence"})
        ).toBeVisible()
        expect(screen.getByText("ballotLocator.result.captionProblem")).toBeVisible()
    })

    it("keeps checking until the receipt has been looked for", () => {
        mockTracked.mockReturnValue(tracked({kept: false, settled: false}))
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.status.checking.head")).toBeVisible()
        expect(section().querySelector(".tracker-step-record")).toHaveClass("tracker-step-active")
        expect(screen.queryByRole("button", {name: "ballotLocator.result.save"})).toBeNull()
    })

    it("says honestly that this device has no receipt, and shows no status", () => {
        mockTracked.mockReturnValue(tracked({kept: false, settled: true}))
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.noReceipt")).toBeVisible()
        expect(screen.queryByRole("img", {name: /sealLabel/})).toBeNull()
        expect(screen.queryByText(/ballotLocator\.result\.status\./)).toBeNull()
        expect(section().querySelectorAll(".tracker-step")).toHaveLength(0)
        expect(screen.getByText("30ff d314", {selector: ".tracker-short-id"})).toBeVisible()
        expect(screen.getByText(/ballotLocator\.found /)).toBeVisible()
    })

    it("says so when a kept receipt couldn't be completed or checked", () => {
        mockTracked.mockReturnValue(
            tracked({view: {notice: EReceiptNotice.WAITING}, kept: true, settled: true})
        )
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.receiptPending")).toBeVisible()
    })

    it("shows the ballot's content and goes back to the search", async () => {
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.contentDesc")).toBeVisible()
        expect(screen.getByText("content of 30ffd314")).toBeVisible()
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.locateAgain"}))
        expect(router.state.location.pathname).toBe(`${PATH}/`)
        expect(findButton()).toBeDisabled()
    })
})

describe("the result for an event without transparency", () => {
    it("keeps the original result", () => {
        mockVotes = [vote(FULL)]
        renderAt(`/${FULL}`)
        expect(screen.getByText(/ballotLocator\.found /)).toHaveTextContent(`"ballotId":"${FULL}"`)
        expect(screen.getByText("content of 30ffd314")).toBeVisible()
        expect(screen.queryByRole("region", {name: "ballotLocator.result.receipt"})).toBeNull()
        expect(screen.queryByText("ballotLocator.foundShort")).toBeNull()
    })

    it("says when no ballot has the ID", () => {
        renderAt(`/${FULL}`)
        expect(screen.getByText(/ballotLocator\.notFound/)).toBeVisible()
    })
})

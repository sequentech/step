/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The ballot tracker (design boards TrackerSearch, TrackerResult, PhoneTracker,
// TrackerAudit and PhoneTrackerAudit): the search, the list of ballots that
// share a short ID, the result in each state of the receipt's check, and the
// result for a ballot that was audited, not cast.

import React from "react"
import {render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import type {ChainResult, TierName, TierResult, TierStatus} from "@vitrine/verify"
import theme from "../../../ui-essentials/src/services/theme"
import {EReceiptNotice, IReceiptView} from "../services/vitrine/castReceipt"
import type {ITrackerReceipt} from "../services/vitrine/ballotTracker"
import {EAuditOutcome, TAudit} from "../services/vitrine/ballotAudit"
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
        copyBallotHash: jest.requireActual(
            "../../../ui-essentials/src/components/BallotHash/BallotHash"
        ).copyBallotHash,
        CopyBallotHashStatus: jest.requireActual(
            "../../../ui-essentials/src/components/BallotHash/BallotHash"
        ).CopyBallotHashStatus,
        PageLimit: ({children}: {children: React.ReactNode}) => <div>{children}</div>,
        InfoDataBox: ({children}: {children: React.ReactNode}) => <pre>{children}</pre>,
        BreadCrumbSteps: ({selected}: {selected: number}) => (
            <div data-testid="steps">{selected}</div>
        ),
        Icon: () => null,
        IconButton: () => null,
        checkIsInvalidVote: () => false,
        checkIsExplicitBlankVote: () => false,
        getImageUrl: () => undefined,
        Dialog: () => null,
        ExpandableText: () => null,
        Candidate: ({title, description}: {title: string; description?: React.ReactNode}) => (
            <li className="candidate-item">
                <span className="candidate-title">{title}</span>
                <span className="candidate-description">{description}</span>
            </li>
        ),
        BlankAnswer: ({title}: {title?: string}) => (
            <li className="blank-answer">{title ?? "candidate.blankVote"}</li>
        ),
    }),
    {virtual: true}
)
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {
            DISABLE_AUTH: false,
            BALLOT_VERIFIER_URL: "https://verifier.example/",
            PUBLIC_BUCKET_URL: "https://bucket.example/",
        },
    }),
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({isPreferential: () => false}),
}))
jest.mock("../store/hooks", () => ({
    useAppDispatch: () => jest.fn(),
    useAppSelector: () => undefined,
}))

const mockTracked = jest.fn()
jest.mock("../hooks/useTrackedReceipt", () => ({
    useTrackedReceipt: (...args: unknown[]) => mockTracked(...args),
}))

let mockShort: {status: string; result?: unknown} = {status: "off"}
jest.mock("../hooks/useShortIdMatches", () => ({
    useShortIdMatches: () => mockShort,
}))

const mockAudit = jest.fn()
jest.mock("../services/vitrine/ballotAudit", () => ({
    ...jest.requireActual("../services/vitrine/ballotAudit"),
    provideAuditService: () => ({}),
    auditBallot: (...args: unknown[]) => mockAudit(...args),
}))

interface ICastVote {
    ballot_id: string
    created_at: string
    content: string
}

let mockPresentation: Record<string, unknown> = {}
let mockTelephone = false
let mockVotes: ICastVote[] = []
const mockQueries: {name: string; variables: Record<string, unknown>; skip?: boolean}[] = []

/** What the ballot box answers to a lookup, from the voter's ballots it would match. */
const located = () =>
    mockVotes.length === 1
        ? {
              status: "found",
              ballot_id: mockVotes[0].ballot_id,
              content: mockVotes[0].content,
              cast_at: mockVotes[0].created_at,
              checks_available_until: null,
          }
        : {
              status: mockVotes.length ? "ambiguous" : "not-found",
              ballot_id: null,
              content: null,
              cast_at: null,
              checks_available_until: null,
          }

jest.mock("@apollo/client/react", () => ({
    useQuery: (
        query: {definitions: {kind: string; name?: {value: string}}[]},
        options: {variables: Record<string, unknown>; skip?: boolean}
    ) => {
        const name = query.definitions.find((d) => d.kind === "OperationDefinition")?.name?.value
        mockQueries.push({name: name ?? "", variables: options.variables, skip: options.skip})
        const data =
            name === "GetElectionEvent"
                ? {sequent_backend_election_event: [{id: "event", presentation: mockPresentation}]}
                : name === "GetElections"
                  ? {
                        sequent_backend_election: [
                            {id: "election", voting_channels: {telephone: mockTelephone}},
                        ],
                    }
                  : name === "LocateBallot"
                    ? {locate_ballot: located()}
                    : undefined
        return {data, loading: false, refetch: jest.fn()}
    },
}))

const FULL = "FTBEMHRX0123456789ABCDEFGHJKMNPQRSTVWXYZ0123456789AB"
const OTHER = "FTBEMHRX9C071BE27A50E3D946B108FCD25E6F13B7A4C9013E88"
const SHORT = "FTBEMHRX"
const GROUPED = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"
const NOW = 1_793_270_528
const TRACKER = {transparency: "public"}
const PATH = "/tenant/tenant/event/event/election/election/ballot-locator"

const vote = (
    ballotId: string,
    createdAt = "2026-10-29T10:42:08.000000+00:00",
    content = `content of ${ballotId.slice(0, 8)}`
): ICastVote => ({
    ballot_id: ballotId,
    created_at: createdAt,
    content,
})

const STYLE = {
    id: "council-style",
    contests: [
        {
            id: "mayor",
            name: "Mayor",
            counting_algorithm: "plurality-at-large",
            max_votes: 1,
            candidates: [
                {id: "ortega", name: "Maria Ortega", description: "Civic Alliance"},
                {id: "puig", name: "Joan Puig", description: "Harbour Party"},
            ],
        },
    ],
}

/** What the ballot box keeps of a cast ballot: its config is text. */
const CAST_CONTENT = JSON.stringify({
    version: 2,
    issue_date: "30/10/2026",
    contests: ["EQAAAGNvdW5jaWwtY29udGVzdC0x"],
    config: "council-style",
    ballot_style_hash: "1dce3614efd2cc5d0ddb8a1c47daf79b",
    voter_signing_pk: null,
    voter_ballot_signature: null,
})

/** What the Audit screen shows: the ballot style is in it, as an object. */
const auditedContent = (ballotHash = FULL, extra: Record<string, unknown> = {}) =>
    JSON.stringify({
        version: 2,
        issue_date: "30/10/2026",
        config: STYLE,
        contests: ["EQAAAGNvdW5jaWwtY29udGVzdC0x"],
        ballot_hash: ballotHash,
        voter_signing_pk: null,
        voter_ballot_signature: null,
        ...extra,
    })

const SELECTED: TAudit = {
    outcome: EAuditOutcome.VERIFIED,
    selections: [
        {
            contest_id: "mayor",
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
            invalid_errors: [],
            invalid_alerts: [],
            choices: [{id: "ortega", selected: 0}],
        },
    ],
}

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
    mockShort = {status: "off"}
    mockTracked.mockReset()
    mockTracked.mockReturnValue(tracked({view: VERIFIED}))
    mockAudit.mockReset()
    mockAudit.mockReturnValue({outcome: EAuditOutcome.NOT_VERIFIABLE})
})

describe("the search of an event without transparency", () => {
    it("groups the ID in fours as the voter types", async () => {
        renderAt()
        await userEvent.type(field(), "FTBEMHRX0123")
        expect(field().value).toBe("FTBE-MHRX-0123")
    })

    it("takes the short ID and the full one, with or without spaces", async () => {
        renderAt()
        expect(findButton()).toBeDisabled()
        await userEvent.type(field(), "FTBE MHR")
        expect(findButton()).toBeDisabled()
        await userEvent.type(field(), "4")
        expect(findButton()).toBeEnabled()
        await userEvent.type(field(), "4238")
        // The start of an ID takes 8 characters or more.
        expect(findButton()).toBeEnabled()
        await userEvent.clear(field())
        await userEvent.type(field(), "FTBE")
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
        await userEvent.type(field(), "FTBEU")
        expect(field().value).toBe("FTBE")
        expect(screen.getByRole("alert")).toHaveTextContent("ballotLocator.wrongFormatBallotId")
        expect(screen.queryByText("ballotLocator.idHelp")).toBeNull()
        await userEvent.type(field(), "d")
        expect(screen.getByRole("alert")).toBeEmptyDOMElement()
    })

    it("finds the ballot by the ID without its spaces", async () => {
        renderAt()
        await userEvent.type(field(), "FTBE-MHRX")
        await userEvent.click(findButton())
        expect(router.state.location.pathname).toBe(`${PATH}/${SHORT}`)
    })

    it("finds the ballot when the voter presses Enter, but only for an ID", async () => {
        renderAt()
        await userEvent.type(field(), "FTBE{Enter}")
        expect(router.state.location.pathname).toBe(PATH)
        await userEvent.type(field(), "MHRX{Enter}")
        expect(router.state.location.pathname).toBe(`${PATH}/${SHORT}`)
    })

    it("takes no fewer than eight characters, telephone voting or not", async () => {
        mockTelephone = true
        renderAt()
        await userEvent.type(field(), "FTBE")
        expect(findButton()).toBeDisabled()
    })

    it("keeps the classic locator: the field, a button to find, and Back below them", () => {
        renderAt()
        expect(screen.queryByRole("region", {name: "ballotLocator.search.title"})).toBeNull()
        const header = document.querySelector(".ballot-locator-header") as HTMLElement
        expect(within(header).queryByRole("link", {name: "votingScreen.backButton"})).toBeNull()
        expect(screen.getByRole("link", {name: "votingScreen.backButton"})).toBeVisible()
        expect(screen.getByText("ballotLocator.description")).toBeVisible()
    })
})

describe("the search of an event with transparency", () => {
    beforeEach(() => {
        mockPresentation = TRACKER
    })

    const find = () => screen.getByRole("button", {name: "ballotLocator.search.find"})
    const header = () => document.querySelector(".ballot-locator-header") as HTMLElement

    it("puts Back to the right of the title, and nothing but the search below", () => {
        renderAt()
        expect(within(header()).getByRole("heading", {level: 1})).toHaveTextContent(
            "ballotLocator.title"
        )
        const back = within(header()).getByRole("link", {name: "votingScreen.backButton"})
        expect(back).toHaveAttribute("href", "/tenant/tenant/event/event/election-chooser")
        expect(screen.getAllByRole("link", {name: "votingScreen.backButton"})).toHaveLength(1)
        expect(screen.queryByRole("button", {name: "ballotLocator.locate"})).toBeNull()
        expect(screen.queryByText("ballotLocator.description")).toBeNull()
        expect(screen.getByTestId("steps")).toHaveTextContent("0")
    })

    it("keeps the kiosk mode in the Back link", () => {
        renderAt("?kiosk=true")
        expect(
            within(header()).getByRole("link", {name: "votingScreen.backButton"})
        ).toHaveAttribute("href", "/tenant/tenant/event/event/election-chooser?kiosk=true")
    })

    it("asks the voter to check a ballot in the public record, and says what the tracker shows", () => {
        renderAt()
        const card = screen.getByRole("region", {name: "ballotLocator.search.title"})
        expect(within(card).getByText("ballotLocator.search.intro")).toBeVisible()
        for (const feature of ["cast", "audited", "secret"]) {
            expect(screen.getByText(`ballotLocator.search.features.${feature}.title`)).toBeVisible()
            expect(screen.getByText(`ballotLocator.search.features.${feature}.text`)).toBeVisible()
        }
    })

    it("labels the field with the Tracking ID, and groups the ID in fours as the voter types", async () => {
        renderAt()
        expect(field()).toHaveAttribute("placeholder", "FTBE-MHRX")
        await userEvent.type(field(), "FTBEMHRX0123")
        expect(field().value).toBe("FTBE-MHRX-0123")
    })

    it("takes the short ID and the full one, with or without spaces", async () => {
        renderAt()
        expect(find()).toBeDisabled()
        await userEvent.type(field(), "FTBE MHR")
        expect(find()).toBeDisabled()
        await userEvent.type(field(), "4")
        expect(find()).toBeEnabled()
        await userEvent.type(field(), "4238")
        // The start of an ID takes 8 characters or more.
        expect(find()).toBeEnabled()
        await userEvent.clear(field())
        await userEvent.type(field(), "FTBE")
        expect(find()).toBeDisabled()
        await userEvent.clear(field())
        await userEvent.paste(GROUPED)
        expect(field().value).toBe(GROUPED)
        expect(find()).toBeEnabled()
    })

    it("shows the formats the field takes, and says when a character can't be part of the ID", async () => {
        renderAt()
        expect(field()).toHaveAttribute("aria-invalid", "false")
        const help = document.getElementById(field().getAttribute("aria-describedby") ?? "")
        for (const format of ["short", "full", "spaces"]) {
            expect(help).toHaveTextContent(`ballotLocator.search.chips.${format}`)
        }
        expect(screen.getByRole("alert")).toBeEmptyDOMElement()

        await userEvent.type(field(), "FTBEU")
        expect(field().value).toBe("FTBE")
        expect(field()).toHaveAttribute("aria-invalid", "true")
        const error = screen.getByRole("alert")
        expect(error).toHaveTextContent("ballotLocator.search.invalid")
        expect(field()).toHaveAttribute("aria-describedby", error.id)
        expect(screen.queryByText(/ballotLocator\.search\.chips\./)).toBeNull()

        await userEvent.type(field(), "d")
        expect(screen.getByRole("alert")).toBeEmptyDOMElement()
        expect(field()).toHaveAttribute("aria-invalid", "false")
        expect(screen.getByText(/ballotLocator\.search\.chips\.short/)).toBeVisible()
    })

    it("finds the ballot with the button, or when the voter presses Enter, but only for an ID", async () => {
        renderAt()
        await userEvent.type(field(), "FTBE{Enter}")
        expect(router.state.location.pathname).toBe(PATH)
        await userEvent.type(field(), "MHRX{Enter}")
        expect(router.state.location.pathname).toBe(`${PATH}/${SHORT}`)
    })

    it("finds the ballot with the button", async () => {
        renderAt()
        await userEvent.type(field(), "FTBE-MHRX")
        await userEvent.click(find())
        expect(router.state.location.pathname).toBe(`${PATH}/${SHORT}`)
    })

    it("takes no fewer than eight characters, telephone voting or not", async () => {
        mockTelephone = true
        renderAt()
        await userEvent.type(field(), "FTBE")
        expect(find()).toBeDisabled()
    })

    it("names its tab after the tracking, and keeps Back below the logs, which have no title row", async () => {
        mockPresentation = {...TRACKER, show_cast_vote_logs: "show-logs-tab"}
        renderAt()
        expect(screen.getByRole("tab", {name: "ballotLocator.tabs.ballotLocator"})).toBeVisible()
        expect(screen.getAllByRole("link", {name: "votingScreen.backButton"})).toHaveLength(1)
        await userEvent.click(screen.getByRole("tab", {name: "ballotLocator.tabs.logs"}))
        const back = screen.getByRole("link", {name: "votingScreen.backButton"})
        expect(back).toHaveAttribute("href", "/tenant/tenant/event/event/election-chooser")
        expect(back.closest(".ballot-locator-header")).toBeNull()
    })
})

describe("ballots that the ID does not tell apart", () => {
    it("asks for more of the Tracking ID instead of choosing one", () => {
        mockPresentation = TRACKER
        mockVotes = [vote(FULL), vote(OTHER)]
        renderAt(`/${SHORT}`)
        expect(screen.getByText(/ballotLocator\.ambiguous/)).toBeVisible()
        expect(screen.queryByText(/ballotLocator\.found /)).toBeNull()
        expect(screen.getByRole("button", {name: "ballotLocator.search.find"})).toBeEnabled()
    })
})

describe("ballots that share the short ID, listed from a bucket of the record's servers", () => {
    const listing = (extra: Record<string, unknown>) => ({
        kind: "listed",
        typed: SHORT,
        matches: [],
        total: 0,
        truncated: false,
        ...extra,
    })
    const row = (third: string, extra: Record<string, unknown> = {}) => ({
        ballotId: "ab".repeat(32),
        trackingId: `${SHORT}${third}5ZBD${"0".repeat(36)}`,
        third,
        fourth: "5ZBD",
        audited: false,
        at: 1_793_270_528,
        event: "event",
        election: "election",
        ...extra,
    })
    const locates = () =>
        mockQueries.filter((query) => query.name === "LocateBallot" && !query.skip)

    beforeEach(() => {
        mockPresentation = TRACKER
    })

    it("lists them without asking the election's server which ballot it is", () => {
        mockShort = {
            status: "done",
            result: listing({matches: [row("71J7"), row("0WDY")], total: 2}),
        }
        renderAt(`/${SHORT}`)
        const links = within(
            document.querySelector(".short-id-matches") as HTMLElement
        ).getAllByRole("link")
        expect(links.map((link) => link.getAttribute("href"))).toEqual([
            `${PATH}/${SHORT}71J75ZBD${"0".repeat(36)}`,
            `${PATH}/${SHORT}0WDY5ZBD${"0".repeat(36)}`,
        ])
        expect(locates()).toHaveLength(0)
        expect(screen.queryByText("ballotLocator.shortId.serverSees")).toBeNull()
        expect(screen.queryByText(/ballotLocator\.notFound/)).toBeNull()
        expect(screen.getByTestId("steps")).toHaveTextContent("0")
    })

    it("asks only for the third group when many share it", () => {
        mockShort = {status: "done", result: listing({kind: "many", total: 28})}
        renderAt(`/${SHORT}`)
        expect(screen.queryAllByRole("link", {name: /FTBE-MHRX/})).toHaveLength(0)
        expect(screen.getByLabelText("ballotLocator.shortId.third")).toBeVisible()
    })

    it("narrows by the third group the voter types", async () => {
        mockShort = {status: "done", result: listing({kind: "many", total: 28})}
        renderAt(`/${SHORT}`)
        await userEvent.type(screen.getByLabelText("ballotLocator.shortId.third"), "71J7")
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.shortId.find"}))
        expect(router.state.location.pathname).toBe(`${PATH}/${SHORT}71J7`)
        expect(field().value).toBe("FTBE-MHRX-71J7")
    })

    it("waits for the bucket before asking the election's server", () => {
        mockShort = {status: "loading"}
        renderAt(`/${SHORT}`)
        expect(locates()).toHaveLength(0)
        expect(screen.queryByText(/ballotLocator\.notFound/)).toBeNull()
    })

    it("falls back to the election's server when no server lists buckets, and says it saw the ID", () => {
        mockShort = {status: "done", result: listing({kind: "unsupported"})}
        mockVotes = [vote(FULL)]
        renderAt(`/${SHORT}`)
        expect(screen.getByText("ballotLocator.shortId.serverSees")).toBeVisible()
        expect(locates().some((query) => query.variables.ballotId === SHORT)).toBe(true)
    })

    it("says the same when the event has no policy to name a server", () => {
        mockShort = {status: "done"}
        mockVotes = [vote(FULL)]
        renderAt(`/${SHORT}`)
        expect(screen.getByText("ballotLocator.shortId.serverSees")).toBeVisible()
    })

    it("says nothing about it where no bucket was tried", () => {
        mockVotes = [vote(FULL)]
        renderAt(`/${SHORT}`)
        expect(screen.queryByText("ballotLocator.shortId.serverSees")).toBeNull()
    })
})

describe("the result for an event with transparency", () => {
    beforeEach(() => {
        mockPresentation = TRACKER
        mockVotes = [vote(FULL)]
    })

    const section = () => screen.getByRole("region", {name: "ballotLocator.result.receipt"})
    const header = () => document.querySelector(".ballot-locator-header") as HTMLElement
    const tagsOf = (element: Element) =>
        Array.from(element.querySelectorAll(".status-tag")).map((tag) => [
            tag.textContent,
            tag.getAttribute("data-tone"),
        ])

    it("names the ballot by its short ID, for assistive technology only, and shows both IDs", () => {
        renderAt(`/${FULL}`)
        expect(screen.getByTestId("steps")).toHaveTextContent("1")
        const found = screen.getByText(/ballotLocator\.found /)
        expect(found).toHaveTextContent('"ballotId":"FTBE-MHRX"')
        expect(found.closest("[role=status]")).not.toBeNull()
        expect(document.querySelector(".ballot-locator-success")).toBeNull()
        expect(screen.queryByText("ballotLocator.foundShort")).toBeNull()
        const ids = within(section())
        expect(ids.getByText("a11y.ballotIdLabel")).toBeVisible()
        expect(ids.getByText("FTBE-MHRX", {selector: ".tracker-short-id"})).toBeVisible()
        expect(ids.getByText("ballotLocator.result.idComment")).toBeVisible()
        const full = section().querySelector(".full-tracking-id-value")
        expect(full?.textContent).toBe(GROUPED)
        expect(
            Array.from(full?.querySelectorAll(".full-tracking-id-short") ?? []).map(
                (group) => group.textContent
            )
        ).toEqual(["FTBE-", "MHRX-"])
        expect(section().querySelector(".full-tracking-id")?.getAttribute("data-surface")).toBe(
            "tint"
        )
        expect(ids.getByText("b1d8 c475 6132 ··· 5a0d")).toBeVisible()
        expect(ids.getByRole("button", {name: "ballotLocator.result.save"})).toBeVisible()
    })

    it("tags the ballot as cast and found, under its short ID", () => {
        renderAt(`/${FULL}`)
        expect(tagsOf(section())).toEqual([
            ["vitrine.status.tag.cast", "success"],
            ["ballotLocator.result.tag.found", "success"],
        ])
    })

    it("tags a ballot whose content can't be read as cast, as the portal lists it", () => {
        mockVotes = [vote(FULL, undefined, '{"truncated": ')]
        renderAt(`/${FULL}`)
        expect(tagsOf(section())[0]).toEqual(["vitrine.status.tag.cast", "success"])
        expect(mockAudit).not.toHaveBeenCalled()
    })

    it("puts Back and Find another Ballot beside the title, and nothing below the result", async () => {
        renderAt(`/${FULL}`)
        const top = within(header())
        expect(top.getByRole("heading", {level: 1})).toHaveTextContent("ballotLocator.title")
        expect(top.getByRole("link", {name: "votingScreen.backButton"})).toBeVisible()
        expect(screen.getAllByRole("link", {name: "votingScreen.backButton"})).toHaveLength(1)
        expect(screen.getAllByRole("button", {name: "ballotLocator.locateAgain"})).toHaveLength(1)
        expect(screen.getByText("ballotLocator.description")).toBeVisible()
        await userEvent.click(top.getByRole("button", {name: "ballotLocator.locateAgain"}))
        expect(router.state.location.pathname).toBe(`${PATH}/`)
        expect(screen.getByRole("button", {name: "ballotLocator.search.find"})).toBeDisabled()
    })

    it("keeps the kiosk mode when it goes back or searches again", async () => {
        renderAt(`/${FULL}?kiosk=true`)
        expect(
            within(header()).getByRole("link", {name: "votingScreen.backButton"})
        ).toHaveAttribute("href", "/tenant/tenant/event/event/election-chooser?kiosk=true")
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.locateAgain"}))
        expect(router.state.location.search).toBe("?kiosk=true")
    })

    it("tells the voter who found the ballot by its short ID to compare the full one", () => {
        renderAt(`/${SHORT}`)
        expect(screen.getByText("ballotLocator.foundShort")).toBeVisible()
        expect(mockTracked.mock.calls.at(-1)?.[0]).toEqual({
            eventId: "event",
            electionId: "election",
            ballotId: FULL,
            stage: "cast",
            acceptedAt: Date.UTC(2026, 9, 29, 10, 42, 8) / 1000,
        })
    })

    it("copies the full ID, grouped, and swaps the copy icon for a check and the word Copied", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        Object.defineProperty(navigator, "clipboard", {value: {writeText}, configurable: true})
        renderAt(`/${FULL}`)
        const copy = screen.getByRole("button", {name: "vitrine.ballotId.copy"})
        expect(copy).toHaveTextContent("vitrine.ballotId.copyText")
        await userEvent.click(copy)
        expect(writeText).toHaveBeenCalledWith(GROUPED)
        const copied = screen.getByRole("button", {name: "vitrine.ballotId.copied"})
        expect(copied).toBe(copy)
        expect(copied).toHaveTextContent("vitrine.ballotId.copiedText")
        expect(copied.querySelector(".full-tracking-id-copied-icon")).not.toBeNull()
    })

    it("lists the explorers of the policy under the result, each opening this ballot", async () => {
        const policy = {
            id: "sha256:abc123",
            operator: "Election authority",
            monitors: new Map([
                ["obs", {name: "obs", party: "Observer", explorer: "https://obs.example/"}],
            ]),
            hubs: new Map(),
            explorers: ["https://election.example/"],
        } as unknown as NonNullable<IReceiptView["policy"]>
        mockTracked.mockReturnValue(tracked({view: {...VERIFIED, policy}}))
        renderAt(`/${FULL}`)
        const toggle = within(section()).getByRole("button", {name: /vitrine.elsewhere.title/})
        await userEvent.click(toggle)
        const links = Array.from(section().querySelectorAll<HTMLAnchorElement>(".find-elsewhere a"))
        expect(links.map((a) => a.getAttribute("href"))).toEqual([
            `https://obs.example/#policy=sha256%3Aabc123&log=ballots&find=${GROUPED}`,
            `https://election.example/#policy=sha256%3Aabc123&log=ballots&find=${GROUPED}`,
        ])
        expect(links.every((a) => a.target === "_blank" && a.rel === "noopener noreferrer")).toBe(
            true
        )
    })

    it("shows no explorers when the policy names none", () => {
        renderAt(`/${FULL}`)
        expect(section().querySelector(".find-elsewhere")).toBeNull()
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
        mockTracked.mockReturnValue(tracked({settled: false}))
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.status.checking.head")).toBeVisible()
        expect(section().querySelector(".tracker-step-record")).toHaveClass("tracker-step-active")
        expect(screen.queryByRole("button", {name: "ballotLocator.result.save"})).toBeNull()
    })

    it("says honestly that no receipt could be checked, and shows no status, but the content", () => {
        mockTracked.mockReturnValue(tracked({settled: true}))
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.noReceipt")).toBeVisible()
        expect(screen.queryByRole("img", {name: /sealLabel/})).toBeNull()
        expect(screen.queryByText(/ballotLocator\.result\.status\./)).toBeNull()
        expect(section().querySelectorAll(".tracker-step")).toHaveLength(0)
        expect(screen.getByText("FTBE-MHRX", {selector: ".tracker-short-id"})).toBeVisible()
        expect(screen.getByText(/ballotLocator\.found /)).toBeVisible()
        expect(
            within(section()).getByRole("region", {name: "ballotLocator.result.record.title"})
        ).toBeVisible()
    })

    it("says the same when the receipt the log gave couldn't be completed or checked", () => {
        mockTracked.mockReturnValue(
            tracked({view: {notice: EReceiptNotice.WAITING} as IReceiptView, settled: true})
        )
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.noReceipt")).toBeVisible()
    })

    it("says when no ballot has the ID, with the buttons beside the title", () => {
        mockVotes = []
        renderAt(`/${FULL}`)
        expect(screen.getByText(/ballotLocator\.notFound/)).toBeVisible()
        const top = within(header())
        expect(top.getByRole("link", {name: "votingScreen.backButton"})).toBeVisible()
        expect(top.getByRole("button", {name: "ballotLocator.locateAgain"})).toBeVisible()
        expect(screen.queryByRole("region", {name: "ballotLocator.result.receipt"})).toBeNull()
    })
})

describe("Who signed and Which hubs in the result for an event with transparency", () => {
    const SIGNED_AT = NOW - 600
    const POLICY = {
        id: "sha256:abc123",
        operator: "Election authority",
        browserPeriod: 600,
        witnesses: {
            witnesses: new Map([
                ["w1", {name: "w1", party: "Observer A"}],
                ["w2", {name: "w2", party: "Observer B"}],
            ]),
            groups: new Map([["quorum", {k: 2, members: ["w1", "w2"]}]]),
            quorum: "quorum",
        },
        monitors: new Map([
            ["obs", {name: "obs", party: "Observer A", explorer: "https://obs.example/"}],
        ]),
        hubs: new Map([
            ["hub-a", {name: "hub-a", operator: "Hub A", explorer: "https://hub-a.example/"}],
            ["hub-b", {name: "hub-b", operator: "Election authority"}],
        ]),
        explorers: [],
        anchorHubs: {min: 2, minIndependent: 1},
    } as unknown as NonNullable<IReceiptView["policy"]>

    const signers = {
        witnesses: new Map([
            ["w1", SIGNED_AT],
            ["w2", SIGNED_AT + 1],
        ]),
        hubs: new Map([["hub-a", SIGNED_AT + 2]]),
        size: 22,
    }

    const hubTiers = [
        {hub: "hub-a", status: "verified"},
        {hub: "hub-b", status: "late"},
    ]

    const withSigners = (
        status: ChainResult["status"],
        tiers: TierResult[],
        extra: Partial<IReceiptView> = {}
    ): IReceiptView => ({
        ...viewOf({
            ...resultOf(status, tiers),
            policy: {hubs: ["hub-a", "hub-b"], browser: "bootstrap"},
        } as unknown as ChainResult),
        policy: POLICY,
        signers,
        ...extra,
    })

    const step = (name: string) => document.querySelector(`.tracker-step-${name}`) as HTMLElement
    const toggleOf = (name: string) =>
        within(step(name)).queryByRole("button", {name: /vitrine\.signers\./})

    beforeEach(() => {
        mockPresentation = TRACKER
        mockVotes = [vote(FULL)]
    })

    it("opens a list of the witnesses under the step they confirmed, and of the hubs under theirs", () => {
        mockTracked.mockReturnValue(
            tracked({
                view: withSigners("verified", [
                    tier("record", "verified"),
                    tier("witnesses", "verified"),
                    tier("hubs", "verified", {hubs: hubTiers} as unknown as Partial<TierResult>),
                    tier("browser", "verified"),
                ]),
            })
        )
        renderAt(`/${FULL}`)
        expect(toggleOf("record")).toBeNull()
        expect(toggleOf("witnesses")).toHaveTextContent("vitrine.signers.witnesses.open")
        expect(toggleOf("hubs")).toHaveTextContent("vitrine.signers.hubs.open")
        expect(toggleOf("browser")).toBeNull()
    })

    it("lists the witnesses with when each signed, and a link only where the policy names a site", async () => {
        mockTracked.mockReturnValue(
            tracked({
                view: withSigners("pending", [
                    tier("record", "verified"),
                    tier("witnesses", "verified"),
                    tier("hubs", "pending"),
                    tier("browser", "pending"),
                ]),
            })
        )
        renderAt(`/${FULL}`)
        await userEvent.click(toggleOf("witnesses")!)
        const rows = Array.from(step("witnesses").querySelectorAll(".witness-list-row"))
        expect(rows.map((row) => row.querySelector(".witness-list-name")?.textContent)).toEqual([
            "w1",
            "w2",
        ])
        expect(
            Array.from(step("witnesses").querySelectorAll<HTMLAnchorElement>("a")).map((a) =>
                a.getAttribute("href")
            )
        ).toEqual([
            "https://obs.example/#policy=sha256%3Aabc123&log=ballots&checkpoint=22&cosigner=w1",
        ])
    })

    it("lists the hubs, the late one too, and leaves out where browsers check", async () => {
        mockTracked.mockReturnValue(
            tracked({
                view: withSigners("late", [
                    tier("record", "verified"),
                    tier("witnesses", "verified"),
                    tier("hubs", "late", {
                        deadline: NOW - 60,
                        hubs: hubTiers,
                    } as unknown as Partial<TierResult>),
                    tier("browser", "pending"),
                ]),
            })
        )
        renderAt(`/${FULL}`)
        await userEvent.click(toggleOf("hubs")!)
        const rows = Array.from(step("hubs").querySelectorAll(".witness-list-row"))
        expect(rows.map((row) => row.querySelector(".witness-list-name")?.textContent)).toEqual([
            "hub-a",
            "hub-b",
        ])
        expect(rows[1].querySelector(".witness-list-when")?.textContent).toBe(
            "vitrine.signers.late"
        )
    })

    it("offers no list before the witnesses have signed, or when the receipt gave no times", () => {
        mockTracked.mockReturnValue(
            tracked({
                view: withSigners("pending", [
                    tier("record", "verified"),
                    tier("witnesses", "pending"),
                    tier("hubs", "pending"),
                    tier("browser", "pending"),
                ]),
            })
        )
        const {unmount} = renderAt(`/${FULL}`)
        expect(toggleOf("witnesses")).toBeNull()
        unmount()
        mockTracked.mockReturnValue(
            tracked({
                view: withSigners(
                    "verified",
                    [
                        tier("record", "verified"),
                        tier("witnesses", "verified"),
                        tier("hubs", "verified"),
                        tier("browser", "verified"),
                    ],
                    {signers: undefined}
                ),
            })
        )
        renderAt(`/${FULL}`)
        expect(toggleOf("witnesses")).toBeNull()
        expect(toggleOf("hubs")).toBeNull()
    })
})

describe("the record in the result for an event with transparency", () => {
    beforeEach(() => {
        mockPresentation = TRACKER
    })

    const viewer = () =>
        screen.getByRole("region", {name: "ballotLocator.result.record.title"}) as HTMLElement

    it("is in the record viewer, with what the record is, as the ballot box keeps it", () => {
        mockVotes = [vote(FULL, undefined, CAST_CONTENT)]
        renderAt(`/${FULL}`)
        expect(viewer()).toHaveTextContent("ballotLocator.result.record.title")
        expect(viewer()).toHaveTextContent(
            /ballotLocator\.result\.record\.subtitle · vitrine\.record\.json · vitrine\.record\.fields \{"count":7\} · vitrine\.record\.size\.bytes/
        )
        expect(viewer().querySelector(".record-viewer-preview")).toHaveTextContent(
            '{"version":2,"issue_date":"30/10/2026"'
        )
        expect(screen.queryByText("ballotLocator.contentDesc")).toBeNull()
    })

    it("is named after the short ID when it is saved", () => {
        mockVotes = [vote(FULL, undefined, CAST_CONTENT)]
        renderAt(`/${FULL}`)
        expect(
            within(viewer()).getByRole("button", {
                name: 'vitrine.record.download {"file":"ballot-FTBEMHRX.json"}',
            })
        ).toBeVisible()
    })

    it("is shown as text when it isn't JSON", () => {
        mockVotes = [vote(FULL, undefined, "a ballot, as plain text")]
        renderAt(`/${FULL}`)
        expect(viewer()).toHaveTextContent("vitrine.record.text")
        expect(within(viewer()).getByText("a ballot, as plain text")).toBeVisible()
    })

    it("keeps the class that custom styles of the content box know", () => {
        mockVotes = [vote(FULL)]
        renderAt(`/${FULL}`)
        expect(viewer()).toHaveClass("ballot-content")
    })
})

describe("the result for a ballot that was audited", () => {
    beforeEach(() => {
        mockPresentation = TRACKER
        mockVotes = [vote(FULL, undefined, auditedContent())]
        mockAudit.mockReturnValue(SELECTED)
    })

    const section = () => screen.getByRole("region", {name: "ballotLocator.result.receipt"})
    const tagsOf = (element: Element) =>
        Array.from(element.querySelectorAll(".status-tag")).map((tag) => [
            tag.textContent,
            tag.getAttribute("data-tone"),
        ])

    it("checks the ballot in the record against the Tracking ID that found it", () => {
        renderAt(`/${FULL}`)
        expect(mockAudit).toHaveBeenCalledWith(
            {},
            expect.objectContaining({kind: "audited", encoding: "single-contest"}),
            FULL
        )
    })

    it("tags the ballot as auditable and found, and says it was never cast", () => {
        renderAt(`/${FULL}`)
        expect(tagsOf(section())).toEqual([
            ["vitrine.status.tag.auditable", "success"],
            ["ballotLocator.result.tag.found", "success"],
        ])
        expect(screen.getByText("ballotLocator.descriptionAudited")).toBeVisible()
        expect(screen.queryByText("ballotLocator.description")).toBeNull()
    })

    it("shows the selections it decodes into, under the record", () => {
        renderAt(`/${FULL}`)
        const card = screen.getByRole("region", {name: "ballotLocator.audit.title"})
        expect(within(card).getByRole("heading", {name: "Mayor", level: 3})).toBeVisible()
        expect(within(card).getByText("Maria Ortega")).toBeVisible()
        expect(within(card).queryByText("Joan Puig")).toBeNull()
        expect(
            section().compareDocumentPosition(card) & Node.DOCUMENT_POSITION_FOLLOWING
        ).toBeTruthy()
    })

    it("keeps the decoded selections in memory: nothing is stored, and nothing is sent", () => {
        localStorage.clear()
        sessionStorage.clear()
        const sent = jest.spyOn(global, "fetch").mockResolvedValue(new Response("{}"))
        renderAt(`/${FULL}`)
        expect(screen.getByText("Maria Ortega")).toBeVisible()
        expect(localStorage).toHaveLength(0)
        expect(sessionStorage).toHaveLength(0)
        expect(sent).not.toHaveBeenCalled()
        sent.mockRestore()
    })

    it("keeps the rest of the result as it is for a cast ballot", () => {
        renderAt(`/${FULL}`)
        expect(screen.getByText("ballotLocator.result.status.verified.head")).toBeVisible()
        expect(section().querySelectorAll(".tracker-step")).toHaveLength(4)
        expect(
            within(section()).getByRole("button", {name: "ballotLocator.result.save"})
        ).toBeVisible()
        expect(
            within(section()).getByRole("region", {name: "ballotLocator.result.record.title"})
        ).toBeVisible()
    })

    it("is told of a failed audit, with a danger tag, and shows no selections", () => {
        mockAudit.mockReturnValue({outcome: EAuditOutcome.MISMATCH})
        renderAt(`/${FULL}`)
        expect(tagsOf(section())).toEqual([
            ["ballotLocator.result.tag.auditFailed", "danger"],
            ["ballotLocator.result.tag.found", "success"],
        ])
        expect(screen.getByRole("alert")).toHaveTextContent("ballotLocator.audit.failed.text")
        expect(screen.getByText("ballotLocator.descriptionAuditedBrief")).toBeVisible()
        expect(screen.queryByText("Maria Ortega")).toBeNull()
        expect(screen.queryByRole("region", {name: "ballotLocator.audit.title"})).toBeNull()
    })

    it("says, neutrally, that the audit could not be checked, and shows no selections", () => {
        mockAudit.mockReturnValue({outcome: EAuditOutcome.NOT_VERIFIABLE})
        renderAt(`/${FULL}`)
        expect(tagsOf(section())).toEqual([
            ["vitrine.status.tag.auditable", "neutral"],
            ["ballotLocator.result.tag.found", "success"],
        ])
        expect(screen.queryByRole("alert")).toBeNull()
        expect(
            screen
                .getAllByRole("status")
                .some((status) =>
                    status.textContent?.includes("ballotLocator.audit.unchecked.text")
                )
        ).toBe(true)
        expect(screen.getByText("ballotLocator.descriptionAuditedBrief")).toBeVisible()
        expect(screen.queryByText("Maria Ortega")).toBeNull()
    })

    it("does not audit a cast ballot, nor one that can't be read", () => {
        mockVotes = [vote(FULL, undefined, CAST_CONTENT)]
        renderAt(`/${FULL}`)
        expect(mockAudit).not.toHaveBeenCalled()
        expect(screen.queryByRole("region", {name: "ballotLocator.audit.title"})).toBeNull()
    })

    it("shows an audited ballot as a ballot in the classic result too, without checking it", () => {
        mockPresentation = {}
        renderAt(`/${FULL}`)
        expect(mockAudit).not.toHaveBeenCalled()
        expect(screen.getByText("ballotLocator.contentDesc")).toBeVisible()
        expect(screen.queryByRole("region", {name: "ballotLocator.audit.title"})).toBeNull()
    })
})

describe("the result for an event without transparency", () => {
    it("keeps the original result", () => {
        mockVotes = [vote(FULL)]
        renderAt(`/${FULL}`)
        expect(screen.getByText(/ballotLocator\.found /)).toHaveTextContent(`"ballotId":"${FULL}"`)
        expect(screen.getByText("content of FTBEMHRX")).toBeVisible()
        expect(screen.queryByRole("region", {name: "ballotLocator.result.receipt"})).toBeNull()
        expect(screen.queryByText("ballotLocator.foundShort")).toBeNull()
        expect(screen.queryByRole("region", {name: "ballotLocator.result.record.title"})).toBeNull()
    })

    it("keeps its buttons below the result", async () => {
        mockVotes = [vote(FULL)]
        renderAt(`/${FULL}`)
        const header = document.querySelector(".ballot-locator-header") as HTMLElement
        expect(within(header).queryByRole("link", {name: "votingScreen.backButton"})).toBeNull()
        expect(within(header).queryByRole("button", {name: "ballotLocator.locateAgain"})).toBeNull()
        await userEvent.click(screen.getByRole("button", {name: "ballotLocator.locateAgain"}))
        expect(router.state.location.pathname).toBe(`${PATH}/`)
        expect(screen.getByRole("link", {name: "votingScreen.backButton"})).toBeVisible()
    })

    it("says when no ballot has the ID", () => {
        renderAt(`/${FULL}`)
        expect(screen.getByText(/ballotLocator\.notFound/)).toBeVisible()
    })
})

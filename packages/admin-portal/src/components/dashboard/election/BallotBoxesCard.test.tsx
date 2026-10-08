/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {act, fireEvent, render, screen, waitFor} from "@testing-library/react"
import {BallotBoxesCard, EBallotBoxRowStatus, ballotBoxRowStatus} from "./BallotBoxesCard"
import {
    EBallotBoxClosedByKind,
    EBallotBoxSealStatus,
    type IBallotBoxSeal,
} from "@/types/ballotBoxSeal"

let mockSeals: IBallotBoxSeal[] = []
let mockStyleAreas: string[] = []
let mockError: Error | undefined
let mockPolicy = "seal-at-close"
let mockCanDownload = true
const mockFetchDocument = jest.fn()
const mockDownloadUrl = jest.fn()

jest.mock("@apollo/client", () => ({
    gql: (parts: TemplateStringsArray) => parts.join(""),
    useLazyQuery: () => [mockFetchDocument, {loading: false}],
    useQuery: (query: string, options?: {skip?: boolean}) => {
        if (options?.skip) return {data: undefined}
        if (query.includes("GetBallotBoxSeals")) {
            return {
                data: mockError ? undefined : {sequent_backend_ballot_box_seal: mockSeals},
                error: mockError,
            }
        }
        if (query.includes("GetBallotBoxAreaNames")) {
            return {
                data: {
                    sequent_backend_area: mockStyleAreas.map((id) => ({id, name: `Area ${id}`})),
                },
            }
        }
        return {
            data: {sequent_backend_ballot_style: mockStyleAreas.map((area_id) => ({area_id}))},
        }
    },
}))
jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, options?: Record<string, unknown>) =>
            options ? `${key} ${JSON.stringify(options)}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...require("../../../../../ui-core/src/types/ElectionEventPresentation"),
    ...require("../../../../../ui-core/src/types/CoreTypes"),
    downloadUrl: (...args: unknown[]) => mockDownloadUrl(...args),
}))
jest.mock("@/providers/AuthContextProvider", () => ({
    AuthContext: require("react").createContext({
        isAuthorized: (_: boolean, __: string, permission: string) =>
            permission !== "document-download" || mockCanDownload,
    }),
}))
jest.mock("@/providers/TenantContextProvider", () => ({
    useTenantStore: () => ["tenant"],
}))
jest.mock("@/types/keycloak", () => ({
    IPermissions: {DOCUMENT_DOWNLOAD: "document-download"},
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({BallotHashCopyButton: () => null, theme: {palette: {customGrey: {main: "grey"}}}}),
    {virtual: true}
)
jest.mock("../charts/Charts", () => ({
    __esModule: true,
    default: ({title, children}: {title: string; children: React.ReactNode}) =>
        require("react").createElement("section", {"aria-label": title}, children),
}))
jest.mock("@/hooks/useZonedFormat", () => ({
    useEventPresentation: () => ({ballot_box_seal_policy: mockPolicy}),
}))
jest.mock("@/hooks/useZonedTime", () => ({
    intlLanguage: (lang: string) => lang,
    useZonedTime: () => (value?: string | null) => value ?? "",
}))
jest.mock("@/components/signing/format", () => ({
    formatList: (names: string[]) => names.join(" and "),
    shortHash: (hash: string) => hash.slice(0, 8),
}))
jest.mock("@/providers/SettingsContextProvider", () => ({
    SettingsContext: require("react").createContext({
        globalSettings: {QUERY_POLL_INTERVAL_MS: 1000, PUBLIC_BUCKET_URL: "https://public/"},
    }),
}))

const CLOSED = "2028-03-13T17:00:00Z"
const DEADLINE = "2028-03-13T17:15:00Z"
const seal = (status: EBallotBoxSealStatus, overrides: Partial<IBallotBoxSeal> = {}) =>
    ({
        id: "seal",
        election_id: "election",
        area_id: "area",
        area: {id: "area", name: "Madrid office"},
        status,
        closed_at: CLOSED,
        grace_deadline: DEADLINE,
        closed_by: {kind: EBallotBoxClosedByKind.USER, username: "admin"},
        ...overrides,
    }) as IBallotBoxSeal

beforeEach(() => {
    mockSeals = []
    mockStyleAreas = []
    mockError = undefined
    mockPolicy = "seal-at-close"
    mockCanDownload = true
    mockFetchDocument.mockReset()
    mockDownloadUrl.mockReset()
})
afterEach(() => jest.useRealTimers())

const renderCard = (now?: Date, election?: {status?: unknown; voting_channels?: unknown}) =>
    render(
        <BallotBoxesCard
            electionEventId="event"
            electionId="election"
            election={election}
            now={now}
        />
    )

const at = (iso: string) => new Date(iso)

describe("ballotBoxRowStatus", () => {
    const before = at("2028-03-13T17:05:00Z")
    const justAfter = at("2028-03-13T17:15:30Z")
    const later = at("2028-03-13T17:30:00Z")
    it.each([
        [EBallotBoxSealStatus.PENDING, before, EBallotBoxRowStatus.SEALING],
        [EBallotBoxSealStatus.PENDING, justAfter, EBallotBoxRowStatus.DUE],
        [EBallotBoxSealStatus.PENDING, later, EBallotBoxRowStatus.OVERDUE],
        [EBallotBoxSealStatus.SEALED, later, EBallotBoxRowStatus.PUBLISHING],
        [EBallotBoxSealStatus.PUBLISHED, later, EBallotBoxRowStatus.SEALED],
        [EBallotBoxSealStatus.FAILED, later, EBallotBoxRowStatus.FAILED],
    ])("shows %s at %s as %s", (status, now, expected) => {
        expect(ballotBoxRowStatus(seal(status), now)).toBe(expected)
    })
    it("shows a box without a seal row as open", () => {
        expect(ballotBoxRowStatus(undefined, before)).toBe(EBallotBoxRowStatus.OPEN)
    })
    it("is overdue at once when the sealer says what holds it", () => {
        for (const reason of ["channel_open:KIOSK", "datafix_votes:2", "error:vault"]) {
            expect(
                ballotBoxRowStatus(
                    seal(EBallotBoxSealStatus.PENDING, {
                        waiting_reason: reason,
                        last_attempt_at: "2028-03-13T17:15:10Z",
                    }),
                    justAfter
                )
            ).toBe(EBallotBoxRowStatus.OVERDUE)
        }
    })
    it("is due while the sealer tried recently, overdue once its last attempt is stale", () => {
        const busy = seal(EBallotBoxSealStatus.PENDING, {
            waiting_reason: "busy",
            last_attempt_at: "2028-03-13T17:20:00Z",
        })
        expect(ballotBoxRowStatus(busy, at("2028-03-13T17:21:00Z"))).toBe(EBallotBoxRowStatus.DUE)
        expect(ballotBoxRowStatus(busy, at("2028-03-13T17:24:00Z"))).toBe(
            EBallotBoxRowStatus.OVERDUE
        )
    })
})

describe("the reason a pending box isn't sealed is shown inline", () => {
    it.each([
        ["channel_open:KIOSK", "dashboard.ballotBoxes.why.channelOpen"],
        ["channel_not_enabled:KIOSK", "dashboard.ballotBoxes.why.channelNotEnabled"],
        ["channel_has_ballots:TELEPHONE", "dashboard.ballotBoxes.why.channelHasBallots"],
        ["datafix_votes:3", 'dashboard.ballotBoxes.why.datafixVotes {"count":3}'],
        ["error:keystore", "dashboard.ballotBoxes.why.errorCategory.keystore"],
        ["error:board", "dashboard.ballotBoxes.why.errorCategory.board"],
        ["error:ballots", "dashboard.ballotBoxes.why.errorCategory.ballots"],
        ["error:database", "dashboard.ballotBoxes.why.errorCategory.database"],
        // An unknown category, or an old row's raw text, never shows the text itself.
        ["error:vault.internal:8200 refused", "dashboard.ballotBoxes.why.errorCategory.other"],
    ])("%s", (reason, text) => {
        mockSeals = [
            seal(EBallotBoxSealStatus.PENDING, {
                waiting_reason: reason,
                last_attempt_at: "2028-03-13T17:16:00Z",
            }),
        ]
        renderCard(at("2028-03-13T17:16:30Z"))
        expect(screen.getByText("dashboard.ballotBoxes.status.overdue")).toBeTruthy()
        expect(screen.getByText(text, {exact: false})).toBeTruthy()
        expect(screen.queryByText(/vault/)).toBeNull()
    })
    it("says when the sealer hasn't tried, and when its last try is stale", () => {
        mockSeals = [seal(EBallotBoxSealStatus.PENDING)]
        const view = renderCard(at("2028-03-13T17:30:00Z"))
        expect(screen.getByText("dashboard.ballotBoxes.why.notTried")).toBeTruthy()
        view.unmount()
        mockSeals = [seal(EBallotBoxSealStatus.PENDING, {last_attempt_at: "2028-03-13T17:16:00Z"})]
        renderCard(at("2028-03-13T17:30:00Z"))
        expect(screen.getByText(/dashboard.ballotBoxes.why.stale/)).toBeTruthy()
    })
    it("shows a known failure reason in the admin's language", () => {
        mockSeals = [
            seal(EBallotBoxSealStatus.FAILED, {
                failure_reason:
                    "a ballot does not match its Ballot ID (stored a, content hashes to b)",
            }),
        ]
        renderCard(at("2028-03-13T17:30:00Z"))
        expect(screen.getByText("dashboard.ballotBoxes.failure.ballotIdMismatch")).toBeTruthy()
    })
})

describe("the summary", () => {
    it("without a grace period says the ballot boxes are being sealed", () => {
        mockSeals = [seal(EBallotBoxSealStatus.PENDING, {grace_deadline: CLOSED})]
        renderCard(at("2028-03-13T17:00:10Z"))
        expect(screen.getByText(/dashboard.ballotBoxes.sealingNow/)).toBeTruthy()
    })
    it("after the grace period says it ended", () => {
        mockSeals = [seal(EBallotBoxSealStatus.PENDING)]
        renderCard(at("2028-03-13T17:16:00Z"))
        expect(screen.getByText(/dashboard.ballotBoxes.sealingPastGrace/)).toBeTruthy()
    })
    it.each([
        [{voting_status: "NOT_STARTED"}, "dashboard.ballotBoxes.notStarted"],
        [{voting_status: "OPEN"}, "dashboard.ballotBoxes.openOn"],
        [{voting_status: "PAUSED"}, "dashboard.ballotBoxes.paused"],
        [
            {voting_status: "CLOSED", kiosk_voting_status: "NOT_STARTED"},
            "dashboard.ballotBoxes.holding",
        ],
    ])("before the close follows the election's status %j", (status, key) => {
        renderCard(undefined, {status, voting_channels: {online: true, kiosk: true}})
        expect(screen.getByText(new RegExp(key))).toBeTruthy()
    })
    it("before the close, names a never-started early voting that holds the seal", () => {
        renderCard(undefined, {
            status: {voting_status: "CLOSED", early_voting_status: "NOT_STARTED"},
            voting_channels: {online: true, early_voting: true},
        })
        expect(
            screen.getByText(/dashboard.ballotBoxes.holding .*publish.dialog.channel.EARLY_VOTING/)
        ).toBeTruthy()
    })
    it("before the close, names an open channel the Post doesn't enable", () => {
        renderCard(undefined, {
            status: {voting_status: "CLOSED", kiosk_voting_status: "OPEN"},
            voting_channels: {online: true, kiosk: false},
        })
        expect(
            screen.getByText(/publish.dialog.sealNotEnabled .*publish.dialog.channel.KIOSK/)
        ).toBeTruthy()
    })
})

it("moves a pending box to due when its grace period ends, without a reload", () => {
    jest.useFakeTimers()
    jest.setSystemTime(at("2028-03-13T17:14:50Z"))
    mockSeals = [seal(EBallotBoxSealStatus.PENDING)]
    renderCard()
    expect(screen.getByText(/dashboard.ballotBoxes.status.sealing/)).toBeTruthy()
    act(() => {
        jest.advanceTimersByTime(30_000)
    })
    expect(screen.getByText("dashboard.ballotBoxes.status.due")).toBeTruthy()
})

it("lists each ballot box of the election as Open before the close", () => {
    mockStyleAreas = ["a1", "a2"]
    renderCard(undefined, {status: {voting_status: "OPEN"}, voting_channels: {online: true}})
    expect(screen.getByRole("cell", {name: "Area a1"})).toBeTruthy()
    expect(screen.getByRole("cell", {name: "Area a2"})).toBeTruthy()
    expect(screen.getAllByText("dashboard.ballotBoxes.status.open")).toHaveLength(2)
})

it("merges the areas with the seal rows after the close", () => {
    mockStyleAreas = ["area", "a2"]
    mockSeals = [seal(EBallotBoxSealStatus.PUBLISHED, {public_path: "x.json"})]
    renderCard(at("2028-03-13T17:30:00Z"))
    expect(screen.getByRole("cell", {name: "Madrid office"})).toBeTruthy()
    expect(screen.getByRole("cell", {name: "Area a2"})).toBeTruthy()
})

it("names each ballot box after its area and links its published record", () => {
    mockSeals = [
        seal(EBallotBoxSealStatus.PUBLISHED, {
            seal_hash: "109a3264aa",
            sealed_at: DEADLINE,
            ballots_in_box: 3,
            ballots_counted: 2,
            public_path: "/ballot-box-seals/election/area.json",
        }),
    ]
    renderCard(at("2028-03-13T17:20:00Z"))
    expect(screen.getByRole("cell", {name: "Madrid office"})).toBeTruthy()
    expect(screen.getByRole("link").getAttribute("href")).toBe(
        "https://public/ballot-box-seals/election/area.json"
    )
    expect(screen.getByText(/dashboard.ballotBoxes.closedByUser/)).toBeTruthy()
    expect(screen.getByText("dashboard.ballotBoxes.help.counted")).toBeTruthy()
})

describe("a restricted seal record", () => {
    const restricted = () =>
        seal(EBallotBoxSealStatus.PUBLISHED, {
            seal_hash: "109a3264aa",
            sealed_at: DEADLINE,
            public_path: null,
            public_document_id: "record-document",
        })

    it("is downloaded through a presigned URL of its private document", async () => {
        mockSeals = [restricted()]
        mockFetchDocument.mockResolvedValue({
            data: {fetchDocument: {url: "https://private/signed"}},
        })
        renderCard(at("2028-03-13T17:20:00Z"))
        expect(screen.queryByRole("link")).toBeNull()
        const button = screen.getByRole("button", {
            name: /dashboard.ballotBoxes.downloadRecord.*Madrid office/,
        })
        fireEvent.click(button)
        await waitFor(() => expect(mockDownloadUrl).toHaveBeenCalledTimes(1))
        expect(mockFetchDocument).toHaveBeenCalledWith({
            variables: {electionEventId: "event", documentId: "record-document"},
        })
        // The election and the area: two Posts sharing an area don't clash (N1).
        expect(mockDownloadUrl).toHaveBeenCalledWith(
            "https://private/signed",
            "ballot-box-seal-election-area.json"
        )
        expect(screen.queryByText("dashboard.ballotBoxes.recordError")).toBeNull()
    })

    it("says so when the download fails", async () => {
        mockSeals = [restricted()]
        mockFetchDocument.mockResolvedValue({data: undefined, error: new Error("denied")})
        renderCard(at("2028-03-13T17:20:00Z"))
        fireEvent.click(screen.getByRole("button", {name: /dashboard.ballotBoxes.downloadRecord/}))
        expect(await screen.findByText("dashboard.ballotBoxes.recordError")).toBeTruthy()
        expect(mockDownloadUrl).not.toHaveBeenCalled()
    })

    it("says the document is missing, without offering a retry, when it isn't found (S1)", async () => {
        mockSeals = [restricted()]
        mockFetchDocument.mockResolvedValue({
            data: undefined,
            error: {
                message: "Document not found",
                graphQLErrors: [{message: "Document not found", extensions: {}}],
            },
        })
        renderCard(at("2028-03-13T17:20:00Z"))
        fireEvent.click(screen.getByRole("button", {name: /dashboard.ballotBoxes.downloadRecord/}))
        expect(await screen.findByText("dashboard.ballotBoxes.recordMissing")).toBeTruthy()
        expect(screen.queryByText("dashboard.ballotBoxes.recordError")).toBeNull()
        expect(mockDownloadUrl).not.toHaveBeenCalled()
    })

    it("says who can download it when the admin can't", () => {
        mockCanDownload = false
        mockSeals = [restricted()]
        renderCard(at("2028-03-13T17:20:00Z"))
        expect(screen.queryByRole("button", {name: /downloadRecord/})).toBeNull()
        expect(screen.getByText("dashboard.ballotBoxes.recordRestricted")).toBeTruthy()
    })

    it("isn't offered before the seal is published", () => {
        mockSeals = [
            seal(EBallotBoxSealStatus.SEALED, {
                seal_hash: "109a3264aa",
                sealed_at: DEADLINE,
                public_document_id: null,
            }),
        ]
        renderCard(at("2028-03-13T17:20:00Z"))
        expect(screen.queryByRole("button", {name: /downloadRecord/})).toBeNull()
        expect(screen.getByText("dashboard.ballotBoxes.notYet")).toBeTruthy()
    })
})

it("says the seals are unknown, not open, when they can't be read", () => {
    mockError = new Error("permission denied")
    renderCard()
    expect(screen.getByText("dashboard.ballotBoxes.loadError")).toBeTruthy()
    expect(screen.queryByRole("table")).toBeNull()
})

it("shows nothing when the event doesn't seal its ballot boxes", () => {
    mockPolicy = "do-not-seal"
    const {container} = renderCard()
    expect(container.innerHTML).toBe("")
})

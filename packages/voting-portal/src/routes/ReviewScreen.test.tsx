// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Where the review screen puts the box that shows the ballot's ID (design
// boards Review and PhoneReview): in the title row, after the steps, for an
// event that keeps a transparency log; as before for one that doesn't.

import React from "react"
import {fireEvent, render, screen, within} from "@testing-library/react"
import {Provider} from "react-redux"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {EConsolidatedReportPolicy, EVotingPortalAuditButtonCfg} from "@sequentech/ui-core"
import type {IAuditableBallot, IContest, IElection} from "@sequentech/ui-core"
import {BallotEngineProvider} from "@sequentech/ui-essentials"
import {anEngine} from "../../../ui-essentials/src/testing/ballot"
import theme from "../../../ui-essentials/src/services/theme"
import {ELECTION_WITH_INVALID} from "../fixtures/election"
import {RootState, store} from "../store/store"
import {ETransparencyMode} from "../services/vitrine/types"
import {
    clearBallotSent,
    markBallotSent,
    readBallotSent,
} from "../store/receivedBallots/sentBallotMarker"
import {ReviewScreen} from "./ReviewScreen"

jest.mock("react-i18next", () => ({
    Trans: ({i18nKey}: {i18nKey: string}) => i18nKey,
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/types/ElectionEventPresentation"),
    ...jest.requireActual("../../../ui-core/src/services/acclamation"),
    sortContestList: (contests?: IContest[]) => contests ?? [],
    hashBallot: () => mockBallotId,
    hashMultiBallot: () => mockBallotId,
}))
jest.mock("@sequentech/ui-essentials", () => ({
    ...jest.requireActual("../../../ui-essentials/src/index"),
    Question: ({question}: {question: IContest}) => <h2>{question.name}</h2>,
}))
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: RootState) => unknown) => selector(mockState),
    useAppDispatch: () => jest.fn(),
}))
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual<typeof React>("react").createContext({
        userId: "voter-1",
        logout: jest.fn(),
        isKiosk: () => false,
        isGoldUser: () => false,
        reauthWithGold: jest.fn(),
    }),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {DISABLE_AUTH: true},
    }),
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({
        interpretContestSelection: () => [],
        interpretMultiContestSelection: () => [],
        hashBallot: () => mockBallotId,
        hashMultiBallot: () => mockBallotId,
        decodeAuditableBallot: () => undefined,
        decodeAuditableMultiBallot: () => undefined,
        toHashableBallot: () => ({}),
        toHashableMultiBallot: () => ({}),
    }),
}))
jest.mock("../hooks/root-back-link", () => ({useRootBackLink: () => "/chooser"}))
jest.mock("../components/Stepper", () => ({
    __esModule: true,
    default: () => <ol className="stepper-mock" />,
}))
jest.mock("@apollo/client/react", () => ({
    useMutation: () => [jest.fn()],
    useApolloClient: () => ({}),
    useQuery: () => ({data: undefined, startPolling: jest.fn(), stopPolling: jest.fn()}),
}))

let mockState: RootState
const mockBallotId = "0123456789abcdef".repeat(4)

const setUpState = ({
    auditButtonCfg,
    transparency,
    isFullyAcclaimed = false,
}: {
    auditButtonCfg?: EVotingPortalAuditButtonCfg
    transparency?: ETransparencyMode
    isFullyAcclaimed?: boolean
} = {}) => {
    const ballotEml = structuredClone(ELECTION_WITH_INVALID)
    ballotEml.election_id = "election-1"
    ballotEml.election_presentation = {
        audit_button_cfg: auditButtonCfg,
        consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
    }
    ballotEml.election_event_presentation = {
        ...ballotEml.election_event_presentation,
        ...(transparency ? {transparency} : {}),
    } as typeof ballotEml.election_event_presentation
    ballotEml.contests = ballotEml.contests.slice(0, 1).map((contest) => ({
        ...contest,
        is_acclaimed: isFullyAcclaimed,
    }))
    mockState = {
        ...store.getState(),
        elections: {
            "election-1": {
                ...ballotEml,
                id: "election-1",
                image_document_id: "",
                presentation: ballotEml.election_presentation,
            } as IElection,
        },
        ballotStyles: {
            "election-1": {
                id: "election-1",
                election_id: "election-1",
                election_event_id: "event-1",
                tenant_id: "tenant-1",
                ballot_eml: ballotEml,
                created_at: "",
                last_updated_at: "",
            },
        },
        ballotSelections: {"election-1": []},
        auditableBallots: {
            "election-1": {
                auditableBallot: {config: ballotEml, ballot_hash: mockBallotId} as IAuditableBallot,
                isBlankBallot: false,
            },
        },
    }
}

const renderReview = () => {
    const router = createMemoryRouter(
        [
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/*",
                element: <ReviewScreen />,
                action: () => null,
            },
        ],
        {initialEntries: ["/tenant/tenant-1/event/event-1/election/election-1/review?preview=true"]}
    )
    return {
        router,
        ...render(
            <Provider store={store}>
                <ThemeProvider theme={theme}>
                    <BallotEngineProvider engine={anEngine()}>
                        <RouterProvider router={router} />
                    </BallotEngineProvider>
                </ThemeProvider>
            </Provider>
        ),
    }
}

const trackingIdBox = (container: HTMLElement) => container.querySelector(".vitrine-hash-container")
const classicBox = (container: HTMLElement) =>
    container.querySelector(".hash-container:not(.vitrine-hash-container)")

describe("an event that keeps a transparency log", () => {
    beforeEach(() => setUpState({transparency: ETransparencyMode.PUBLIC}))

    it("puts the Tracking ID box in the title row, after the steps", () => {
        const {container} = renderReview()
        const header = container.querySelector(".review-header")!
        const [steps, title, id] = Array.from(header.children)
        expect(steps.className).toContain("review-header-steps")
        expect(steps.querySelector(".stepper-mock")).not.toBeNull()
        expect(title.className).toContain("review-header-title")
        expect(title.querySelector("h1.screen-title")).not.toBeNull()
        expect(id.className).toContain("review-header-id")
        expect(id.querySelector(".vitrine-hash-container")).not.toBeNull()
        expect(classicBox(container)).toBeNull()
    })

    it("shows the box once, with the tag of a ballot not cast", () => {
        const {container} = renderReview()
        expect(container.querySelectorAll(".vitrine-hash-container")).toHaveLength(1)
        expect(screen.getAllByRole("button", {name: /^vitrine.status.aria/})).toHaveLength(1)
        expect(container.querySelector(".status-tag")?.textContent).toBe(
            "vitrine.status.tag.notCast"
        )
        expect(container.querySelector(".vitrine-short-ballot-id")?.textContent).toBe("0123-4567")
    })

    it("opens the help about the Tracking ID from the box", () => {
        renderReview()
        fireEvent.click(screen.getByRole("button", {name: "reviewScreen.ballotIdHelpDialog.title"}))
        const dialog = screen.getByRole("dialog")
        expect(within(dialog).getByText("reviewScreen.ballotIdHelpDialog.cancel")).toBeTruthy()
    })

    it("shows the audit button in the help about the Tracking ID, when it is set to", () => {
        setUpState({
            transparency: ETransparencyMode.PUBLIC,
            auditButtonCfg: EVotingPortalAuditButtonCfg.SHOW_IN_HELP,
        })
        const {container} = renderReview()
        expect(container.querySelector(".audit-button")).toBeNull()
        fireEvent.click(screen.getByRole("button", {name: "reviewScreen.ballotIdHelpDialog.title"}))
        expect(
            within(screen.getByRole("dialog")).getByText("reviewScreen.auditButton")
        ).toBeTruthy()
    })

    it.each([
        ["the audit button is hidden", {auditButtonCfg: EVotingPortalAuditButtonCfg.NOT_SHOW}],
        ["every contest was decided by acclamation", {isFullyAcclaimed: true}],
    ])("shows no box, and keeps the steps and the title as they were, when %s", (_name, setUp) => {
        setUpState({transparency: ETransparencyMode.PUBLIC, ...setUp})
        const {container} = renderReview()
        expect(trackingIdBox(container)).toBeNull()
        expect(classicBox(container)).toBeNull()
        expect(container.querySelector(".review-header")).toBeNull()
        expect(container.querySelector(".stepper-box .stepper-mock")).not.toBeNull()
        expect(container.querySelector("h1.screen-title")).not.toBeNull()
    })
})

describe("an event that keeps no transparency log", () => {
    beforeEach(() => setUpState())

    it("keeps the portal's own box above the steps", () => {
        const {container} = renderReview()
        const box = classicBox(container)!
        const steps = container.querySelector(".stepper-box")!
        expect(box).not.toBeNull()
        expect(box.compareDocumentPosition(steps) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy()
        expect(container.querySelector(".review-header")).toBeNull()
        expect(trackingIdBox(container)).toBeNull()
        expect(screen.queryByRole("button", {name: /^vitrine.status.aria/})).toBeNull()
        expect(container.querySelector(".status-tag")).toBeNull()
    })

    it("still copies the whole ID, and says so", () => {
        renderReview()
        expect(screen.getByRole("button", {name: "reviewScreen.copyBallotId"})).toBeTruthy()
    })
})

describe("a page reloaded after the ballot was sent", () => {
    const owner = {tenantId: "tenant-1", eventId: "event-1", userId: "voter-1"}
    const sent = (override: Partial<Parameters<typeof markBallotSent>[0]> = {}) =>
        markBallotSent({owner, electionId: "election-1", trackingId: "FTBE-MHRX", ...override})
    // The one-time key and the ballot only lived in memory, so nothing is left to cast.
    const afterReload = () => {
        setUpState({transparency: ETransparencyMode.PUBLIC})
        mockState = {...mockState, auditableBallots: {}}
    }
    beforeEach(() => clearBallotSent())
    afterAll(() => clearBallotSent())

    it("says the ballot was not cast, and why", () => {
        afterReload()
        sent()
        const {container} = renderReview()
        expect(container.querySelector(".review-reloaded-title")?.textContent).toBe(
            "reviewScreen.reloaded.title"
        )
        expect(screen.getByText("reviewScreen.reloaded.text")).toBeTruthy()
        expect(screen.getByRole("alert")).toBeTruthy()
        expect(screen.queryByRole("button", {name: "reviewScreen.castBallotButton"})).toBeNull()
    })

    it("takes the voter back to the ballot, and the notice does not come back", () => {
        afterReload()
        sent()
        const {router} = renderReview()
        fireEvent.click(screen.getByRole("button", {name: "reviewScreen.reloaded.back"}))
        expect(router.state.location.pathname).toBe(
            "/tenant/tenant-1/event/event-1/election/election-1/vote"
        )
        expect(router.state.location.search).toBe("?preview=true")
        expect(readBallotSent(owner, "election-1")).toBeUndefined()
        expect(screen.queryByText("reviewScreen.reloaded.text")).toBeNull()
    })

    it.each([
        ["no ballot was sent", () => undefined],
        ["the ballot sent was another voter's", () => sent({owner: {...owner, userId: "voter-2"}})],
        ["the ballot sent was for another election", () => sent({electionId: "election-2"})],
    ])("shows no notice when %s", (_name, mark) => {
        afterReload()
        mark()
        renderReview()
        expect(screen.queryByText("reviewScreen.reloaded.text")).toBeNull()
    })

    it("shows no notice while a ballot is under review, whatever was sent before", () => {
        setUpState({transparency: ETransparencyMode.PUBLIC})
        sent()
        const {container} = renderReview()
        expect(screen.queryByText("reviewScreen.reloaded.text")).toBeNull()
        expect(trackingIdBox(container)).not.toBeNull()
    })
})

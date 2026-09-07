// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {EConsolidatedReportPolicy, EVotingPortalAuditButtonCfg} from "@sequentech/ui-core"
import type {
    IAuditableBallot,
    IContest,
    IElection,
    IVotingScreenBackPolicy,
} from "@sequentech/ui-core"
import theme from "../../../ui-essentials/src/services/theme"
import {ELECTION_WITH_INVALID} from "../fixtures/election"
import {RootState, store} from "../store/store"
import {clearIsVoted} from "../store/extra/extraSlice"
import VotingScreen from "./VotingScreen"
import {ReviewScreen} from "./ReviewScreen"
import ConfirmationScreen from "./ConfirmationScreen"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values?: {ballotId?: string}) =>
            key === "ballotHash" ? `Ballot ID: ${values?.ballotId?.slice(0, 8)}` : key,
        i18n: {language: "en"},
    }),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../ui-core/src/types/ElectionEventPresentation"),
    ...jest.requireActual("../../../ui-core/src/services/acclamation"),
    getDefaultVotingScreenBackPolicy: () => "election-selection-screen",
    check_voting_not_allowed_next_bool: () => false,
    check_voting_error_dialog_bool: () => false,
    sortContestList: (contests?: IContest[]) => contests ?? [],
    hashBallot: () => "0123456789abcdef".repeat(4),
    hashMultiBallot: () => "0123456789abcdef".repeat(4),
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        PageLimit: jest.requireActual("../../../ui-essentials/src/components/PageLimit/PageLimit")
            .default,
        Icon: jest.requireActual("../../../ui-essentials/src/components/Icon/Icon").default,
        IconButton: jest.requireActual(
            "../../../ui-essentials/src/components/IconButton/IconButton"
        ).default,
        DecorativeIconBox: jest.requireActual(
            "../../../ui-essentials/src/components/Icon/DecorativeIconBox"
        ).default,
        VisuallyHidden: jest.requireActual(
            "../../../ui-essentials/src/components/VisuallyHidden/VisuallyHidden"
        ).default,
        ...jest.requireActual(
            "../../../ui-essentials/src/components/ConfirmationActions/ConfirmationActions"
        ),
        BallotHash: jest.requireActual(
            "../../../ui-essentials/src/components/BallotHash/BallotHash"
        ).default,
        BallotHashCopyButton: jest.requireActual(
            "../../../ui-essentials/src/components/BallotHash/BallotHash"
        ).BallotHashCopyButton,
        theme: jest.requireActual("../../../ui-essentials/src/services/theme").default,
        Dialog: () => null,
        QRCode: () => null,
    }),
    {virtual: true}
)
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: (state: RootState) => unknown) => selector(mockState),
    useAppDispatch: () => mockDispatch,
}))
jest.mock("../providers/AuthContextProvider", () => ({
    AuthContext: jest.requireActual<typeof React>("react").createContext({
        logout: jest.fn(),
        isGoldUser: () => false,
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
        hashBallot: () => "0123456789abcdef".repeat(4),
        hashMultiBallot: () => "0123456789abcdef".repeat(4),
    }),
}))
jest.mock("../hooks/useEncryptBallotForReview", () => ({
    useEncryptBallotForReview: () => ({encryptAndStoreBallot: jest.fn()}),
}))
jest.mock("../hooks/root-back-link", () => ({
    useRootBackLink: () => "/tenant/tenant-1/event/event-1/election-chooser",
}))
jest.mock("../hooks/public-document-url", () => ({
    useGetPublicDocumentUrl: () => ({getDocumentUrl: jest.fn()}),
}))
jest.mock("../components/Question/Question", () => ({
    Question: ({question}: {question: IContest}) => <h2>{question.name}</h2>,
}))
jest.mock("../components/Stepper", () => ({__esModule: true, default: () => null}))
jest.mock("@apollo/client/react", () => ({
    useMutation: () => [jest.fn()],
    useQuery: () => ({startPolling: jest.fn(), stopPolling: jest.fn()}),
}))

const mockDispatch = jest.fn()
let mockState: RootState
const BALLOT_ID = "0123456789abcdef".repeat(4)
const ELECTION_PATH = "/tenant/tenant-1/event/event-1/election/election-1"

const setUpState = ({
    auditButtonCfg,
    backPolicy,
    isFullyAcclaimed = false,
    storedConfirmation = false,
}: {
    auditButtonCfg?: EVotingPortalAuditButtonCfg
    backPolicy?: IVotingScreenBackPolicy
    isFullyAcclaimed?: boolean
    storedConfirmation?: boolean
} = {}) => {
    const ballotEml = structuredClone(ELECTION_WITH_INVALID)
    ballotEml.election_id = "election-1"
    ballotEml.election_presentation = {
        audit_button_cfg: auditButtonCfg,
        consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
    }
    ballotEml.contests = ["First contest", "Second contest"].map((name, index) => ({
        ...ballotEml.contests[0],
        id: `contest-${index}`,
        name,
        is_acclaimed: isFullyAcclaimed,
        presentation: {pagination_policy: `page-${index}`},
    }))
    mockState = {
        ...store.getState(),
        elections: {
            "election-1": {
                ...ballotEml,
                id: "election-1",
                image_document_id: "",
                presentation: {
                    ...ballotEml.election_presentation,
                    voting_screen_back_policy: backPolicy,
                },
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
        auditableBallots: storedConfirmation
            ? {}
            : {
                  "election-1": {
                      auditableBallot: {
                          config: ballotEml,
                          ballot_hash: BALLOT_ID,
                      } as IAuditableBallot,
                      isBlankBallot: false,
                  },
              },
        confirmationScreenData: storedConfirmation
            ? {
                  "election-1": {ballotId: BALLOT_ID, isDemo: true},
              }
            : {},
    }
}

const renderRoute = (element: React.ReactElement, path: string) => {
    const router = createMemoryRouter(
        [
            {path: "/tenant/:tenantId/event/:eventId/election/:electionId/*", element},
            {
                path: "/tenant/:tenantId/event/:eventId/election-chooser",
                element: <div>Chooser</div>,
            },
        ],
        {initialEntries: [`${ELECTION_PATH}/${path}?preview=true`]}
    )
    const view = render(
        <ThemeProvider theme={theme}>
            <RouterProvider router={router} />
        </ThemeProvider>
    )
    return {...view, router}
}

beforeEach(() => {
    jest.clearAllMocks()
    setUpState()
})

describe("selection-screen Back", () => {
    it.each(["{Enter}", " "])(
        "returns to the previous contest with %s without leaving the ballot",
        async (key) => {
            const user = userEvent.setup()
            const {router} = renderRoute(<VotingScreen />, "vote")
            await user.click(screen.getByRole("button", {name: "votingScreen.reviewButton"}))
            expect(screen.getByRole("heading", {name: "Second contest"})).toBeInTheDocument()
            const back = screen.getByRole("button", {name: "votingScreen.backButton"})
            expect(back.tagName).toBe("BUTTON")
            await user.tab()
            await user.tab()
            expect(back).toHaveFocus()
            await user.tab()
            await user.tab({shift: true})
            expect(back).toHaveFocus()
            await user.keyboard(key)
            expect(screen.getByRole("heading", {name: "First contest"})).toBeInTheDocument()
            expect(router.state.location.pathname + router.state.location.search).toBe(
                `${ELECTION_PATH}/vote?preview=true`
            )
            expect(mockDispatch).not.toHaveBeenCalledWith(clearIsVoted())
        }
    )

    it.each([
        ["start-screen", "{Enter}", `${ELECTION_PATH}/start`],
        ["start-screen", " ", `${ELECTION_PATH}/start`],
        ["election-selection-screen", "{Enter}", "/tenant/tenant-1/event/event-1/election-chooser"],
        ["election-selection-screen", " ", "/tenant/tenant-1/event/event-1/election-chooser"],
        [undefined, " ", "/tenant/tenant-1/event/event-1/election-chooser"],
    ] as const)(
        "uses back policy %s on the first contest with %s",
        async (backPolicy, key, expectedPath) => {
            setUpState({backPolicy})
            const user = userEvent.setup()
            const {router} = renderRoute(<VotingScreen />, "vote")
            await user.tab()
            await user.tab()
            await user.tab()
            expect(screen.getByRole("button", {name: "votingScreen.backButton"})).toHaveFocus()
            await user.keyboard(key)
            expect(router.state.location.pathname + router.state.location.search).toBe(
                `${expectedPath}?preview=true`
            )
            expect(mockDispatch).toHaveBeenCalledWith(clearIsVoted())
        }
    )
})

describe("Ballot ID copy visibility", () => {
    it.each([
        [EVotingPortalAuditButtonCfg.SHOW, true],
        [EVotingPortalAuditButtonCfg.SHOW_IN_HELP, true],
        [EVotingPortalAuditButtonCfg.NOT_SHOW, false],
        [undefined, true],
    ] as const)("matches Review on success for audit policy %s", (auditButtonCfg, visible) => {
        setUpState({auditButtonCfg})
        const review = renderRoute(<ReviewScreen />, "review")
        expect(Boolean(screen.queryByRole("button", {name: "reviewScreen.copyBallotId"}))).toBe(
            visible
        )
        review.unmount()
        renderRoute(<ConfirmationScreen />, "confirmation")
        expect(Boolean(screen.queryByRole("button", {name: "reviewScreen.copyBallotId"}))).toBe(
            visible
        )
        expect(screen.getByText(BALLOT_ID)).toBeInTheDocument()
        const help = within(screen.getByText(BALLOT_ID).parentElement!).getByRole("button", {
            name: "a11y.helpAbout",
        })
        expect(getComputedStyle(help).marginLeft).toBe(visible ? "0px" : "16px")
    })

    it("shows no copy control on either screen for fully acclaimed elections", () => {
        setUpState({isFullyAcclaimed: true})
        const review = renderRoute(<ReviewScreen />, "review")
        expect(screen.queryByRole("button", {name: "reviewScreen.copyBallotId"})).toBeNull()
        review.unmount()
        renderRoute(<ConfirmationScreen />, "confirmation")
        expect(screen.queryByRole("button", {name: "reviewScreen.copyBallotId"})).toBeNull()
        expect(screen.queryByText(BALLOT_ID)).toBeNull()
    })

    it.each([false, true])(
        "copies the full success-screen ID, including stored confirmation: %s",
        async (storedConfirmation) => {
            setUpState({storedConfirmation})
            const user = userEvent.setup()
            const writeText = jest.spyOn(navigator.clipboard, "writeText").mockResolvedValue()
            renderRoute(<ConfirmationScreen />, "confirmation")
            await user.click(screen.getByRole("button", {name: "reviewScreen.copyBallotId"}))
            expect(writeText).toHaveBeenCalledWith(BALLOT_ID)
            expect(
                screen.getByRole("button", {name: "reviewScreen.ballotIdCopied"})
            ).toBeInTheDocument()
        }
    )
})

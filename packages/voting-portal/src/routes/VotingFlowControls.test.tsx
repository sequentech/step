// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "node:assert/strict"
import React from "react"
import {render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {createMemoryRouter, RouterProvider} from "react-router-dom"
import {
    ECastVoteGoldLevelPolicy,
    EConsolidatedReportPolicy,
    EVotingPortalAuditButtonCfg,
} from "@sequentech/ui-core"
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
import confirmationScreenDataReducer, {
    setConfirmationScreenData,
} from "../store/castVotes/confirmationScreenDataSlice"
import {BALLOT_DATA_KEY} from "../store/castVotes/sessionBallotData"
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
        QRCode: jest.requireActual("../../../ui-essentials/src/components/QRCode/QRCode").default,
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
        isGoldUser: () => mockIsGoldUser,
        reauthWithGold: (url: string) => mockReauthWithGold(url),
    }),
}))
jest.mock("../providers/SettingsContextProvider", () => ({
    SettingsContext: jest.requireActual<typeof React>("react").createContext({
        globalSettings: {
            get DISABLE_AUTH() {
                return mockDisableAuth
            },
        },
    }),
}))
jest.mock("../services/BallotService", () => ({
    provideBallotService: () => ({
        interpretContestSelection: () => [],
        interpretMultiContestSelection: () => [],
        hashBallot: () => "0123456789abcdef".repeat(4),
        hashMultiBallot: () => "0123456789abcdef".repeat(4),
        toHashableBallot: () => ({}),
        toHashableMultiBallot: () => ({}),
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
    useMutation: () => [mockInsertCastVote],
    useQuery: () => ({
        data: mockElectionQueryData,
        startPolling: jest.fn(),
        stopPolling: jest.fn(),
    }),
}))

const mockDispatch = jest.fn()
const mockReauthWithGold = jest.fn()
const mockInsertCastVote = jest.fn()
let mockIsGoldUser = false
let mockDisableAuth = true
let mockElectionQueryData:
    | {
          sequent_backend_election: Array<{
              id: string
              presentation: IElection["presentation"]
              status: {voting_status: string}
          }>
      }
    | undefined
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
        elections: storedConfirmation
            ? {}
            : {
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
        ballotStyles: storedConfirmation
            ? {}
            : {
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
            {
                path: "/tenant/:tenantId/event/:eventId/election/:electionId/*",
                element,
                action: () => null,
            },
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
    mockDispatch.mockReset()
    mockInsertCastVote.mockResolvedValue({data: {insert_cast_vote: {id: "cast-vote-1"}}})
    mockReauthWithGold.mockResolvedValue(undefined)
    mockIsGoldUser = false
    mockDisableAuth = true
    mockElectionQueryData = undefined
    sessionStorage.clear()
    setUpState()
})

afterEach(() => sessionStorage.clear())

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
    it("can hide the complete receipt ID row without hiding verification or actions", async () => {
        setUpState({storedConfirmation: true})
        mockState.confirmationScreenData["election-1"] = {
            ballotId: BALLOT_ID,
            isDemo: true,
            auditButtonCfg: EVotingPortalAuditButtonCfg.SHOW,
        }
        const user = userEvent.setup()
        const {container} = renderRoute(<ConfirmationScreen />, "confirmation")
        const row = container.querySelector(".ballot-id-container")
        assert.ok(row, "Confirmation must render a receipt ID row")
        expect(row).toContainElement(
            screen.getByRole("button", {name: "reviewScreen.copyBallotId"})
        )
        expect(row.querySelector(".ballot-id-help-button")).toHaveProperty("tagName", "BUTTON")
        expect(row.querySelector(".ballot-id-value-desktop")).toHaveTextContent(BALLOT_ID)
        expect(row.querySelector(".ballot-id-value-mobile")).toBeInTheDocument()
        expect(container.querySelector(".finish-button-label")).toHaveTextContent(
            "confirmationScreen.finishButton"
        )
        const style = document.createElement("style")
        style.textContent = ".confirmation-screen .ballot-id-container { display: none; }"
        document.head.appendChild(style)
        try {
            expect(row).not.toBeVisible()
            expect(screen.queryByRole("button", {name: "reviewScreen.copyBallotId"})).toBeNull()
            const qr = container.querySelector(".qr-code-svg")
            assert.ok(qr, "Hiding the receipt ID must preserve its verification QR code")
            expect(qr).toBeVisible()
            expect(qr).toHaveAccessibleName("confirmationScreen.verifyCastDescription")
            expect(
                screen.getByRole("button", {name: "confirmationScreen.printButton"})
            ).toBeVisible()
            expect(
                screen.getByRole("button", {name: "confirmationScreen.finishButton"})
            ).toBeVisible()
            for (let index = 0; index < 4; index++) {
                await user.tab()
                expect(row).not.toContainElement(document.activeElement as HTMLElement)
            }
        } finally {
            style.remove()
        }
    })

    it.each([
        [EVotingPortalAuditButtonCfg.SHOW, true],
        [EVotingPortalAuditButtonCfg.SHOW_IN_HELP, true],
        [EVotingPortalAuditButtonCfg.NOT_SHOW, false],
        [undefined, true],
    ] as const)(
        "preserves copy visibility for %s across gold reauthentication",
        async (auditButtonCfg, visible) => {
            setUpState({auditButtonCfg})
            const election = mockState.elections["election-1"]
            assert.ok(election, "The voting fixture must include the selected election")
            const presentation = {
                ...election.presentation,
                consolidated_report_policy: EConsolidatedReportPolicy.DO_NOT_GENERATE,
                cast_vote_gold_level: ECastVoteGoldLevelPolicy.GOLD_LEVEL,
            }
            election.presentation = presentation
            mockDisableAuth = false
            const user = userEvent.setup()
            const review = renderRoute(<ReviewScreen />, "review")
            expect(Boolean(screen.queryByRole("button", {name: "reviewScreen.copyBallotId"}))).toBe(
                visible
            )
            await user.click(screen.getByRole("button", {name: "reviewScreen.castBallotButton"}))
            await waitFor(() => expect(mockReauthWithGold).toHaveBeenCalledTimes(1))
            expect(mockInsertCastVote).not.toHaveBeenCalled()
            const storedBallot = sessionStorage.getItem(BALLOT_DATA_KEY)
            assert.ok(storedBallot, "Gold reauthentication must retain the pending ballot")
            expect(JSON.parse(storedBallot)).toMatchObject({
                ballotId: BALLOT_ID,
                isDemo: false,
                auditButtonCfg: auditButtonCfg ?? EVotingPortalAuditButtonCfg.SHOW,
            })
            review.unmount()

            mockState = store.getState()
            mockIsGoldUser = true
            mockElectionQueryData = {
                sequent_backend_election: [
                    {
                        id: election.id,
                        presentation: {
                            ...presentation,
                            audit_button_cfg: EVotingPortalAuditButtonCfg.SHOW,
                        },
                        status: {voting_status: "open"},
                    },
                ],
            }
            mockDispatch.mockImplementation((action) => {
                if (setConfirmationScreenData.match(action)) {
                    mockState = {
                        ...mockState,
                        confirmationScreenData: confirmationScreenDataReducer(
                            mockState.confirmationScreenData,
                            action
                        ),
                    }
                }
            })
            const authenticatedReview = renderRoute(<ReviewScreen />, "review")
            await waitFor(() =>
                expect(mockDispatch).toHaveBeenCalledWith(
                    setConfirmationScreenData({
                        electionId: "election-1",
                        confirmationScreenData: {
                            ballotId: BALLOT_ID,
                            isDemo: false,
                            auditButtonCfg: auditButtonCfg ?? EVotingPortalAuditButtonCfg.SHOW,
                        },
                    })
                )
            )
            expect(mockInsertCastVote).toHaveBeenCalledTimes(1)
            expect(mockInsertCastVote).toHaveBeenCalledWith({
                variables: {electionId: "election-1", ballotId: BALLOT_ID, content: "{}"},
            })
            expect(sessionStorage.getItem(BALLOT_DATA_KEY)).toBeNull()
            expect(mockState.ballotStyles).toEqual({})
            expect(mockState.elections).toEqual({})
            authenticatedReview.unmount()

            renderRoute(<ConfirmationScreen />, "confirmation")
            expect(screen.getByText(BALLOT_ID)).toBeInTheDocument()
            expect(Boolean(screen.queryByRole("button", {name: "reviewScreen.copyBallotId"}))).toBe(
                visible
            )
        }
    )

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
        const receiptRow = screen.getByText(BALLOT_ID).parentElement
        assert.ok(receiptRow, "The receipt ID must have a container for its help button")
        const help = within(receiptRow).getByRole("button", {
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

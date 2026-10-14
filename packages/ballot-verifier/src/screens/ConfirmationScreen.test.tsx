// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {render, screen, waitFor, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {ThemeProvider} from "@mui/material"
import {theme} from "@sequentech/ui-essentials"
import {
    EBlankBallotsPolicy,
    EDeclineToVotePolicy,
    EElectionEventContestEncryptionPolicy,
    IBallotStyle,
    IDecodedVoteContest,
    IElectionEventPresentation,
    IElectionPresentation,
} from "@sequentech/ui-core"
import "../services/i18n"
import {ConfirmationScreen} from "./ConfirmationScreen"
import {IConfirmationBallot} from "../services/BallotService"
import {TenantEventProvider} from "../providers/TenantEventContext"
import {resetSequentCore, sequentCore} from "../__mocks__/sequentCore"
import {ballotStyle, firstCandidateChosen, IDS} from "../__mocks__/auditableBallots"

const eventPath = `/tenant/${IDS.tenant}/event/${IDS.event}`
const ballotId = "5c".repeat(32)
// Changes the first character only, as the journeys do.
const otherBallotId = `0${ballotId.slice(1)}`
const mismatchError = "Does’t match the decoded ballot ID"

const verified = (
    style: IBallotStyle = ballotStyle(),
    decoded: IDecodedVoteContest[] = firstCandidateChosen(style)
): IConfirmationBallot => ({
    ballot_hash: ballotId,
    election_config: style,
    decoded_questions: decoded,
})

const nothingChosen = (style: IBallotStyle, flags: Partial<IDecodedVoteContest> = {}) =>
    firstCandidateChosen(style).map((contest) => ({
        ...contest,
        ...flags,
        choices: contest.choices.map((choice) => ({...choice, selected: -1})),
    }))

function renderConfirmation(confirmationBallot: IConfirmationBallot | null, providedId: string) {
    return render(
        <ThemeProvider theme={theme}>
            <MemoryRouter initialEntries={[`${eventPath}/confirmation`]}>
                <TenantEventProvider tenantId={IDS.tenant} eventId={IDS.event}>
                    <div className="app-root" data-testid="app-root">
                        <Routes>
                            <Route
                                path={`${eventPath}/confirmation`}
                                element={
                                    <ConfirmationScreen
                                        confirmationBallot={confirmationBallot}
                                        ballotId={providedId}
                                    />
                                }
                            />
                            <Route path={`${eventPath}/start`} element={<p>Import step</p>} />
                        </Routes>
                    </div>
                </TenantEventProvider>
            </MemoryRouter>
        </ThemeProvider>
    )
}

const selectionsHeading = () =>
    screen.queryByRole("heading", {name: "Verify your ballot selections"})

beforeEach(() => {
    resetSequentCore()
    jest.spyOn(console, "log").mockImplementation(() => undefined)
    jest.spyOn(console, "info").mockImplementation(() => undefined)
})
afterEach(() => jest.restoreAllMocks())

describe("comparing ballot IDs", () => {
    it("shows the decoded selections when the provided ballot ID matches", () => {
        renderConfirmation(verified(), ballotId)

        expect(screen.getAllByText(ballotId)).toHaveLength(2)
        expect(screen.queryByText(mismatchError)).not.toBeInTheDocument()
        expect(selectionsHeading()).toBeVisible()
        expect(screen.getByText("Council representative")).toBeVisible()
        expect(screen.getByText("Alice Example", {exact: true})).toBeVisible()
        expect(screen.queryByText("Bob Example", {exact: true})).not.toBeInTheDocument()
    })

    it("flags a provided ballot ID that differs and withholds the selections", () => {
        renderConfirmation(verified(), otherBallotId)

        expect(screen.getByText(ballotId)).toBeVisible()
        expect(screen.getByText(otherBallotId)).toBeVisible()
        expect(screen.getByText(mismatchError)).toBeVisible()
        expect(selectionsHeading()).not.toBeInTheDocument()
        expect(screen.queryByText("Alice Example", {exact: true})).not.toBeInTheDocument()
    })
})

describe("decoded selections", () => {
    it("lists every contest of a multi-contest ballot in the published contest order", () => {
        const style = ballotStyle(true)
        renderConfirmation(verified(style, firstCandidateChosen(style).reverse()), ballotId)

        const alice = screen.getByText("Alice Example", {exact: true})
        const charlie = screen.getByText("Charlie Example", {exact: true})
        expect(screen.getByText("Council representative")).toBeVisible()
        expect(screen.getByText("School representative")).toBeVisible()
        expect(alice.compareDocumentPosition(charlie) & Node.DOCUMENT_POSITION_FOLLOWING).toBe(
            Node.DOCUMENT_POSITION_FOLLOWING
        )
        expect(screen.queryByText("Bob Example", {exact: true})).not.toBeInTheDocument()
    })

    it("adds an acclaimed contest from the ballot style, which the ballot never encodes", () => {
        const style = ballotStyle()
        style.contests.push({
            ...style.contests[0],
            id: "board",
            name: "Board of trustees",
            is_acclaimed: true,
            candidates: ["Dana Example", "Eve Example"].map((name, index) => ({
                ...style.contests[0].candidates[0],
                id: `trustee-${index}`,
                contest_id: "board",
                name,
            })),
        })
        renderConfirmation(verified(style, firstCandidateChosen(ballotStyle())), ballotId)

        expect(screen.getByText("Alice Example", {exact: true})).toBeVisible()
        expect(screen.getByText("Board of trustees")).toBeVisible()
        expect(screen.getByRole("alert")).toHaveTextContent(
            "This contest was decided by acclamation."
        )
        expect(screen.getByText("Dana Example", {exact: true})).toBeVisible()
        expect(screen.getByText("Eve Example", {exact: true})).toBeVisible()
    })

    it("reports a decoded contest that the ballot style does not define", () => {
        const decoded = firstCandidateChosen(ballotStyle())
        decoded[0].contest_id = "missing"
        renderConfirmation(verified(ballotStyle(), decoded), ballotId)

        expect(screen.getByText("Contest not found: missing")).toBeVisible()
    })

    const {MULTIPLE_CONTESTS, SINGLE_CONTEST} = EElectionEventContestEncryptionPolicy
    const blank = {
        ballot: "a blank ballot",
        flags: {is_blank_ballot: true},
        policy: {blank_ballots_policy: EBlankBallotsPolicy.ENABLED},
    }
    const declined = {
        ballot: "a declined ballot",
        flags: {is_decline_to_vote: true},
        policy: {decline_to_vote_policy: EDeclineToVotePolicy.ENABLED},
    }
    // Both policies need multiple-contest encryption; otherwise an empty contest
    // is an ordinary blank vote (docs: 07-decline-to-vote, 09-blank-ballots).
    it.each([
        {...blank, encryption: MULTIPLE_CONTESTS, label: "Blank ballot"},
        {...declined, encryption: MULTIPLE_CONTESTS, label: "Declined to vote"},
        {...blank, encryption: SINGLE_CONTEST, label: "Blank Vote"},
        {...declined, encryption: SINGLE_CONTEST, label: "Blank Vote"},
    ])(
        "labels $ballot with $encryption encryption as $label",
        ({flags, policy, encryption, label}) => {
            const style = ballotStyle()
            style.election_presentation = policy as IElectionPresentation
            style.election_event_presentation = {
                contest_encryption_policy: encryption,
            } as IElectionEventPresentation
            // sequent-core calls a contest blank when nothing is selected (plaintext.rs).
            sequentCore.check_is_blank_js.mockReturnValue(true)
            renderConfirmation(verified(style, nothingChosen(style, flags)), ballotId)

            expect(screen.getByText(label, {exact: true})).toBeVisible()
            expect(screen.queryByText("Alice Example", {exact: true})).not.toBeInTheDocument()
        }
    )
})

describe("leaving the confirmation step", () => {
    it("returns to the event's import step when no ballot has been verified", async () => {
        renderConfirmation(null, "")

        expect(await screen.findByText("Import step")).toBeVisible()
    })

    it("goes back to the event's import step", async () => {
        renderConfirmation(verified(), ballotId)

        userEvent.click(screen.getByRole("link", {name: "Back"}))

        expect(await screen.findByText("Import step")).toBeVisible()
    })

    it("prints the verification", () => {
        const print = jest.spyOn(window, "print").mockImplementation(() => undefined)
        renderConfirmation(verified(), ballotId)

        userEvent.click(screen.getByRole("button", {name: "Print"}))

        expect(print).toHaveBeenCalledTimes(1)
    })
})

it("explains both ballot IDs and the selections in help dialogs", async () => {
    renderConfirmation(verified(), ballotId)
    for (const [label, title] of [
        ["Decoded Ballot Id", "Information: Decoded Ballot Id"],
        ["The Ballot Id you provided", "Information: The Ballot Id you provided"],
        ["Verify your ballot selections", "Information: Verify your ballot selections"],
    ]) {
        // Each help button sits at the end of the row it explains.
        const row = within(screen.getByText(label).parentElement!)
        userEvent.click(row.getAllByRole("button").at(-1)!)
        const dialog = await screen.findByRole("dialog", {name: title})
        userEvent.click(within(dialog).getByRole("button", {name: "OK"}))
        await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument())
    }
})

// The app root carries a class for the verified election, which election
// Custom CSS can target; ui-core's formatter prefixes "e-" and drops spaces.
it.each([
    [
        "its alias",
        {i18n: {en: {alias: "Council 2026", name: "Community Council"}}},
        "e-Council2026",
    ],
    ["its name", {i18n: {en: {name: "Community Council"}}}, "e-CommunityCouncil"],
    [
        "its name in the default language",
        {i18n: {es: {name: "Consejo"}}, language_conf: {default_language_code: "es"}},
        "e-Consejo",
    ],
    ["its ID without a name", {i18n: {}}, `e-${IDS.election}`],
])(
    "marks the app root with the election's class from %s until the voter leaves",
    async (_source, presentation, className) => {
        const style = ballotStyle()
        style.election_presentation = presentation as IElectionPresentation
        renderConfirmation(verified(style), ballotId)
        const appRoot = screen.getByTestId("app-root")
        expect(appRoot).toHaveClass(className)

        userEvent.click(screen.getByRole("link", {name: "Back"}))

        expect(await screen.findByText("Import step")).toBeVisible()
        expect(appRoot).not.toHaveClass(className)
    }
)

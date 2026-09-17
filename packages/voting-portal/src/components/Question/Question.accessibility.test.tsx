// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// These cases assert rendered styles and ARIA wiring, so they run the real
// Candidate and WarnBox components instead of stubs. ui-essentials and ui-core
// are imported from source rather than their build output so the test does not
// depend on a prior `yarn build:ui-essentials`; the virtual module mock below
// only lists what Question renders, so adding an import there means adding it
// here too.
import React from "react"
import {render, screen} from "@testing-library/react"
import {ThemeProvider} from "@mui/material/styles"
import {IInvalidPlaintextErrorType} from "../../types/errors"
import type {BallotSelection, ICandidate, IContest} from "@sequentech/ui-core"
import theme from "../../../../ui-essentials/src/services/theme"
import type {RootState} from "../../store/store"
import type {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {Question, IQuestionProps} from "./Question"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("@sequentech/ui-core"),
    ...jest.requireActual("../../../../ui-core/src/utils/array"),
    ...jest.requireActual("../../../../ui-core/src/services/acclamation"),
    ...jest.requireActual("../../../../ui-core/src/services/categoryService"),
    // These helpers normally call WASM; the fixture has one ordinary eligible
    // candidate, so no encoding, winner filtering or ordering is needed here.
    sortCandidatesInContest: (candidates: ICandidate[]) => candidates,
    checkIsBlank: () => false,
    isEligibleAcclaimedCandidate: () => true,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        theme: jest.requireActual("../../../../ui-essentials/src/services/theme").default,
        VisuallyHidden: jest.requireActual(
            "../../../../ui-essentials/src/components/VisuallyHidden/VisuallyHidden"
        ).default,
        BlankAnswer: jest.requireActual(
            "../../../../ui-essentials/src/components/BlankAnswer/BlankAnswer"
        ).default,
        Candidate: jest.requireActual(
            "../../../../ui-essentials/src/components/Candidate/Candidate"
        ).default,
        ...jest.requireActual("../../../../ui-essentials/src/components/WarnBox/WarnBox"),
        WarnBox: jest.requireActual("../../../../ui-essentials/src/components/WarnBox/WarnBox")
            .default,
    }),
    {virtual: true}
)
jest.mock("../../store/hooks", () => ({
    useAppSelector: (selector: (state: RootState) => unknown) =>
        selector({
            ...jest
                .requireActual<typeof import("../../store/store")>("../../store/store")
                .store.getState(),
            ballotSelections: {election: mockSelection},
        }),
    useAppDispatch: () => jest.fn(),
}))
jest.mock("../../services/BallotService", () => ({
    provideBallotService: () => ({isPreferential: () => false}),
}))

const question: IContest = {
    id: "contest",
    tenant_id: "tenant",
    election_event_id: "event",
    election_id: "election",
    name: "Council",
    max_votes: 1,
    min_votes: 1,
    winning_candidates_num: 1,
    is_encrypted: true,
    candidates: [
        {
            id: "candidate",
            tenant_id: "tenant",
            election_event_id: "event",
            election_id: "election",
            contest_id: "contest",
            name: "Candidate One",
        },
    ],
}
let mockSelection: BallotSelection

const opacity = (element: Element) => {
    const value = getComputedStyle(element).opacity || "1"
    return parseFloat(value) / (value.endsWith("%") ? 100 : 1)
}

const renderQuestion = (props: Partial<IQuestionProps> = {}) => {
    const contest = props.question ?? question
    const ballotStyle = {
        election_id: "election",
        ballot_eml: {contests: [contest]},
    } as IBallotStyle
    return render(
        <ThemeProvider theme={theme}>
            <Question
                ballotStyle={ballotStyle}
                question={contest}
                isReview={false}
                setDecodedContests={jest.fn()}
                errorSelectionState={mockSelection}
                {...props}
            />
        </ThemeProvider>
    )
}

beforeEach(() => {
    mockSelection = [
        {
            contest_id: question.id,
            choices: [{id: "candidate", selected: 0}],
            invalid_alerts: [],
            invalid_errors: [
                {
                    message: "errors.implicit.underVote",
                    error_type: IInvalidPlaintextErrorType.Implicit,
                    message_map: {},
                },
            ],
            is_explicit_invalid: false,
            is_decline_to_vote: false,
            is_blank_ballot: false,
        },
    ]
})

it.each([false, true])(
    "does not describe an acclaimed contest with a missing error region (review: %s)",
    (isReview) => {
        renderQuestion({question: {...question, is_acclaimed: true}, isReview})
        expect(screen.getByText("Candidate One")).toBeVisible()
        expect(screen.queryByRole("status")).toBeNull()
        expect(screen.getByRole("group", {name: /Council/})).not.toHaveAttribute("aria-describedby")
    }
)

it("keeps acclaimed candidate text opaque on review while retaining disabled voting controls", () => {
    const contest = {...question, is_acclaimed: true}
    const vote = renderQuestion({question: contest})
    const candidate = screen.getByText("Candidate One").closest("li")!
    expect(opacity(candidate)).toBe(0.5)
    expect(screen.getByRole("checkbox", {name: /Candidate One/})).toBeDisabled()
    vote.unmount()

    renderQuestion({question: contest, isReview: true})
    const reviewCandidate = screen.getByText("Candidate One").closest("li")!
    expect(opacity(reviewCandidate)).toBe(1)
    expect(screen.queryByRole("checkbox")).toBeNull()
})

it.each([false, true])(
    "keeps ordinary contest errors associated with the answer group (review: %s)",
    (isReview) => {
        renderQuestion({isReview})
        const group = screen.getByRole("group", {name: /Council/})
        const status = screen.getByRole("status")
        expect(document.getElementById(group.getAttribute("aria-describedby")!)).toBe(status)
        expect(status).toHaveTextContent("errors.implicit.underVote")
        expect(group).toHaveAccessibleDescription(/errors.implicit.underVote/)
    }
)

it.each([{isBlankBallot: true}, {isDeclineToVote: true}])(
    "does not leave an answer-group error reference on ballot-level review: %j",
    (props) => {
        const {container} = renderQuestion({isReview: true, ...props})
        expect(screen.queryByRole("group")).toBeNull()
        expect(screen.queryByRole("status")).toBeNull()
        expect(container.querySelector("[aria-describedby]")).toBeNull()
    }
)

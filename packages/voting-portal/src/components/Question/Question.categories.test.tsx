// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, waitFor} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {ThemeProvider} from "@mui/material/styles"
import {ECollapsibleLists} from "@sequentech/ui-core"
import type {ICandidate} from "@sequentech/ui-core"
import theme from "../../../../ui-essentials/src/services/theme"
import {ELECTION_WITH_INVALID} from "../../fixtures/election"
import type {IBallotStyle} from "../../store/ballotStyles/ballotStylesSlice"
import {Question} from "./Question"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key, i18n: {language: "en"}}),
}))
jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual<typeof import("@sequentech/ui-core")>("@sequentech/ui-core"),
    ...jest.requireActual<typeof import("../../../../ui-core/src/services/categoryService")>(
        "../../../../ui-core/src/services/categoryService"
    ),
    ...jest.requireActual<typeof import("../../../../ui-core/src/services/candidatePresentation")>(
        "../../../../ui-core/src/services/candidatePresentation"
    ),
    ...jest.requireActual<typeof import("../../../../ui-core/src/services/presentationOrder")>(
        "../../../../ui-core/src/services/presentationOrder"
    ),
    ...jest.requireActual<typeof import("../../../../ui-core/src/services/acclamation")>(
        "../../../../ui-core/src/services/acclamation"
    ),
    ...jest.requireActual<typeof import("../../../../ui-core/src/utils/array")>(
        "../../../../ui-core/src/utils/array"
    ),
    sortCandidatesInContest: (candidates: ICandidate[]) => candidates,
}))
jest.mock(
    "@sequentech/ui-essentials",
    () => ({
        theme: jest.requireActual<typeof import("../../../../ui-essentials/src/services/theme")>(
            "../../../../ui-essentials/src/services/theme"
        ).default,
        CandidatesList: jest.requireActual<
            typeof import("../../../../ui-essentials/src/components/CandidatesList/CandidatesList")
        >("../../../../ui-essentials/src/components/CandidatesList/CandidatesList").default,
        VisuallyHidden: jest.requireActual<
            typeof import("../../../../ui-essentials/src/components/VisuallyHidden/VisuallyHidden")
        >("../../../../ui-essentials/src/components/VisuallyHidden/VisuallyHidden").default,
    }),
    {virtual: true}
)
// Keep Question, AnswersList and CandidatesList real. Vote interpretation and
// individual candidate inputs are outside this category-expansion contract.
jest.mock("../../services/BallotService", () => ({
    provideBallotService: () => ({isPreferential: () => false}),
}))
jest.mock("../../store/hooks", () => ({
    useAppSelector: () => undefined,
    useAppDispatch: () => jest.fn(),
}))
jest.mock("../Answer/Answer", () => ({
    Answer: ({answer}: {answer: ICandidate}) => <li>{answer.name}</li>,
}))
jest.mock("../InvalidErrorsList/InvalidErrorsList", () => ({
    InvalidErrorsList: () => null,
    contestErrorsId: (id: string) => `errors-${id}`,
}))

const CATEGORY_NAMES = ["__proto__", "constructor", "toString", "Regular category"]

function renderCategories(policy: ECollapsibleLists) {
    const ballot = structuredClone(ELECTION_WITH_INVALID)
    const question = ballot.contests[0]
    question.presentation = {collapsible_lists: policy, types_presentation: {}}
    question.candidates = CATEGORY_NAMES.map((name, index) => ({
        ...question.candidates[0],
        id: `candidate-${index}`,
        name: `Candidate ${index}`,
        candidate_type: name,
        presentation: {},
    }))
    const ballotStyle: IBallotStyle = {
        id: ballot.id,
        election_id: ballot.election_id,
        election_event_id: ballot.election_event_id,
        tenant_id: ballot.tenant_id,
        ballot_eml: ballot,
        created_at: "2026-01-01T00:00:00Z",
        last_updated_at: "2026-01-01T00:00:00Z",
    }
    render(
        <ThemeProvider theme={theme}>
            <Question
                ballotStyle={ballotStyle}
                question={question}
                isReview={false}
                setDecodedContests={jest.fn()}
                errorSelectionState={[]}
            />
        </ThemeProvider>
    )
}

/** Read actual buttons from the shared CandidatesList, not a mocked expansion map. */
const categoryButtons = () =>
    screen
        .getAllByRole("button")
        .filter(
            (button) => button.hasAttribute("aria-controls") && button.hasAttribute("aria-expanded")
        )

// The old lookup could mutate inherited presentation objects while rendering.
// Restore them even when an assertion fails so no later test inherits pollution.
let originalNames: Array<[object, PropertyDescriptor | undefined]>
beforeEach(() => {
    originalNames = [Object.prototype, Object, Object.prototype.toString].map((target) => [
        target,
        Object.getOwnPropertyDescriptor(target, "name"),
    ])
})
afterEach(() => {
    for (const [target, descriptor] of originalNames) {
        if (descriptor) Object.defineProperty(target, "name", descriptor)
        else Reflect.deleteProperty(target, "name")
    }
})

describe("category expansion through the UI Core consumer", () => {
    it.each([ECollapsibleLists.ENABLED_COLLAPSED, ECollapsibleLists.ENABLED_EXPANDED])(
        "honors initial state and toggle-all with %s",
        async (policy) => {
            const user = userEvent.setup()
            renderCategories(policy)
            const initiallyExpanded = policy === ECollapsibleLists.ENABLED_EXPANDED
            const toggle = screen.getByRole("button", {
                name: initiallyExpanded ? "candidatesList.collapseAll" : "candidatesList.expandAll",
            })
            const categories = categoryButtons().filter((button) => button !== toggle)
            expect(categories).toHaveLength(4)
            for (const button of categories)
                expect(button).toHaveAttribute("aria-expanded", String(initiallyExpanded))

            await user.click(toggle)
            for (const button of categories)
                expect(button).toHaveAttribute("aria-expanded", String(!initiallyExpanded))
            await user.click(toggle)
            for (const button of categories)
                expect(button).toHaveAttribute("aria-expanded", String(initiallyExpanded))
        }
    )

    it("allows an individual keyboard toggle and preserves built-in object properties", async () => {
        const user = userEvent.setup()
        renderCategories(ECollapsibleLists.ENABLED_COLLAPSED)
        const firstCategory = categoryButtons()[0]
        expect(firstCategory).toBeDefined()
        firstCategory?.focus()
        await user.keyboard("{Enter}")
        await waitFor(() => expect(firstCategory).toHaveAttribute("aria-expanded", "true"))
        for (const [target, descriptor] of originalNames)
            expect(Object.getOwnPropertyDescriptor(target, "name")).toEqual(descriptor)
    })
})

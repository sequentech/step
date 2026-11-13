// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen} from "@testing-library/react"
import {MemoryRouter, Route, Routes} from "react-router-dom"
import {EInstructionsStepPolicy} from "@sequentech/ui-core"
import {EBallotStep} from "@sequentech/ui-essentials"
import Stepper from "./Stepper"

jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))
jest.mock("@sequentech/ui-essentials", () => ({
    EBallotStep: {INSTRUCTIONS: "breadcrumbSteps.instructions"},
    BallotSteps: (props: {
        selected: string | number
        withElectionList: boolean
        withInstructions: boolean
    }) => (
        <div
            data-testid="steps"
            data-selected={String(props.selected)}
            data-list={String(props.withElectionList)}
            data-instructions={String(props.withInstructions)}
        />
    ),
}))
jest.mock("../store/hooks", () => ({
    useAppSelector: (selector: () => unknown) => selector(),
}))
jest.mock("../store/extra/extraSlice", () => ({
    selectBypassChooser: () => () => false,
}))
jest.mock("../store/ballotStyles/ballotStylesSlice", () => ({
    selectFirstBallotStyle: () => mockBallotStyle,
}))
jest.mock("../store/electionEvents/electionEventsSlice", () => ({
    selectElectionEventById: () => () => mockElectionEvent,
}))

let mockBallotStyle: unknown
let mockElectionEvent: unknown

const steps = (selected: EBallotStep | number = EBallotStep.INSTRUCTIONS) => {
    render(
        <MemoryRouter initialEntries={["/tenant/t/event/e/start"]}>
            <Routes>
                <Route
                    path="/tenant/:tenantId/event/:eventId/*"
                    element={<Stepper selected={selected} />}
                />
            </Routes>
        </MemoryRouter>
    )
    return screen.getByTestId("steps")
}

beforeEach(() => {
    mockBallotStyle = undefined
    mockElectionEvent = undefined
})

describe("Stepper", () => {
    it("folds the instructions into the ballot when the event sets no policy", () => {
        mockElectionEvent = {presentation: {}}
        expect(steps()).toHaveAttribute("data-instructions", "false")
    })

    it("gives the instructions their own step when the event asks for it", () => {
        mockElectionEvent = {
            presentation: {instructions_step_policy: EInstructionsStepPolicy.OWN_STEP},
        }
        const drawn = steps()
        expect(drawn).toHaveAttribute("data-instructions", "true")
        expect(drawn).toHaveAttribute("data-selected", EBallotStep.INSTRUCTIONS)
        expect(drawn).toHaveAttribute("data-list", "true")
    })

    it("prefers the published ballot style's presentation over the event's", () => {
        mockBallotStyle = {
            ballot_eml: {
                election_event_presentation: {
                    instructions_step_policy: EInstructionsStepPolicy.MERGED_WITH_BALLOT,
                },
            },
        }
        mockElectionEvent = {
            presentation: {instructions_step_policy: EInstructionsStepPolicy.OWN_STEP},
        }
        expect(steps()).toHaveAttribute("data-instructions", "false")
    })

    it("passes a numbered step through", () => {
        expect(steps(2)).toHaveAttribute("data-selected", "2")
    })
})

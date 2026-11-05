// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"

import {ProblemList} from "../ProblemList"
import type {Problem} from "../types"

/**
 * What an import found wrong, as the Admin Portal and the Election Architect both
 * show it. Switch the toolbar locale: every named problem is translated from the
 * `problems.*` catalogue in `ui-core`, and an unnamed one falls back to the core's
 * English.
 */
const meta: Meta<typeof ProblemList> = {
    title: "components/ProblemList",
    component: ProblemList,
    parameters: {
        backgrounds: {default: "white"},
    },
}

export default meta

type Story = StoryObj<typeof ProblemList>

const weightOutOfRange: Problem = {
    severity: "error",
    code: "invalid_value",
    path: "row 4 column 'vote-weight'",
    message: "the vote weight 0 on row 4 must be between 1 and 100",
    id: "voters.vote-weight-out-of-range",
    details: {row: "4", value: "0", min: "1", max: "100"},
}

const missingElection: Problem = {
    severity: "error",
    code: "dangling_reference",
    path: "contests[0].election_id",
    message: "contest 'president' belongs to an election that is not in this file",
    id: "contest.election-missing",
    external_id: "president",
}

const graceZero: Problem = {
    severity: "warning",
    code: "invalid_value",
    path: "elections[0].grace_period",
    message: "a grace period of zero",
    id: "election.grace-zero",
}

/** Nothing wrong, said out loud: silence reads as "it did not run". */
export const Empty: Story = {
    args: {report: {problems: []}},
}

/** A voters file refused, the Admin Portal's case: no link, the lead in bold. */
export const ImportRefused: Story = {
    args: {report: {problems: [weightOutOfRange]}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByTestId("problem-group-error")).toBeInTheDocument()
        await expect(canvas.getAllByTestId("problem")).toHaveLength(1)
        // Neither the path nor the code is printed.
        await expect(canvas.queryByText(/vote-weight'$/)).not.toBeInTheDocument()
        await expect(canvas.queryByText("invalid_value")).not.toBeInTheDocument()
    },
}

/** Errors and warnings apart, with a way to go to each: the Election Architect's case. */
export const WithGoTo: Story = {
    args: {
        report: {problems: [missingElection, graceZero]},
        onGoTo: fn(),
    },
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByTestId("problem-group-warning")).toBeInTheDocument()
        await expect(canvas.getByText("president")).toBeInTheDocument()
        await userEvent.click(canvas.getByTestId("goto-contests[0].election_id"))
        await expect(args.onGoTo).toHaveBeenCalledWith("contests[0].election_id")
    },
}

/** Under a strict build a warning blocks, and is coloured as one. */
export const Strict: Story = {
    args: {report: {problems: [graceZero]}, strict: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByTestId("problem-group-error")).toBeInTheDocument()
    },
}

/** A problem the core has not named: its own English, never a key. */
export const Unnamed: Story = {
    args: {
        report: {
            problems: [
                {
                    severity: "error",
                    code: "invalid_value",
                    path: "file",
                    message: "something the core has no name for yet",
                },
            ],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("something the core has no name for yet")).toBeInTheDocument()
    },
}

const missingPrompts = (language: string, prompts: string[]): Problem => ({
    severity: "error",
    code: "MissingField",
    path: "ivr.prompts",
    message: `${prompts.join(", ")} have no words in '${language}'`,
    id: "ivr.missing-prompts",
    details: {language, prompts: prompts.join(", "), count: String(prompts.length)},
})

/**
 * A sentence that agrees with what it counts: one missing prompt "has" no words,
 * two "have" none. The Call Emulator once said "greeting have no words in 'es'".
 */
export const CountedInTheSingularAndPlural: Story = {
    args: {
        report: {
            problems: [
                missingPrompts("es", ["greeting"]),
                missingPrompts("fr", ["greeting", "declaration_text"]),
            ],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const [one, two] = canvas.getAllByTestId("problem")
        await expect(one).toHaveTextContent(
            "Call prompt missing — greeting has no words in 'es', and the telephone system refuses every call until it does."
        )
        await expect(two).toHaveTextContent(
            "Call prompts missing — greeting, declaration_text have no words in 'fr', and the telephone system refuses every call until they do."
        )
    },
}

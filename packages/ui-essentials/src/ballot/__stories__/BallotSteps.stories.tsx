// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Meta, StoryObj} from "@storybook/react"

import {BallotSteps} from "../BallotSteps"

/**
 * The voter's breadcrumb, with the portal's own four steps.
 *
 * `BreadCrumbSteps` next door is the general one — any labels, any count. This is the
 * ballot's: the wording, the numbering and what happens to both when an event has a
 * single election and skips the list. Both the voting portal and the Election
 * Architect's Ballot Preview draw *this* one.
 */
const meta: Meta<typeof BallotSteps> = {
    title: "ballot/BallotSteps",
    component: BallotSteps,
    parameters: {
        backgrounds: {
            default: "white",
        },
    },
}

export default meta

type Story = StoryObj<typeof BallotSteps>

/** Where a voter starts: choosing which ballot to vote. */
export const BallotList: Story = {
    args: {selected: 0},
}

/** Marking the ballot itself. */
export const Ballot: Story = {
    args: {selected: 1},
}

export const Review: Story = {
    args: {selected: 2},
}

export const Confirmation: Story = {
    args: {selected: 3},
}

/**
 * One election, so no list to choose from — three steps, and the caller still says
 * `selected={1}` for the ballot.
 */
export const WithoutTheBallotList: Story = {
    args: {selected: 1, withElectionList: false},
}

/** The audit screen draws the current step as a warning. */
export const Warning: Story = {
    args: {selected: 2, warning: true},
}

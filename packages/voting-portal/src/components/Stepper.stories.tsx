// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {Provider} from "react-redux"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import Stepper from "./Stepper"
import {clearVoterSession, store} from "../store/store"
import {setBypassChooser} from "../store/extra/extraSlice"

const meta = {
    title: "Voting/Progress",
    component: Stepper,
    args: {selected: 2},
    loaders: [
        ({parameters}) => {
            store.dispatch(clearVoterSession())
            store.dispatch(setBypassChooser(Boolean(parameters.bypassChooser)))
            return {}
        },
    ],
    decorators: [
        (Story) => (
            <Provider store={store}>
                <main>
                    <h1>Voting progress</h1>
                    <Story />
                </main>
            </Provider>
        ),
    ],
} satisfies Meta<typeof Stepper>
export default meta
type Story = StoryObj<typeof meta>

export const ReviewStep: Story = {
    play: async ({canvasElement}) => {
        const list = within(within(canvasElement).getByRole("list", {name: "Voting progress"}))
        await expect(list.getAllByRole("listitem")).toHaveLength(4)
        await expect(list.getByText("Review").closest("li")).toHaveAttribute("aria-current", "step")
        await expect(list.getByText("Ballot").closest("li")).not.toHaveAttribute("aria-current")
    },
}

export const BypassedChooser: Story = {
    parameters: {bypassChooser: true},
    play: async ({canvasElement}) => {
        const list = within(within(canvasElement).getByRole("list", {name: "Voting progress"}))
        await expect(list.getAllByRole("listitem")).toHaveLength(3)
        await expect(list.queryByText("Ballots")).not.toBeInTheDocument()
        await expect(list.getByText("Review").closest("li")).toHaveAttribute("aria-current", "step")
    },
}

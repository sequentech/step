// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {resultsFixture} from "../../../tests/fixtures/results"
import {ResultsPageContent} from "../ResultsPageContent"

const fixture = resultsFixture()
const meta = {
    title: "Results/Publication page",
    component: ResultsPageContent,
    args: {manifest: fixture.manifest, dataset: fixture.dataset, onElectionChange: fn()},
    decorators: [
        (Story) => (
            <main>
                <Story />
            </main>
        ),
    ],
    parameters: {
        expectedFailure: {
            reason: "The shared participation summary renders an empty column header.",
            a11y: ["empty-table-header"],
        },
    },
} satisfies Meta<typeof ResultsPageContent>
export default meta
type Story = StoryObj<typeof meta>

export const SwitchElectionAndArea: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByRole("heading", {name: "Community Election Results"})
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("tab", {name: "North district"}))
        await expect(canvas.getByRole("row", {name: /Alice Example/})).toHaveTextContent("18")
        await userEvent.click(canvas.getByRole("tab", {name: "School Board"}))
        await expect(canvas.getByRole("row", {name: /Charlie Example/})).toHaveTextContent("20")
        await expect(canvas.queryByRole("tab", {name: "North district"})).not.toBeInTheDocument()
        await expect(args.onElectionChange).toHaveBeenLastCalledWith("school")
        await userEvent.click(canvas.getByRole("tab", {name: "Community Council"}))
        await expect(canvas.getByRole("tab", {name: "Global"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(canvas.getByRole("row", {name: /Alice Example/})).toHaveTextContent("45")
    },
}

export const InitialElection: Story = {
    args: {initialElectionId: "school"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByRole("tab", {name: "School Board"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(canvas.getByRole("row", {name: /Charlie Example/})).toHaveTextContent("20")
        await expect(canvas.queryByRole("row", {name: /Alice Example/})).not.toBeInTheDocument()
    },
}

export const AreaBasedPublication: Story = {
    args: {
        manifest: {
            ...fixture.manifest,
            visibility_scope: "area_based",
            access: "authenticated",
            election_ids: ["council"],
            contests: [fixture.manifest.contests[1]],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.queryByRole("tab", {name: "Global"})).not.toBeInTheDocument()
        await expect(canvas.getByRole("tab", {name: "North district"})).toHaveAttribute(
            "aria-selected",
            "true"
        )
        await expect(canvas.getByRole("row", {name: /Alice Example/})).toHaveTextContent("18")
        await expect(canvas.getByText("Signed-in access")).toBeVisible()
    },
}

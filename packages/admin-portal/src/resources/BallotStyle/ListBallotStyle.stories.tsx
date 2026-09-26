// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {TENANT_ID} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ListBallotStyle} from "./ListBallotStyle"
import {
    BallotStyleFixture,
    reads,
    setUpBallotStyles,
    southBallot,
    type BallotStyleServices,
} from "./__stories__/BallotStyleFixture"

const meta = {
    title: "Admin/Ballot style/ListBallotStyle",
    component: ListBallotStyle,
    args: {reads: "records", empty: false},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {
        expectedFailure: {
            reason: "React-admin row selection labels a MUI 7 span instead of its checkbox.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: ({args}) => setUpBallotStyles(args),
    render: () => (
        <BallotStyleFixture>
            <ListBallotStyle />
        </BallotStyleFixture>
    ),
} satisfies WidgetMeta<BallotStyleServices>
export default meta
type Story = StoryObj<BallotStyleServices>

const row = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(name)})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const north = await row(canvasElement, "North district")
        await expect(within(north).getByText("PUBLISHED")).toBeVisible()
        await expect(await row(canvasElement, "South district")).toBeVisible()
        expect(reads("getList", "sequent_backend_ballot_style")[0].args[1]).toMatchObject({
            filter: {tenant_id: TENANT_ID},
        })
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {
        expectedFailure: {
            reason: "React-admin's default empty page greys its message below the contrast minimum.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("No Ballot Styles yet.")).toBeVisible()
        expect(within(canvasElement).queryByRole("row", {name: /district/})).toBeNull()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(reads("getList", "sequent_backend_ballot_style")).toHaveLength(1)
        )
        expect(within(canvasElement).queryByRole("row", {name: /district/})).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    parameters: {expectedFailure: null},
    play: async ({canvasElement}) => {
        const message = await within(document.body).findByText("Synthetic service unavailable")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("row", {name: /district/})).toBeNull()
    },
}

export const RowOpensTheBallotStyle: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(await row(canvasElement, "South district"))
        const filter = JSON.stringify({
            election_event_id: southBallot.election_event_id,
            election_id: southBallot.election_id,
            area_id: southBallot.area_id,
        })
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(`/sequent_backend_ballot_style/${southBallot.id}?filter=${filter}`)
        )
    },
}

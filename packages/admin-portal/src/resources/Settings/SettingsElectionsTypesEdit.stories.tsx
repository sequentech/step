// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary, type ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingselectionsTypesEdit} from "./SettingsElectionsTypesEdit"
import {
    ELECTION_TYPE_RESOURCE,
    REFERENDUM_TYPE_ID,
    electionTypeRecords,
} from "./__stories__/SettingsFixture"

interface Scenario {
    /** What reading the election type does. */
    reads: ReadState
    /** Whether saving the type fails. */
    failure: boolean
    /** Called when the drawer holding the form should close. */
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingselectionsTypesEdit",
    component: SettingselectionsTypesEdit,
    args: {reads: "records", failure: false, close: fn()},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[ELECTION_TYPE_RESOURCE]: electionTypeRecords()},
            {
                reads: args.reads,
                writeError: args.failure ? "Synthetic election type failure" : undefined,
            }
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <SettingselectionsTypesEdit id={REFERENDUM_TYPE_ID} close={close} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function nameInput(canvasElement: HTMLElement) {
    const input = await within(canvasElement).findByRole("textbox", {name: "Name"})
    await waitFor(() => expect(input).toHaveValue("Referendum"))
    return input
}

async function rename(canvasElement: HTMLElement) {
    const input = await nameInput(canvasElement)
    await userEvent.clear(input)
    await userEvent.type(input, "Citizens' initiative")
    await userEvent.click(within(canvasElement).getByRole("button", {name: "Save"}))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await nameInput(canvasElement)
        await expect(
            within(canvasElement).getByText(i18n.t("electionTypeScreen.edit.title"))
        ).toBeVisible()
        expect(data.calls.map(({method, args}) => [method, args[0]])).toEqual([
            ["getOne", ELECTION_TYPE_RESOURCE],
        ])
    },
}

export const RenameAType: Story = {
    play: async ({canvasElement, args}) => {
        await rename(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledOnce())
        expect(data.writes).toEqual([
            {
                method: "update",
                resource: ELECTION_TYPE_RESOURCE,
                params: expect.objectContaining({
                    id: REFERENDUM_TYPE_ID,
                    data: expect.objectContaining({name: "Citizens' initiative"}),
                }),
            },
        ])
    },
}

export const SaveFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await rename(canvasElement)
        const message = await within(document.body).findByText("Synthetic election type failure")
        await waitFor(() => expect(message).toBeVisible())
        expect(args.close).toHaveBeenCalledOnce()
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(data.calls.map(({method}) => method)).toContain("getOne"))
        expect(within(canvasElement).queryByRole("textbox")).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement, args}) => {
        const message = await within(document.body).findByText("Element does not exist")
        await waitFor(() => expect(message).toBeVisible())
        expect(within(canvasElement).queryByRole("textbox")).toBeNull()
        expect(args.close).not.toHaveBeenCalled()
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsElectionsTypesCreate} from "./SettingsElectionsTypesCreate"
import {
    ELECTION_TYPE_RESOURCE,
    TENANT_RESOURCE,
    settingsTenant,
} from "./__stories__/SettingsFixture"

interface Scenario {
    /** Whether creating the type fails. */
    failure: boolean
    /** Called when the drawer holding the form should close. */
    close: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Settings/SettingsElectionsTypesCreate",
    component: SettingsElectionsTypesCreate,
    args: {failure: false, close: fn()},
    beforeEach: async ({args}) => {
        data = resourceBoundary(
            {[ELECTION_TYPE_RESOURCE]: [], [TENANT_RESOURCE]: [settingsTenant()]},
            {writeError: args.failure ? "Synthetic election type failure" : undefined}
        )
        graphql = graphqlBoundary({}, {schema: true})
        await graphql.ready
    },
    render: ({close}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider}>
            <ResourceContextProvider value={ELECTION_TYPE_RESOURCE}>
                <SettingsElectionsTypesCreate close={close} />
            </ResourceContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function createByElection(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await expect(await canvas.findByText(i18n.t("electionTypeScreen.create.title"))).toBeVisible()
    await userEvent.type(canvas.getByRole("textbox", {name: "Name"}), "By-election")
    await userEvent.click(canvas.getByRole("button", {name: "Save"}))
}

export const Empty: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("textbox", {name: "Name"})).toHaveValue("")
        expect(data.writes).toEqual([])
        expect(args.close).not.toHaveBeenCalled()
    },
}

export const CreateAType: Story = {
    play: async ({canvasElement, args}) => {
        await createByElection(canvasElement)
        await waitFor(() => expect(args.close).toHaveBeenCalledOnce())
        expect(data.writes).toEqual([
            {
                method: "create",
                resource: ELECTION_TYPE_RESOURCE,
                params: {data: {name: "By-election", tenant_id: TENANT_ID}},
            },
        ])
    },
}

export const CreateFailure: Story = {
    args: {failure: true},
    play: async ({canvasElement, args}) => {
        await createByElection(canvasElement)
        const message = await within(document.body).findByText("Synthetic election type failure")
        await waitFor(() => expect(message).toBeVisible())
        expect(data.writes.map(({method}) => method)).toEqual(["create"])
        // The drawer stays open with what was typed.
        expect(args.close).not.toHaveBeenCalled()
        expect(within(canvasElement).getByRole("textbox", {name: "Name"})).toHaveValue("By-election")
    },
}

export const RequiresAName: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("textbox", {name: "Name"})
        // Nothing to save until the name changes.
        await expect(canvas.getByRole("button", {name: "Save"})).toBeDisabled()
        expect(args.close).not.toHaveBeenCalled()
        expect(data.writes).toEqual([])
    },
}

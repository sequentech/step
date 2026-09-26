// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, waitFor, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {Route, Routes} from "react-router"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, electionRecord} from "@/__stories__/fixtures"
import type {ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionBaseTabs} from "./ElectionBaseTabs"
import {answerOrPending, paramsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const RESOURCE = "sequent_backend_election"

interface Scenario {
    /** What reading the election does. */
    reads: ReadState
    /** The election's permission label. */
    label?: string
    /** Permission labels of the signed-in user. */
    userLabels?: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

function Fixture({userLabels}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            auth={userLabels ? {permissionLabels: userLabels} : undefined}
            tenant={tenant}
        >
            <ResourceContextProvider value={RESOURCE}>
                <Routes>
                    <Route path={`/${RESOURCE}/:id`} element={<ElectionBaseTabs />} />
                </Routes>
            </ResourceContextProvider>
        </AdminStoryProvider>
    )
}

// The tabs' contents have their own sections; their reads stay loading here.
const meta = {
    title: "Admin/Election event/ElectionBaseTabs",
    component: ElectionBaseTabs,
    args: {reads: "records"},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {router: {initialEntries: [`/${RESOURCE}/${STORY_IDS.election}`]}},
    beforeEach: async ({args}) => {
        data = recordsOrPending(
            {[RESOURCE]: [electionRecord(undefined, {permission_label: args.label ?? null})]},
            {reads: args.reads}
        )
        graphql = graphqlBoundary(answerOrPending(), {schema: true})
        await graphql.ready
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const noPermission = () => i18n.t("electionScreen.common.noPermission")

export const Populated: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    parameters: {
        expectedFailure: {
            reason: "The election dashboard's loading spinner is a progressbar without an accessible name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Council")).toBeVisible()
        const tabs = canvas.getAllByRole("tab").map((tab) => tab.textContent)
        expect(tabs).toEqual([
            i18n.t("electionScreen.tabs.dashboard"),
            i18n.t("electionScreen.tabs.data"),
            i18n.t("electionScreen.tabs.voters"),
            i18n.t("electionScreen.tabs.publish"),
            i18n.t("electionScreen.tabs.approvals"),
            i18n.t("electionScreen.tabs.tallySheets"),
        ])
        expect(paramsOf(data, "getOne", RESOURCE)).toMatchObject({id: STORY_IDS.election})
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(paramsOf(data, "getOne", RESOURCE)).toBeDefined())
        // Until the election arrives the tabs show the no-permission message.
        await expect(await within(canvasElement).findByText(noPermission())).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
    },
}

export const LoadError: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText(noPermission())).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
        expect(paramsOf(data, "getOne", RESOURCE)).toMatchObject({id: STORY_IDS.election})
    },
}

export const TrusteeTabs: Story = {
    globals: {permissions: EStoryPermissions.TRUSTEE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Council")).toBeVisible()
        // The trustee group holds none of the election tab permissions.
        expect(canvas.queryByRole("tab")).toBeNull()
    },
}

export const OutsideThePermissionLabel: Story = {
    args: {label: "north", userLabels: ["south"]},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(paramsOf(data, "getOne", RESOURCE)).toBeDefined())
        // The election has loaded, but the user holds no label that shows it.
        await waitFor(() => expect(canvas.getByText(noPermission())).toBeVisible())
        expect(canvas.queryByRole("tab")).toBeNull()
        expect(canvas.queryByText("Council")).toBeNull()
    },
}

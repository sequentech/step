// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {ResourceContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {IPermissions} from "@/types/keycloak"
import {EditReportsTab} from "./EditReportsTab"
import {ELECTIONS, REPORTS, TEMPLATES} from "./__stories__/ReportsFixture"

interface Scenario {
    roles: string[]
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const meta = {
    title: "Admin/Reports/EditReportsTab",
    component: EditReportsTab,
    args: {roles: [IPermissions.REPORT_READ]},
    parameters: {
        widgets: ["ListReports"],
        expectedFailure: {
            reason:
                "The report list's row selection checkboxes carry an aria-label on a span " +
                "with no role and their inputs have no label.",
            a11y: ["aria-prohibited-attr", "label"],
        },
    },
    beforeEach: () => {
        data = resourceBoundary({
            sequent_backend_report: REPORTS,
            sequent_backend_template: TEMPLATES,
            sequent_backend_election: ELECTIONS,
        })
        graphql = graphqlBoundary({})
    },
    render: ({roles}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <WidgetsContextProvider>
                <ResourceContextProvider value="sequent_backend_election_event">
                    <EditReportsTab electionEventId={EVENT_ID} />
                </ResourceContextProvider>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const WithReportAccess: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Reports")).toBeVisible()
        await canvas.findByRole("row", {name: / Activity Logs /})
        expect(canvas.getAllByRole("row")).toHaveLength(4)
    },
}

export const WithoutReportAccess: Story = {
    args: {roles: []},
    parameters: {widgets: [], expectedFailure: null},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        expect(canvas.queryByText("Reports")).toBeNull()
        expect(canvas.queryByRole("table")).toBeNull()
        expect(data.calls).toEqual([])
    },
}

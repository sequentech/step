// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within, waitFor} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {SettingsConfigurationPackage} from "./SettingsConfigurationPackage"
import {PACKAGE_PERMISSIONS} from "./configurationPackage"
import {record} from "./configurationPackage.fixture"

interface Scenario {
    allowed: boolean
    blocked: boolean
    unavailable: boolean
}
const DOCUMENT_ID = storyId(41, 1)
const TASK_ID = storyId(41, 2)
let graphql: ReturnType<typeof graphqlBoundary>
const meta = {
    title: "Admin/Settings/SettingsConfigurationPackage",
    component: SettingsConfigurationPackage,
    args: {allowed: true, blocked: false, unavailable: false},
    beforeEach: async ({args}) => {
        sessionStorage.clear()
        const annotations = record(
            "previewed",
            args.blocked ? ["Unknown role: synthetic-reviewer"] : []
        )
        annotations.application.tenantId = TENANT_ID
        annotations.application.packageSha256 = "ab".repeat(32)
        // A readable proposal, not credentials or arbitrary raw configuration.
        const tenantSection = annotations.application.sections[0] as {
            status: {preview: {changes: unknown[]}}
        }
        tenantSection.status.preview.changes = [{action: "update", keys: ["language_conf"]}]
        graphql = graphqlBoundary({
            ListPackageApplications: () => ({data: {sequent_backend_tasks_execution: []}}),
            ApplyConfigurationPackage: () => ({
                data: {
                    apply_configuration_package: {
                        task_execution: {
                            id: TASK_ID,
                            execution_status: "IN_PROGRESS",
                            annotations: null,
                        },
                    },
                },
            }),
            GetPackageTask: () => ({
                data: {
                    sequent_backend_tasks_execution: args.unavailable
                        ? []
                        : [
                              {
                                  id: TASK_ID,
                                  execution_status: "SUCCESS",
                                  annotations,
                                  logs: [],
                              },
                          ],
                },
            }),
        })
        await graphql.ready
        return () => sessionStorage.clear()
    },
    render: ({allowed}) => (
        <AdminStoryProvider boundary={graphql} roles={allowed ? PACKAGE_PERMISSIONS : []}>
            <SettingsConfigurationPackage />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function preview(canvasElement: HTMLElement) {
    const canvas = within(canvasElement)
    await userEvent.type(
        canvas.getByRole("textbox", {name: i18n.t("configurationPackage.documentId")}),
        DOCUMENT_ID
    )
    await userEvent.click(
        canvas.getByRole("button", {name: i18n.t("configurationPackage.preview")})
    )
    await canvas.findByTestId("package-task")
}

export const PreviewAndConfirm: Story = {
    parameters: {widgets: ["ApplicationTable", "ProposedChanges"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByTestId("package-apply")).toBeDisabled()
        await preview(canvasElement)
        await waitFor(() => expect(canvas.getByTestId("package-apply")).toBeEnabled())
        await expect(canvas.getByTestId("package-members")).toHaveTextContent(
            "export_voters-e1.csv"
        )
        await expect(canvas.getByTestId("section-tenant-configuration")).toHaveTextContent(
            "language_conf"
        )
        await userEvent.click(canvas.getByTestId("package-apply"))
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(dialog.getByTestId("package-proposed-changes")).toHaveTextContent(
            "language_conf"
        )
        // Opening confirmation never applies; the only mutation so far is preview.
        expect(
            graphql.calls
                .filter(({name}) => name === "ApplyConfigurationPackage")
                .map(({variables}) => variables.mode)
        ).toEqual(["preview"])
        expect(graphql.unexpected).toEqual([])
    },
}

export const BlockedPreview: Story = {
    args: {blocked: true},
    parameters: {widgets: ["ApplicationTable"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await preview(canvasElement)
        await expect(await canvas.findByTestId("section-tenant-configuration")).toHaveTextContent(
            "Unknown role: synthetic-reviewer"
        )
        await expect(await canvas.findByText(i18n.t("configurationPackage.blocked"))).toBeVisible()
        await expect(canvas.getByTestId("package-apply")).toBeDisabled()
        expect(graphql.calls.filter(({name}) => name === "ApplyConfigurationPackage")).toHaveLength(
            1
        )
        expect(graphql.unexpected).toEqual([])
    },
}

export const TaskUnavailable: Story = {
    args: {unavailable: true},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await preview(canvasElement)
        await waitFor(() =>
            expect(canvas.getByTestId("package-outcome")).toHaveTextContent(
                i18n.t("configurationPackage.notFound")
            )
        )
        await expect(canvas.getByTestId("package-apply")).toBeDisabled()
        await userEvent.click(canvas.getByTestId("package-stop-following"))
        await expect(canvas.queryByTestId("package-run")).not.toBeInTheDocument()
        expect(graphql.calls.filter(({name}) => name === "ApplyConfigurationPackage")).toHaveLength(
            1
        )
        expect(graphql.unexpected).toEqual([])
    },
}

export const PermissionRequired: Story = {
    args: {allowed: false},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("package-not-permitted")).toBeVisible()
        expect(graphql.calls).toEqual([])
    },
}

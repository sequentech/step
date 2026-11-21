// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, TENANT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ArchitectSignerRoster} from "./ArchitectSignerRoster"

let graphql: ReturnType<typeof graphqlBoundary>
type Props = React.ComponentProps<typeof ArchitectSignerRoster>
const meta = {
    title: "Admin/Settings/ArchitectSignerRoster",
    component: ArchitectSignerRoster,
    args: {
        tenantId: TENANT_ID,
        loading: false,
        status: {
            mode: "portal_managed",
            generation: "0",
            active_sha256: null,
            required_approvals: 2,
            can_manage: true,
        },
        refresh: async () => null,
    },
    beforeEach: () => {
        graphql = graphqlBoundary({})
    },
    render: (args) => (
        <AdminStoryProvider boundary={graphql} auth={{userId: "story-operator"}}>
            <ArchitectSignerRoster {...args} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Props>
export default meta
type Story = StoryObj<Props>

export const PrepareRoster: Story = {
    parameters: {widgets: ["RosterSetup"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText(i18n.t("configurationPackage.roster.noRoster"))).toBeVisible()
        await userEvent.click(
            canvas.getByRole("button", {name: i18n.t("configurationPackage.roster.manage")})
        )
        const dialog = within(await within(document.body).findByRole("dialog"))
        await expect(
            dialog.getByLabelText(i18n.t("configurationPackage.roster.file"))
        ).toBeVisible()
        await expect(
            dialog.getByRole("button", {name: i18n.t("configurationPackage.roster.review")})
        ).toBeDisabled()
        await expect(
            dialog.queryByRole("button", {name: i18n.t("configurationPackage.roster.confirm")})
        ).not.toBeInTheDocument()
        expect(graphql.calls).toEqual([])
    },
}

export const LockedSitePin: Story = {
    args: {
        status: {
            mode: "site_pinned",
            generation: "0",
            active_sha256: "ab".repeat(32),
            required_approvals: 2,
            can_manage: false,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText(i18n.t("configurationPackage.roster.sitePinned"))
        ).toBeVisible()
        await expect(
            canvas.queryByRole("button", {name: i18n.t("configurationPackage.roster.manage")})
        ).not.toBeInTheDocument()
        expect(graphql.calls).toEqual([])
    },
}

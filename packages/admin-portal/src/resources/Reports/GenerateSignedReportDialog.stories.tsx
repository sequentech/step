// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {GenerateSignedReportDialog} from "./ReportSigning"

interface Scenario {
    /** The Post and how many sign: the configuration under test. */
    post: string
    needs: number
    onClose: (generate: boolean) => void
}

let boundary: ReturnType<typeof graphqlBoundary>

const meta = {
    title: "Admin/Reports/GenerateSignedReportDialog",
    component: GenerateSignedReportDialog,
    args: {post: "Madrid PE", needs: 3, onClose: fn()},
    beforeEach: async () => {
        boundary = graphqlBoundary({})
        await boundary.ready
    },
    render: ({post, needs, onClose}) => (
        <AdminStoryProvider boundary={boundary}>
            <GenerateSignedReportDialog
                open
                title={String(i18n.t("template.type.PARTICIPATION_REPORT"))}
                post={post}
                needs={needs}
                onClose={onClose}
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

async function dialogOf() {
    const dialog = await within(document.body).findByRole("dialog")
    await waitFor(() => expect(dialog).toBeVisible())
    return dialog
}

export const SaysTheReportWaitsForSignatures: Story = {
    play: async ({args}) => {
        const dialog = await dialogOf()
        await expect(
            within(dialog).getByText(
                i18n.t("signing.reports.generateNotice", {post: args.post, n: args.needs})
            )
        ).toBeVisible()
        await userEvent.click(
            within(dialog).getByRole("button", {name: i18n.t("reportsScreen.actions.generate")})
        )
        expect(args.onClose).toHaveBeenCalledWith(true)
    },
}

/** A second organization: another Post and count. */
export const AnotherOrganizationCancels: Story = {
    args: {post: "Faculty of Science", needs: 2},
    play: async ({args}) => {
        const dialog = await dialogOf()
        await expect(
            within(dialog).getByText(
                i18n.t("signing.reports.generateNotice", {post: "Faculty of Science", n: 2})
            )
        ).toBeVisible()
        await userEvent.click(
            within(dialog).getByRole("button", {name: i18n.t("common.label.cancel")})
        )
        expect(args.onClose).toHaveBeenCalledWith(false)
    },
}

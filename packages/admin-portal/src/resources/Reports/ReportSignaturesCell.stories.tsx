// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ReportSignaturesCell, type ReportSignatures} from "./ReportSigning"

interface Scenario {
    needs: ReportSignatures
}

const meta = {
    title: "Admin/Reports/ReportSignaturesCell",
    component: ReportSignaturesCell,
    args: {needs: 3},
    render: ({needs}) => (
        <AdminStoryProvider boundary={graphqlBoundary({})}>
            <span data-testid="cell">
                <ReportSignaturesCell needs={needs} />
            </span>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const NeedsSignatures: Story = {
    play: async ({canvasElement, args}) => {
        await expect(within(canvasElement).getByTestId("cell")).toHaveTextContent(
            i18n.t("signing.results.needs", {n: args.needs})
        )
    },
}

/** A second configuration: another count. */
export const NeedsTwo: Story = {
    args: {needs: 2},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("cell")).toHaveTextContent(
            i18n.t("signing.results.needs", {n: 2})
        )
    },
}

export const Off: Story = {
    args: {needs: 0},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("cell")).toHaveTextContent(
            i18n.t("signing.results.off")
        )
    },
}

export const TakesNoSignatures: Story = {
    args: {needs: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByTestId("cell")).toHaveTextContent("-")
    },
}

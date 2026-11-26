// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {Button} from "@mui/material"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {PackageImportWorkflow, type PackageImportPhase} from "./PackageImportWorkflow"

interface Scenario {
    phase: PackageImportPhase
}

const meta = {
    title: "Admin/Settings/PackageImportWorkflow",
    component: PackageImportWorkflow,
    args: {phase: "choose"},
    render: ({phase}) => (
        <AdminStoryProvider>
            <PackageImportWorkflow
                phase={phase}
                uploadAction={
                    <Button
                        variant={phase === "review" ? "secondary" : "contained"}
                        disabled={phase === "trustBlocked"}
                    >
                        {i18n.t("configurationPackage.upload")}
                    </Button>
                }
                applyAction={
                    phase === "review" ? (
                        <Button>{i18n.t("configurationPackage.apply")}</Button>
                    ) : null
                }
            />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

export const Upload: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByTestId("package-workflow-current")).toHaveAttribute(
            "aria-current",
            "step"
        )
        await expect(canvas.getByTestId("package-workflow-current")).toHaveTextContent(
            i18n.t("configurationPackage.workflow.steps.upload")
        )
        await expect(
            canvas.queryByRole("button", {name: i18n.t("configurationPackage.apply")})
        ).not.toBeInTheDocument()
    },
}

export const Review: Story = {
    args: {phase: "review"},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByTestId("package-workflow-current")
        ).toHaveTextContent(i18n.t("configurationPackage.workflow.steps.review"))
    },
}

export const TrustBlocked: Story = {
    args: {phase: "trustBlocked"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.queryByTestId("package-workflow-current")).not.toBeInTheDocument()
        await expect(
            canvas.getByRole("button", {name: i18n.t("configurationPackage.upload")})
        ).toBeDisabled()
    },
}

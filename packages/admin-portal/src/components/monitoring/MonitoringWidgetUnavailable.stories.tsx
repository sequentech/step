// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {EProblemSeverity, EWidgetFailure, EWidgetRenderState} from "./types"
import {MonitoringWidgetUnavailable} from "./MonitoringWidgetUnavailable"
import {summaryTable, turnoutTable} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringWidgetUnavailable",
    component: MonitoringWidgetUnavailable,
    args: {
        state: EWidgetRenderState.NOT_CONNECTED,
        reason: "ATTACK_DETECTION_FEED",
        title: "Attack detections",
    },
} satisfies Meta<typeof MonitoringWidgetUnavailable>
export default meta
type Story = StoryObj<typeof meta>

export const NotConnected: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            canvas.getByText("Not connected · no attack detection feed is connected")
        ).toBeVisible()
        await expect(canvas.getByText("Nothing is shown until it is.")).toBeVisible()
        expect(canvas.queryByText("0")).toBeNull()
    },
}

export const NotConnectedUnknownReason: Story = {
    args: {reason: "A_FUTURE_PRODUCER"},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("Not connected · A_FUTURE_PRODUCER")
        ).toBeVisible()
    },
}

export const NoSnapshot: Story = {
    args: {state: EWidgetRenderState.NO_SNAPSHOT, reason: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Not counted yet")).toBeVisible()
    },
}

export const ScopePending: Story = {
    args: {state: EWidgetRenderState.SCOPE_PENDING, reason: null},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Counting this selection")).toBeVisible()
    },
}

export const RenderFailedShowsTable: Story = {
    args: {state: EWidgetRenderState.RENDER_FAILED, reason: null, table: turnoutTable},
    parameters: {widgets: ["Fallback"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("The chart could not be drawn")).toBeVisible()
        await expect(canvas.getByRole("table", {name: "Attack detections"})).toBeVisible()
    },
}

export const RenderFailedShowsFigures: Story = {
    args: {state: EWidgetRenderState.RENDER_FAILED, reason: null, table: summaryTable},
    parameters: {widgets: ["Fallback"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("874,624")).toBeVisible()
        await expect(canvas.getByText("53.2%")).toBeVisible()
        expect(canvas.queryByRole("table")).toBeNull()
    },
}

export const Invalid: Story = {
    args: {
        state: EWidgetRenderState.INVALID,
        reason: null,
        diagnostics: [
            {
                severity: EProblemSeverity.ERROR,
                code: "UNKNOWN_OPTION",
                path: "measure",
                message: "'voted_all' is not an option of 'measure'.",
            },
        ],
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getByText("This widget cannot be shown")).toBeVisible()
        await expect(canvas.getByText("'voted_all' is not an option of 'measure'.")).toBeVisible()
    },
}

export const RequestFailed: Story = {
    args: {state: EWidgetFailure.REQUEST_FAILED, reason: null},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("This widget could not be loaded")
        ).toBeVisible()
    },
}

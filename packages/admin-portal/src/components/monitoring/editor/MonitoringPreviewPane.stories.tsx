// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {MonitoringPreviewPane} from "./MonitoringPreviewPane"
import {EPreviewStatus} from "./yamlDraft"
import {EMonitoringRenderState} from "./types"
import {RENDERED} from "./storyFixtures"

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringPreviewPane",
    component: MonitoringPreviewPane,
    args: {preview: RENDERED, status: EPreviewStatus.READY, height: 160},
} satisfies Meta<typeof MonitoringPreviewPane>
export default meta
type Story = StoryObj<typeof meta>

export const Rendered: Story = {
    play: async ({canvasElement}) => {
        const frame = canvasElement.querySelector("iframe") as HTMLIFrameElement
        await expect(frame).toHaveAttribute("sandbox", "")
        expect(frame.srcdoc).toContain("Content-Security-Policy")
        expect(frame.srcdoc).toContain("<svg")
    },
}

export const ScriptIsStripped: Story = {
    args: {
        preview: {
            ...RENDERED,
            svg: `<svg xmlns="http://www.w3.org/2000/svg"><script>alert(1)</script><a href="https://example.invalid"><text>x</text></a><a href="#bar"><text>y</text></a></svg>`,
        },
    },
    play: async ({canvasElement}) => {
        const frame = canvasElement.querySelector("iframe") as HTMLIFrameElement
        expect(frame.srcdoc).not.toContain("<script")
        expect(frame.srcdoc).not.toContain("example.invalid")
        expect(frame.srcdoc).toContain('href="#bar"')
    },
}

export const NotConnected: Story = {
    args: {
        preview: {
            state: EMonitoringRenderState.NOT_CONNECTED,
            reason: "VOTE-SECOPS does not report detections yet.",
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("alert")).toHaveTextContent(
            "Not connected · VOTE-SECOPS does not report detections yet. Nothing is shown until it is."
        )
        expect(canvasElement.querySelector("iframe")).toBeNull()
    },
}

export const Failed: Story = {
    args: {preview: undefined, status: EPreviewStatus.FAILED, error: "renderer unavailable"},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("alert")).toHaveTextContent(
            "The preview could not be rendered: renderer unavailable"
        )
    },
}

export const Empty: Story = {
    args: {preview: undefined, status: EPreviewStatus.IDLE},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText("The preview appears once the YAML is valid.")
        ).toBeVisible()
    },
}

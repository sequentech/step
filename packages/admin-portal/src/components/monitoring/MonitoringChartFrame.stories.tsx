// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, within} from "storybook/test"
import {EColorScheme} from "./types"
import {CHART_CSP} from "./lib/chartDocument"
import {MonitoringChartFrame} from "./MonitoringChartFrame"
import {barChartSvg} from "./__stories__/MonitoringFixture"

const meta = {
    title: "Admin/Monitoring/MonitoringChartFrame",
    component: MonitoringChartFrame,
    args: {
        svg: barChartSvg([120, 90, 40]),
        title: "Turnout by group chart",
        colorScheme: EColorScheme.LIGHT,
    },
} satisfies Meta<typeof MonitoringChartFrame>
export default meta
type Story = StoryObj<typeof meta>

const frame = (canvasElement: HTMLElement) =>
    within(canvasElement).getByTitle("Turnout by group chart") as HTMLIFrameElement

export const Sandboxed: Story = {
    play: async ({canvasElement}) => {
        const element = frame(canvasElement)
        await expect(element).toBeVisible()
        expect(element.getAttribute("sandbox")).toBe("")
        expect(element.style.height).toBe("280px")
        const head = element.srcdoc.slice(element.srcdoc.indexOf("<head>") + 6)
        expect(
            head.startsWith(`<meta http-equiv="Content-Security-Policy" content="${CHART_CSP}">`)
        ).toBe(true)
        expect(element.srcdoc).toContain("<rect")
    },
}

export const HostileSvgIsCleaned: Story = {
    args: {
        svg:
            '<svg xmlns="http://www.w3.org/2000/svg"><script>parent.alert(1)</script>' +
            '<image href="https://tracker.invalid/pixel.png"/><rect width="10" height="10" onclick="alert(2)"/></svg>',
    },
    play: async ({canvasElement}) => {
        const {srcdoc} = frame(canvasElement)
        expect(srcdoc).not.toMatch(/script|tracker\.invalid|onclick/)
        expect(srcdoc).toContain("<rect")
    },
}

export const ConfiguredHeight: Story = {
    args: {height: 160, colorScheme: EColorScheme.DARK},
    play: async ({canvasElement}) => {
        const element = frame(canvasElement)
        expect(element.style.height).toBe("160px")
        expect(element.srcdoc).toContain("color-scheme: dark")
    },
}

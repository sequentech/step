// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import React from "react"
import {expect, waitFor, within} from "storybook/test"
import {EColorScheme} from "./types"
import {CHART_CSP} from "./lib/chartDocument"
import {CHART_MAX_HEIGHT_FACTOR} from "./lib/chartSize"
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

/** Poll status as the renderer draws it at 400 px: taller than it is wide. */
const tallSvg =
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 400 607.7" width="400" height="607.7">' +
    '<rect x="0" y="0" width="400" height="300" fill="#4a7"/>' +
    '<rect x="0" y="560" width="400" height="47.7" fill="#a47"/></svg>'

/** The frame in a column of a known width, as in a widget card. */
const column =
    (width: number) =>
    (Story: () => React.JSX.Element): React.JSX.Element => (
        <div style={{width}}>
            <Story />
        </div>
    )

const frame = (canvasElement: HTMLElement) =>
    within(canvasElement).getByTitle("Turnout by group chart") as HTMLIFrameElement

export const Sandboxed: Story = {
    play: async ({canvasElement}) => {
        const element = frame(canvasElement)
        await expect(element).toBeVisible()
        expect(element.getAttribute("sandbox")).toBe("")
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
    args: {
        svg: '<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100"><rect width="10" height="10"/></svg>',
        height: 160,
        colorScheme: EColorScheme.DARK,
    },
    play: async ({canvasElement}) => {
        const element = frame(canvasElement)
        expect(element.style.height).toBe("160px")
        expect(element.srcdoc).toContain("color-scheme: dark")
    },
}

/** A chart taller than the widget height gets a frame as tall as the chart. */
export const TallChartGrows: Story = {
    args: {svg: tallSvg, height: 400},
    decorators: [column(520)],
    play: async ({canvasElement}) => {
        const element = frame(canvasElement)
        // Drawn at its own 400 px width, 607.7 px tall.
        await waitFor(() => expect(element.style.height).toBe("608px"))
        expect(element.getBoundingClientRect().height).toBeCloseTo(608, 0)
    },
}

/** In a narrow column the chart is narrowed, and its frame shortened with it. */
export const TallChartNarrowed: Story = {
    args: {svg: tallSvg, height: 400},
    decorators: [column(300)],
    play: async ({canvasElement}) => {
        const element = frame(canvasElement)
        await waitFor(() =>
            expect(element.style.height).toBe(`${Math.ceil(300 * (607.7 / 400))}px`)
        )
    },
}

/** Past twice the widget height the frame stops growing and scrolls. */
export const TallChartCapped: Story = {
    args: {svg: tallSvg, height: 200},
    decorators: [column(520)],
    play: async ({canvasElement}) => {
        const element = frame(canvasElement)
        await waitFor(() => expect(element.style.height).toBe(`${CHART_MAX_HEIGHT_FACTOR * 200}px`))
        expect(element.srcdoc).toContain("overflow-y: auto")
    },
}

/** A chart with no viewBox keeps the widget height. */
export const NoViewBoxKeepsWidgetHeight: Story = {
    args: {
        svg: '<svg xmlns="http://www.w3.org/2000/svg" width="400" height="900"><rect width="10" height="10"/></svg>',
    },
    decorators: [column(520)],
    play: async ({canvasElement}) => {
        expect(frame(canvasElement).style.height).toBe("280px")
    },
}

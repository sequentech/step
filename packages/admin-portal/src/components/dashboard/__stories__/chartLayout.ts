// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {expect, waitFor, within} from "storybook/test"

/** Check the rendered, populated charts inside each real standard dashboard. */
export const expectDashboardChartLayout = async (canvasElement: HTMLElement) => {
    const canvas = within(canvasElement)
    const title = await canvas.findByText("Votes over time")
    const votes = title.closest<HTMLElement>(".MuiPaper-root")
    const voters = canvas.getByText("Voters by channel").closest<HTMLElement>(".MuiPaper-root")
    const row = votes?.parentElement
    if (!votes || !voters || !row) throw new Error("Both charts must share the dashboard row")
    await waitFor(() => {
        const bounds = row.getBoundingClientRect()
        const first = votes.getBoundingClientRect()
        const second = voters.getBoundingClientRect()
        expect(first.left).toBeCloseTo(bounds.left, 0)
        expect(second.right).toBeCloseTo(bounds.right, 0)
        expect(first.width).toBeCloseTo(second.width, 0)
        if (Math.abs(first.top - second.top) < 1) {
            expect(second.left - first.right).toBeGreaterThanOrEqual(16)
            expect(first.height).toBeCloseTo(second.height, 0)
        } else {
            expect(first.width).toBeCloseTo(bounds.width, 0)
            expect(second.top - first.bottom).toBeGreaterThanOrEqual(16)
        }
        for (const card of [votes, voters]) {
            const chart = card.querySelector<HTMLElement>(".apexcharts-canvas")
            if (!chart) throw new Error("The populated chart must be rendered")
            const style = getComputedStyle(card)
            const contentWidth =
                card.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight)
            expect(chart.getBoundingClientRect().width).toBeCloseTo(contentWidth, 0)
            expect(card.scrollWidth).toBeLessThanOrEqual(card.clientWidth + 1)
        }
        expect(title.parentElement?.getBoundingClientRect().height).toBeLessThanOrEqual(32)
    })
}

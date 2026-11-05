// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {editorDiagnostics} from "@/components/monitoring/lib/diagnostics"
import {parseYamlText} from "@/components/monitoring/lib/yamlPatch"
import {MonitoringDiagnosticsList} from "./MonitoringDiagnosticsList"
import {CHART_WARNING, FORBIDDEN_KEY, WIDGET_YAML} from "./storyFixtures"

const problems = editorDiagnostics(WIDGET_YAML, parseYamlText(WIDGET_YAML), {
    local: [FORBIDDEN_KEY],
    server: [CHART_WARNING],
})

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringDiagnosticsList",
    component: MonitoringDiagnosticsList,
    args: {diagnostics: problems, onSelect: fn()},
} satisfies Meta<typeof MonitoringDiagnosticsList>
export default meta
type Story = StoryObj<typeof meta>

export const Problems: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        const error = canvas.getByRole("button", {name: /^Error: A query only chooses/})
        await expect(error).toBeVisible()
        await expect(within(error).getByText("Browser check")).toBeVisible()
        const warning = canvas.getByRole("button", {name: /^Warning: Bars sorted/})
        await expect(within(warning).getByText("dbt Charts WARN-SORT-001")).toBeVisible()
        await expect(within(warning).getByText("Server check")).toBeVisible()
        await userEvent.click(warning)
        expect(args.onSelect).toHaveBeenCalledWith(
            expect.objectContaining({path: "chart.charts.bars"})
        )
    },
}

export const NoProblems: Story = {
    args: {diagnostics: []},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No problems found")).toBeVisible()
    },
}

export const ServerChecksOnly: Story = {
    args: {diagnostics: [], localUnavailable: true},
    play: async ({canvasElement}) => {
        await expect(
            within(canvasElement).getByText(/Checks in the browser are unavailable/)
        ).toBeVisible()
    },
}

export const SyntaxError: Story = {
    args: {
        diagnostics: editorDiagnostics("id: [a\n", parseYamlText("id: [a\n"), {
            local: [],
            server: [],
        }),
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getAllByText("YAML syntax")[0]).toBeVisible()
    },
}

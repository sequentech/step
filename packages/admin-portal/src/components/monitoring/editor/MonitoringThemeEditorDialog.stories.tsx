// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {IMonitoringEditorApi} from "./api"
import {MonitoringThemeEditorDialog} from "./MonitoringThemeEditorDialog"
import {
    EMonitoringProblemSeverity,
    EMonitoringSaveStatus,
    type TMonitoringSaveOutcome,
} from "./types"
import {THEME_YAML, eventEditorApi} from "./storyFixtures"

let boundary: ReturnType<typeof graphqlBoundary>
let api: IMonitoringEditorApi
type TApiOverrides = () => Partial<IMonitoringEditorApi>

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringThemeEditorDialog",
    component: MonitoringThemeEditorDialog,
    args: {
        open: true,
        api: eventEditorApi(),
        themeKey: "default",
        widgetCount: 3,
        preview: {dashboardId: "req-0260", widgetId: "turnout-by-group"},
        onClose: fn(),
        onSaved: fn(),
    },
    beforeEach: ({parameters}) => {
        boundary = graphqlBoundary({})
        api = eventEditorApi((parameters.api as TApiOverrides | undefined)?.() ?? {})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MonitoringThemeEditorDialog {...args} api={api} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MonitoringThemeEditorDialog>
export default meta
type Story = StoryObj<typeof meta>

const dialog = async (canvasElement: HTMLElement) => {
    const found = await within(canvasElement.ownerDocument.body).findByRole("dialog", {
        name: /Dashboard theme/,
    })
    await waitFor(() => expect(found).toBeVisible())
    return within(found)
}

export const PreviewAndApply: Story = {
    play: async ({canvasElement, args}) => {
        const view = await dialog(canvasElement)
        await expect(view.getByText(/applies to 3 widgets/)).toBeVisible()
        await waitFor(() =>
            expect(api.renderWidget).toHaveBeenCalledWith(
                expect.objectContaining({
                    widget_id: "turnout-by-group",
                    draft: {theme_yaml: THEME_YAML},
                })
            )
        )
        const yaml = await view.findByRole("textbox", {name: "YAML"})
        await userEvent.click(yaml)
        await userEvent.keyboard("{Control>}{End}{/Control}# darker{Enter}")
        await userEvent.click(view.getByRole("button", {name: "Apply theme"}))
        await expect(await view.findByText("Theme saved as revision 8")).toBeVisible()
        expect(api.saveConfig).toHaveBeenCalledWith(
            expect.objectContaining({
                kind: "theme",
                key: "default",
                expected_revision: 7,
                yaml: expect.stringContaining("# darker"),
            })
        )
        expect(args.onSaved).toHaveBeenCalledWith(8)
    },
}

export const OneWidget: Story = {
    args: {widgetCount: 1, preview: undefined},
    play: async ({canvasElement}) => {
        const view = await dialog(canvasElement)
        await expect(view.getByText(/applies to 1 widget$/)).toBeVisible()
        await expect(await view.findByRole("status", {name: "Checks"})).toBeVisible()
        expect(api.renderWidget).not.toHaveBeenCalled()
    },
}

export const Refused: Story = {
    parameters: {
        api: (() => ({
            saveConfig: fn(
                async (): Promise<TMonitoringSaveOutcome> => ({
                    status: EMonitoringSaveStatus.INVALID,
                    problems: [
                        {
                            severity: EMonitoringProblemSeverity.ERROR,
                            code: "chart_schema",
                            path: "style.palette",
                            message: "A palette lists colours.",
                        },
                    ],
                })
            ),
        })) satisfies TApiOverrides,
    },
    play: async ({canvasElement}) => {
        const view = await dialog(canvasElement)
        const yaml = await view.findByRole("textbox", {name: "YAML"})
        await userEvent.click(yaml)
        await userEvent.keyboard("{Control>}{End}{/Control}# x{Enter}")
        await userEvent.click(view.getByRole("button", {name: "Apply theme"}))
        await expect(await view.findByText("Not saved: fix the problems listed.")).toBeVisible()
        await expect(view.getByText(/A palette lists colours/)).toBeVisible()
    },
}

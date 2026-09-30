// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {IMonitoringEditorApi} from "./api"
import {MonitoringConfigureWidgetDialog} from "./MonitoringConfigureWidgetDialog"
import {sequentCoreValidator} from "./sequentCoreValidator"
import type {TLocalValidate} from "./yamlDraft"
import {
    EMonitoringConfigKind,
    EMonitoringSaveStatus,
    EMonitoringValidationResult,
    type TMonitoringSaveOutcome,
} from "./types"
import {FORBIDDEN_KEY, RENDERED, SOURCES, WIDGET_YAML, fakeEditorApi} from "./storyFixtures"

let boundary: ReturnType<typeof graphqlBoundary>
/** A fresh fake per story, so recorded calls and queued replies never leak between stories. */
let api: IMonitoringEditorApi
/** A story's replies, given as `parameters.api`: built per run, after Storybook resets its spies. */
type TApiOverrides = () => Partial<IMonitoringEditorApi>
const withApi = (overrides: TApiOverrides) => ({api: overrides})
const callCount = (mock: unknown) => (mock as {mock: {calls: unknown[]}}).mock.calls.length

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringConfigureWidgetDialog",
    component: MonitoringConfigureWidgetDialog,
    args: {
        open: true,
        widgetId: "turnout-by-group",
        dashboardId: "turnout",
        scopeLabel: "All authorized Posts",
        sources: SOURCES,
        api: fakeEditorApi(),
        localValidate: ((text: string) =>
            text.includes("sql:") ? [FORBIDDEN_KEY] : []) as TLocalValidate,
        onClose: fn(),
        onSaved: fn(),
    },
    beforeEach: ({parameters}) => {
        boundary = graphqlBoundary({})
        const overrides = parameters.api as TApiOverrides | undefined
        api = fakeEditorApi(overrides?.() ?? {})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MonitoringConfigureWidgetDialog {...args} api={api} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MonitoringConfigureWidgetDialog>
export default meta
type Story = StoryObj<typeof meta>

const body = (canvasElement: HTMLElement) => within(canvasElement.ownerDocument.body)

const openDialog = async (canvasElement: HTMLElement) => {
    const dialog = await body(canvasElement).findByRole("dialog", {name: /Configure widget/})
    await waitFor(() => expect(dialog).toBeVisible())
    const view = within(dialog)
    await view.findByRole("tab", {name: "Data & query"})
    return view
}

const retitle = async (view: ReturnType<typeof within>, title: string) => {
    const field = view.getByRole("textbox", {name: "Title"})
    await userEvent.clear(field)
    await userEvent.type(field, title)
}

export const EditAndSave: Story = {
    play: async ({canvasElement, args}) => {
        const view = await openDialog(canvasElement)
        const footer = view.getByRole("status", {name: "Preview"})
        await waitFor(() => expect(footer).toHaveTextContent("rendered in 42 ms"))
        await expect(footer).toHaveTextContent("Preview · All authorized Posts")
        await expect(footer).toHaveTextContent("Revision 7")
        await expect(footer).toHaveTextContent(/saved .+ by Ana Reyes/)
        await expect(view.getByRole("button", {name: "Save widget"})).toBeDisabled()

        await retitle(view, "Turnout by voter group")
        await expect(footer).toHaveTextContent("unsaved changes")
        await waitFor(() =>
            expect(api.renderWidget).toHaveBeenLastCalledWith(
                expect.objectContaining({
                    dashboard_id: "turnout",
                    widget_id: "turnout-by-group",
                    draft: {widget_yaml: expect.stringContaining("title: Turnout by voter group")},
                })
            )
        )
        // Only the pause after typing is rendered, not every keystroke.
        expect(callCount(api.renderWidget)).toBeLessThan(4)

        await userEvent.click(view.getByRole("tab", {name: "YAML"}))
        await expect(view.getByRole("textbox", {name: "YAML"})).toHaveTextContent(
            "# One turnout ratio"
        )

        await userEvent.click(view.getByRole("button", {name: "Save widget"}))
        await expect(await view.findByText("Widget saved as revision 8")).toBeVisible()
        expect(api.saveConfig).toHaveBeenCalledWith(
            expect.objectContaining({
                kind: "widget",
                key: "turnout-by-group",
                expected_revision: 7,
                yaml: expect.stringContaining("title: Turnout by voter group"),
            })
        )
        expect(args.onSaved).toHaveBeenCalledWith(8)
        await expect(footer).toHaveTextContent("Revision 8")
    },
}

/** Preview: the draft on its own at the dialog's full width and the widget's height. */
export const PreviewTab: Story = {
    play: async ({canvasElement}) => {
        const view = await openDialog(canvasElement)
        await waitFor(() => expect(api.renderWidget).toHaveBeenCalled())
        const beside = (api.renderWidget as unknown as {mock: {calls: [{width: number}][]}}).mock
            .calls
        const besideWidth = beside.at(-1)![0].width
        await userEvent.click(view.getByRole("tab", {name: "Preview"}))
        const panel = view.getByRole("tabpanel", {name: "Preview"})
        // The preview beside the forms gives way to the large one.
        expect(view.getAllByRole("region", {name: "Turnout by group"})).toHaveLength(1)
        expect(within(panel).getByRole("region", {name: "Turnout by group"})).toBeVisible()
        await waitFor(() => expect(beside.at(-1)![0].width).toBeGreaterThan(besideWidth))
        const frame = await within(panel).findByTitle("Turnout by group")
        await waitFor(() => expect(frame).toHaveStyle({height: "240px"}))
        await userEvent.click(view.getByRole("tab", {name: "YAML"}))
        await waitFor(() => expect(beside.at(-1)![0].width).toBe(besideWidth))
    },
}

export const ConflictKeepEditing: Story = {
    parameters: withApi(() => ({
        // Opened at revision 7; revision 9 is the one saved meanwhile.
        getConfig: fn(async ({kind, key}) => ({kind, key, yaml: WIDGET_YAML, revision: 7}))
            .mockResolvedValueOnce({
                kind: EMonitoringConfigKind.WIDGET,
                key: "turnout-by-group",
                yaml: WIDGET_YAML,
                revision: 7,
            })
            .mockResolvedValue({
                kind: EMonitoringConfigKind.WIDGET,
                key: "turnout-by-group",
                yaml: WIDGET_YAML.replace("height: 240", "height: 300"),
                revision: 9,
            }),
        saveConfig: fn()
            .mockResolvedValueOnce({
                status: EMonitoringSaveStatus.CONFLICT,
                current_revision: 9,
                author: {id: "u-luis", name: "Luis Santos"},
                time: "2026-09-29T09:15:00Z",
            } satisfies TMonitoringSaveOutcome)
            .mockResolvedValue({
                status: EMonitoringSaveStatus.SAVED,
                revision: 10,
                generation: 4,
                warnings: [],
            } satisfies TMonitoringSaveOutcome),
    })),
    play: async ({canvasElement, args}) => {
        const view = await openDialog(canvasElement)
        await retitle(view, "Mine")
        await userEvent.click(view.getByRole("button", {name: "Save widget"}))
        const conflict = within(
            await body(canvasElement).findByRole("dialog", {name: "Someone saved first"})
        )
        await waitFor(() =>
            expect(conflict.getByText(/Luis Santos saved revision 9/)).toBeVisible()
        )
        await userEvent.click(conflict.getByRole("button", {name: "Keep editing"}))
        await waitFor(() =>
            expect(
                body(canvasElement).queryByRole("dialog", {name: "Someone saved first"})
            ).toBeNull()
        )
        await userEvent.click(view.getByRole("button", {name: "Save widget"}))
        await expect(await view.findByText("Widget saved as revision 10")).toBeVisible()
        expect(api.saveConfig).toHaveBeenLastCalledWith(
            expect.objectContaining({
                expected_revision: 9,
                yaml: expect.stringContaining("title: Mine"),
            })
        )
    },
}

export const ConflictReload: Story = {
    parameters: withApi(() => ({
        saveConfig: fn(
            async (): Promise<TMonitoringSaveOutcome> => ({
                status: EMonitoringSaveStatus.CONFLICT,
                current_revision: 9,
            })
        ),
    })),
    play: async ({canvasElement}) => {
        const view = await openDialog(canvasElement)
        await retitle(view, "Mine")
        await userEvent.click(view.getByRole("button", {name: "Save widget"}))
        const conflict = within(
            await body(canvasElement).findByRole("dialog", {name: "Someone saved first"})
        )
        await waitFor(() => expect(conflict.getByRole("button", {name: "Reload"})).toBeEnabled())
        await userEvent.click(conflict.getByRole("button", {name: "Reload"}))
        await waitFor(() =>
            expect(view.getByRole("textbox", {name: "Title"})).toHaveValue("Turnout by group")
        )
        await expect(view.getByRole("button", {name: "Save widget"})).toBeDisabled()
    },
}

export const RefusedAsInvalid: Story = {
    parameters: withApi(() => ({
        saveConfig: fn(
            async (): Promise<TMonitoringSaveOutcome> => ({
                status: EMonitoringSaveStatus.INVALID,
                problems: [{...FORBIDDEN_KEY, path: "widgets.turnout-by-group.query.limit"}],
            })
        ),
    })),
    play: async ({canvasElement}) => {
        const view = await openDialog(canvasElement)
        await retitle(view, "Mine")
        await userEvent.click(view.getByRole("button", {name: "Save widget"}))
        await expect(
            await view.findByText("The widget was not saved: fix the problems listed.")
        ).toBeVisible()
        const checks = view.getByRole("region", {name: /Checks/})
        await expect(within(checks).getByText(/query\.limit/)).toBeVisible()
    },
}

export const ValidateInvalid: Story = {
    parameters: withApi(() => ({
        validateConfig: fn(async () => ({
            result: EMonitoringValidationResult.INVALID,
            problems: [FORBIDDEN_KEY],
            preview: RENDERED,
        })),
    })),
    play: async ({canvasElement, args}) => {
        const view = await openDialog(canvasElement)
        await userEvent.click(view.getByRole("button", {name: "Validate"}))
        await expect(
            await view.findByText("The widget has problems: see the checks.")
        ).toBeVisible()
        expect(api.validateConfig).toHaveBeenCalledWith(
            expect.objectContaining({kind: "widget", key: "turnout-by-group", yaml: WIDGET_YAML})
        )
        await expect(view.getByRole("status", {name: "Preview"})).toHaveTextContent("1 error")
        await expect(view.getByRole("button", {name: "Save widget"})).toBeDisabled()
    },
}

export const DiscardChanges: Story = {
    play: async ({canvasElement, args}) => {
        const view = await openDialog(canvasElement)
        await retitle(view, "Mine")
        await userEvent.click(view.getByRole("button", {name: "Cancel"}))
        const confirm = within(
            await body(canvasElement).findByRole("dialog", {name: "Discard changes?"})
        )
        await userEvent.click(confirm.getByRole("button", {name: "Keep editing"}))
        expect(args.onClose).not.toHaveBeenCalled()
        await userEvent.click(await view.findByRole("button", {name: "Cancel"}))
        await userEvent.click(
            within(
                await body(canvasElement).findByRole("dialog", {name: "Discard changes?"})
            ).getByRole("button", {name: "Discard"})
        )
        expect(args.onClose).toHaveBeenCalledTimes(1)
    },
}

export const LoadFailed: Story = {
    parameters: withApi(() => ({
        getConfig: fn(async () => {
            throw new Error("widget not found")
        }),
    })),
    play: async ({canvasElement}) => {
        const found = await body(canvasElement).findByRole("dialog", {name: /Configure widget/})
        await waitFor(() => expect(found).toBeVisible())
        const dialog = within(found)
        await expect(
            await dialog.findByText("The widget could not be loaded: widget not found")
        ).toBeVisible()
        await expect(dialog.getByRole("button", {name: "Save widget"})).toBeDisabled()
    },
}

/** sequent-core's own policy, run in the browser from the portal's WebAssembly build. */
export const BrowserChecks: Story = {
    args: {localValidate: sequentCoreValidator(EMonitoringConfigKind.WIDGET)},
    // The app loads the WebAssembly at start-up; a story has to ask for it.
    beforeEach: async () => {
        await initCore()
    },
    parameters: withApi(() => ({
        getConfig: fn(async ({kind, key}) => ({
            kind,
            key,
            yaml: WIDGET_YAML.replace(
                "  template: by_group\n",
                "  template: by_group\n  sql: select 1\n"
            ),
            revision: 7,
        })),
    })),
    play: async ({canvasElement}) => {
        const view = await openDialog(canvasElement)
        const checks = view.getByRole("region", {name: /Checks/})
        await expect(await within(checks).findByText("Browser check")).toBeVisible()
        await expect(within(checks).getByText(/query\.sql/)).toBeVisible()
        await expect(view.getByRole("button", {name: "Save widget"})).toBeDisabled()
        expect(within(checks).queryByText(/Checks in the browser are unavailable/)).toBeNull()
    },
}

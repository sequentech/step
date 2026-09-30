// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {IMonitoringEditorApi} from "./api"
import {MonitoringDashboardEditor} from "./MonitoringDashboardEditor"
import {EMonitoringSaveStatus, type TMonitoringSaveOutcome} from "./types"
import {eventEditorApi} from "./storyFixtures"

let boundary: ReturnType<typeof graphqlBoundary>
let api: IMonitoringEditorApi
type TApiOverrides = () => Partial<IMonitoringEditorApi>

const meta = {
    title: "Admin/Monitoring/Editor/MonitoringDashboardEditor",
    component: MonitoringDashboardEditor,
    args: {
        open: true,
        api: eventEditorApi(),
        dashboardId: "req-0260",
        onConfigureWidget: fn(),
        onClose: fn(),
        onSaved: fn(),
        onReset: fn(),
    },
    beforeEach: ({parameters}) => {
        boundary = graphqlBoundary({})
        api = eventEditorApi((parameters.api as TApiOverrides | undefined)?.() ?? {})
    },
    render: (args) => (
        <AdminStoryProvider boundary={boundary}>
            <MonitoringDashboardEditor {...args} api={api} />
        </AdminStoryProvider>
    ),
} satisfies Meta<typeof MonitoringDashboardEditor>
export default meta
type Story = StoryObj<typeof meta>

const body = (canvasElement: HTMLElement) => within(canvasElement.ownerDocument.body)

const openEditor = async (canvasElement: HTMLElement) => {
    const found = await body(canvasElement).findByRole("dialog", {name: /Editing dashboard/})
    await waitFor(() => expect(found).toBeVisible())
    const view = within(found)
    // The widget titles come from the catalog, which loads after the dashboard.
    await view.findByRole("listitem", {name: "Voting activity"})
    return view
}

const order = (view: ReturnType<typeof within>) =>
    view.getAllByRole("listitem").map((item: HTMLElement) => item.getAttribute("aria-label"))

const choose = async (canvasElement: HTMLElement, combobox: HTMLElement, option: string) => {
    await userEvent.click(combobox)
    await userEvent.click(await body(canvasElement).findByRole("option", {name: option}))
}

const savedYaml = () =>
    ((api.saveConfig as unknown as {mock: {calls: [{yaml: string}][]}}).mock.calls.at(-1)?.[0]
        .yaml ?? "") as string

export const EditAndSave: Story = {
    play: async ({canvasElement, args}) => {
        const view = await openEditor(canvasElement)
        expect(order(view)).toEqual(["Voter turnout", "Turnout by group", "Voting activity"])

        const title = view.getByRole("textbox", {name: "Title"})
        await userEvent.clear(title)
        await userEvent.type(title, "Turnout")
        await userEvent.click(view.getByRole("checkbox", {name: "Country"}))

        await userEvent.click(view.getByRole("button", {name: "Move down: Voter turnout"}))
        expect(order(view)).toEqual(["Turnout by group", "Voter turnout", "Voting activity"])

        const activity = view.getByRole("listitem", {name: "Voting activity"})
        await choose(
            canvasElement,
            within(activity).getByRole("combobox", {name: "Width"}),
            "12 of 12"
        )

        await userEvent.click(view.getByRole("button", {name: "Add widget"}))
        const catalog = within(await body(canvasElement).findByRole("dialog", {name: "Add widget"}))
        await userEvent.type(catalog.getByRole("searchbox"), "SW-F-0301")
        await userEvent.click(catalog.getByRole("button", {name: "Add Attack detections"}))
        await waitFor(() => expect(order(view)).toHaveLength(4))

        await userEvent.click(view.getByRole("button", {name: "Save dashboard"}))
        const saved = await view.findByText("Dashboard saved as revision 8")
        await expect(saved).toBeVisible()
        // Beside Save, not below the widget list, where a long dashboard scrolls it away.
        expect(saved.closest(".MuiDialogContent-root")).toBeNull()
        const yaml = savedYaml()
        expect(yaml).toContain("# The SW-F-0260 dashboard.")
        expect(yaml).toContain("title: Turnout\n")
        expect(yaml).toMatch(/selectors:\s+- region\s+- post\n/)
        expect(yaml).toContain(
            "- {widget: turnout-by-group, width: 6, values: {measure: voted_pre}}"
        )
        expect(yaml).toContain("- {widget: voting-activity, width: 12}")
        expect(yaml).toContain("- {widget: attack-log, width: 6}")
        expect(api.saveConfig).toHaveBeenCalledWith(
            expect.objectContaining({kind: "dashboard", key: "req-0260", expected_revision: 7})
        )
        expect(args.onSaved).toHaveBeenCalledWith(8)
    },
}

export const DuplicateAndRemove: Story = {
    play: async ({canvasElement, args}) => {
        const view = await openEditor(canvasElement)
        await userEvent.click(view.getByRole("button", {name: "Actions for Turnout by group"}))
        await userEvent.click(await body(canvasElement).findByRole("menuitem", {name: "Duplicate"}))
        await expect(await view.findByText("Duplicated as turnout-by-group-copy")).toBeVisible()
        expect(api.saveConfig).toHaveBeenCalledWith(
            expect.objectContaining({
                kind: "widget",
                key: "turnout-by-group-copy",
                yaml: expect.stringContaining("id: turnout-by-group-copy"),
            })
        )
        await waitFor(() => expect(order(view)).toHaveLength(4))
        // The copy goes in after the original, titled apart from it.
        expect(order(view)[1]).toBe("Turnout by group")
        expect(order(view)[2]).toBe("Turnout by group (copy)")

        await userEvent.click(view.getAllByRole("button", {name: "Actions for Voter turnout"})[0])
        await userEvent.click(await body(canvasElement).findByRole("menuitem", {name: "Remove"}))
        await waitFor(() => expect(order(view)).toHaveLength(3))

        await userEvent.click(view.getAllByRole("button", {name: "Actions for Voting activity"})[0])
        await userEvent.click(
            await body(canvasElement).findByRole("menuitem", {name: "Configure widget"})
        )
        expect(args.onConfigureWidget).toHaveBeenCalledWith("voting-activity")
    },
}

export const DragToReorder: Story = {
    play: async ({canvasElement}) => {
        const view = await openEditor(canvasElement)
        const [first, , last] = view.getAllByRole("listitem")
        const transfer = new DataTransfer()
        first.dispatchEvent(new DragEvent("dragstart", {bubbles: true, dataTransfer: transfer}))
        last.dispatchEvent(
            new DragEvent("dragover", {bubbles: true, cancelable: true, dataTransfer: transfer})
        )
        last.dispatchEvent(
            new DragEvent("drop", {bubbles: true, cancelable: true, dataTransfer: transfer})
        )
        await waitFor(() =>
            expect(order(view)).toEqual(["Turnout by group", "Voting activity", "Voter turnout"])
        )
    },
}

export const EditTheme: Story = {
    play: async ({canvasElement}) => {
        const view = await openEditor(canvasElement)
        await userEvent.click(view.getByRole("button", {name: "Edit theme"}))
        const theme = within(
            await body(canvasElement).findByRole("dialog", {name: /Dashboard theme · default/})
        )
        // Every dashboard of the event that uses the theme: this one's three widgets.
        await expect(await theme.findByText(/applies to 3 widgets/)).toBeInTheDocument()
        // Previewed on the first widget that draws the theme's colours, not on its KPIs.
        await waitFor(() =>
            expect(api.renderWidget).toHaveBeenCalledWith(
                expect.objectContaining({
                    widget_id: "turnout-by-group",
                    draft: expect.objectContaining({
                        theme_yaml: expect.stringContaining("id: default"),
                    }),
                })
            )
        )
    },
}

export const ResetToPreset: Story = {
    play: async ({canvasElement, args}) => {
        const view = await openEditor(canvasElement)
        await userEvent.click(view.getByRole("button", {name: "Reset to preset"}))
        const reset = within(
            await body(canvasElement).findByRole("dialog", {name: "Reset to preset"})
        )
        await userEvent.click(await reset.findByRole("button", {name: "Reset"}))
        await expect(await reset.findByText("The event now uses COMELEC.")).toBeInTheDocument()
        expect(args.onReset).toHaveBeenCalled()
        // The draft is of the configuration the reset replaced: the editor closes with the dialog.
        await userEvent.click(reset.getByRole("button", {name: "Close"}))
        await waitFor(() => expect(args.onClose).toHaveBeenCalled())
    },
}

export const Conflict: Story = {
    parameters: {
        api: (() => ({
            saveConfig: fn(
                async (): Promise<TMonitoringSaveOutcome> => ({
                    status: EMonitoringSaveStatus.CONFLICT,
                    current_revision: 9,
                    author: {id: "u-luis", name: "Luis Santos"},
                    time: "2026-09-29T09:15:00Z",
                })
            ),
        })) satisfies TApiOverrides,
    },
    play: async ({canvasElement}) => {
        const view = await openEditor(canvasElement)
        await userEvent.click(view.getByRole("button", {name: "Move up: Voting activity"}))
        await userEvent.click(view.getByRole("button", {name: "Save dashboard"}))
        const conflict = within(
            await body(canvasElement).findByRole("dialog", {name: "Someone saved first"})
        )
        await waitFor(() =>
            expect(conflict.getByText(/Luis Santos saved revision 9/)).toBeVisible()
        )
        await conflict.findByRole("region", {name: "My changes"})
    },
}

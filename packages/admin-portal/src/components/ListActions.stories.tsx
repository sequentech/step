// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, spyOn, userEvent, waitFor, within} from "storybook/test"
import {Button} from "@mui/material"
import {DatagridConfigurable, List, TextField, TextInput} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {areaRecords} from "@/__stories__/fixtures"
import {ListActions} from "./ListActions"

interface Scenario {
    withColumns: boolean
    withFilter: boolean
    withImport: boolean
    withExport: boolean
    /** A plain export callback, an export menu opener, or react-admin's CSV export. */
    exportMode: "callback" | "menu" | "csv"
    isExportDisabled: boolean
    withAction: boolean
    /** Adds the "Add" button that opens a drawer with the given component. */
    withComponent: boolean
    withExtraAction: boolean
    onImport: () => void
    onExport: () => void
    onOpenExportMenu: (event: React.MouseEvent<HTMLElement>) => void
    onAction: () => void
    onExtraAction: () => void
}

let boundary: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: {name: string; href: string}[]
let blobs: Blob[]

function Fixture(args: Scenario) {
    const [open, setOpen] = useState(false)
    return (
        <AdminStoryProvider boundary={boundary} dataProvider={data.provider}>
            <List
                resource="sequent_backend_area"
                filters={[<TextInput key="name" source="name@_ilike" label="Name" />]}
                disableSyncWithLocation
                actions={
                    <ListActions
                        withColumns={args.withColumns}
                        withFilter={args.withFilter}
                        withImport={args.withImport}
                        doImport={args.onImport}
                        withExport={args.withExport}
                        doExport={args.onExport}
                        openExportMenu={
                            args.exportMode === "menu" ? args.onOpenExportMenu : undefined
                        }
                        defaultExport={args.exportMode === "csv"}
                        isExportDisabled={args.isExportDisabled}
                        withAction={args.withAction}
                        doAction={args.onAction}
                        actionLabel="electionEventScreen.tally.create.createTallyButton"
                        withComponent={args.withComponent}
                        Component={<p>Synthetic area form</p>}
                        open={open}
                        setOpen={setOpen}
                        extraActions={
                            args.withExtraAction
                                ? [
                                      <Button key="extra" onClick={args.onExtraAction}>
                                          Synthetic extra action
                                      </Button>,
                                  ]
                                : []
                        }
                    />
                }
            >
                <DatagridConfigurable bulkActionButtons={false} rowClick={false}>
                    <TextField source="name" />
                    <TextField source="description" />
                </DatagridConfigurable>
            </List>
        </AdminStoryProvider>
    )
}

const callbacks = ["onImport", "onExport", "onOpenExportMenu", "onAction", "onExtraAction"]

const meta = {
    title: "Admin/Components/ListActions",
    component: ListActions,
    args: {
        withColumns: true,
        withFilter: true,
        withImport: true,
        withExport: true,
        exportMode: "callback",
        isExportDisabled: false,
        withAction: false,
        withComponent: false,
        withExtraAction: false,
        onImport: fn(),
        onExport: fn(),
        onOpenExportMenu: fn(),
        onAction: fn(),
        onExtraAction: fn(),
    },
    argTypes: {
        exportMode: {control: "inline-radio", options: ["callback", "menu", "csv"]},
        ...Object.fromEntries(callbacks.map((name) => [name, {table: {disable: true}}])),
    },
    beforeEach: () => {
        boundary = graphqlBoundary({})
        data = resourceBoundary({sequent_backend_area: areaRecords()})
        downloads = []
        blobs = []
        // react-admin's CSV export saves through a temporary link; keep its target local.
        const objectUrl = spyOn(URL, "createObjectURL").mockImplementation((value) => {
            if (!(value instanceof Blob)) throw new Error("Expected an exported file")
            blobs.push(value)
            return "blob:synthetic-csv-export"
        })
        const anchor = spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(function (
            this: HTMLAnchorElement
        ) {
            downloads.push({name: this.download, href: this.href})
        })
        return () => {
            objectUrl.mockRestore()
            anchor.mockRestore()
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const toolbarButton = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).queryByRole("button", {name})

async function loaded(canvasElement: HTMLElement) {
    await within(canvasElement).findByText("North district")
    return within(canvasElement)
}

export const Populated: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        for (const name of ["Columns", "Add filter", "Import", "Export"]) {
            await expect(canvas.getByRole("button", {name})).toBeEnabled()
        }
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.import")}))
        expect(args.onImport).toHaveBeenCalledTimes(1)
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.export")}))
        expect(args.onExport).toHaveBeenCalledTimes(1)
        expect(args.onOpenExportMenu).not.toHaveBeenCalled()
        expect(downloads).toEqual([])
    },
}

export const ExportDisabled: Story = {
    args: {isExportDisabled: true},
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        await expect(canvas.getByRole("button", {name: "Export"})).toBeDisabled()
        expect(args.onExport).not.toHaveBeenCalled()
    },
}

export const ExportMenuOpener: Story = {
    args: {exportMode: "menu"},
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Export"}))
        expect(args.onOpenExportMenu).toHaveBeenCalledTimes(1)
        expect(args.onExport).not.toHaveBeenCalled()
    },
}

export const CsvExportDownloadsTheList: Story = {
    args: {exportMode: "csv"},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        const reads = data.calls.length
        await userEvent.click(canvas.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "sequent_backend_area.csv", href: "blob:synthetic-csv-export"},
            ])
        )
        expect(data.calls.slice(reads)).toEqual([
            {
                method: "getList",
                args: [
                    "sequent_backend_area",
                    expect.objectContaining({pagination: {page: 1, perPage: 1000}}),
                ],
            },
        ])
        const csv = await blobs[0].text()
        expect(csv.split("\n")[0]).toMatch(/^id,/)
        expect(csv).toContain("North district")
        expect(csv).toContain("South district")
    },
}

export const ActionButtonStartsTheAction: Story = {
    args: {withAction: true},
    play: async ({canvasElement, args}) => {
        const canvas = await loaded(canvasElement)
        const label = i18n.t("electionEventScreen.tally.create.createTallyButton")
        await userEvent.click(canvas.getByRole("button", {name: label}))
        expect(args.onAction).toHaveBeenCalledTimes(1)
    },
}

export const AddOpensTheComponentDrawer: Story = {
    args: {withComponent: true},
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        expect(within(document.body).queryByText("Synthetic area form")).not.toBeInTheDocument()
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.add")}))
        const form = await within(document.body).findByText("Synthetic area form")
        await waitFor(() => expect(form).toBeVisible())
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(within(document.body).queryByText("Synthetic area form")).not.toBeInTheDocument()
        )
    },
}

export const OnlyTheRequestedActions: Story = {
    args: {
        withColumns: false,
        withFilter: false,
        withImport: false,
        withExport: false,
        withExtraAction: true,
    },
    play: async ({canvasElement, args}) => {
        await loaded(canvasElement)
        for (const name of ["Columns", "Add filter", "Import", "Export"]) {
            expect(toolbarButton(canvasElement, name)).not.toBeInTheDocument()
        }
        await userEvent.click(toolbarButton(canvasElement, "Synthetic extra action")!)
        expect(args.onExtraAction).toHaveBeenCalledTimes(1)
    },
}

export const ColumnsButtonHidesAColumn: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await expect(canvas.getByRole("columnheader", {name: "Description"})).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Columns"}))
        await userEvent.click(
            await within(document.body).findByRole("switch", {name: "Description"})
        )
        await waitFor(() =>
            expect(canvas.queryByRole("columnheader", {name: "Description"})).toBeNull()
        )
        await userEvent.keyboard("{Escape}")
        await waitFor(() =>
            expect(within(document.body).queryByRole("switch")).not.toBeInTheDocument()
        )
    },
}

export const FilterButtonAddsAFilterInput: Story = {
    play: async ({canvasElement}) => {
        const canvas = await loaded(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
        await userEvent.click(
            await within(document.body).findByRole("menuitemcheckbox", {name: "Name"})
        )
        const input = await canvas.findByRole("textbox", {name: "Name"})
        await userEvent.type(input, "south")
        await waitFor(() =>
            expect(data.calls.at(-1)?.args[1]).toMatchObject({filter: {"name@_ilike": "south"}})
        )
        await waitFor(() => expect(canvas.queryByText("North district")).toBeNull())
        await expect(canvas.getByText("South district")).toBeVisible()
    },
}

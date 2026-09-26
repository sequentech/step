// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {useLocation} from "react-router-dom"
import type {Identifier} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {documentUrl, recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import {IPermissions} from "@/types/keycloak"
import {
    AREAS,
    CONTEST,
    SHEET_IDS,
    SHEET_IMPORT,
    TALLY_SHEETS,
} from "./__stories__/TallySheetFixture"
import {ListTallySheetVersions} from "./ListTallySheetVersions"
import {WizardSteps} from "./TallySheetWizard"

interface Scenario {
    roles: string[]
    /** Whether the ballot box has any version. */
    versions: boolean
    approveAction: (id: Identifier) => void
    disapproveAction: (id: Identifier) => void
    doAction: (action: number, id?: Identifier) => void
    setShowVersionsTable: (show: boolean) => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: RecordedDownload[]
let location = ""

function LocationProbe() {
    const {pathname, search} = useLocation()
    useEffect(() => {
        location = `${pathname}${search}`
    }, [pathname, search])
    return null
}

const ALL_ROLES = [
    IPermissions.TALLY_SHEET_VIEW,
    IPermissions.TALLY_SHEET_REVIEW,
    IPermissions.TALLY_SHEET_IMPORT_VIEW,
]

const actionDefects = {
    reason:
        "The version actions and the import links are icon buttons named only by a " +
        "tooltip on their hidden icon.",
    a11y: ["button-name"],
}

const listDefects = {
    expectedFailure: {
        reason:
            actionDefects.reason +
            " The source file tooltip labels the span around its button, which has no role.",
        a11y: ["aria-prohibited-attr", "button-name"],
    },
}

const meta = {
    title: "Admin/Tally sheet/ListTallySheetVersions",
    component: ListTallySheetVersions,
    args: {
        roles: ALL_ROLES,
        versions: true,
        approveAction: fn(),
        disapproveAction: fn(),
        doAction: fn(),
        setShowVersionsTable: fn(),
    },
    argTypes: {
        approveAction: {table: {disable: true}},
        disapproveAction: {table: {disable: true}},
        doAction: {table: {disable: true}},
        setShowVersionsTable: {table: {disable: true}},
    },
    parameters: {
        widgets: ["ImportedVersionSourceContextProvider", "ImportedVersionSource"],
        ...listDefects,
    },
    beforeEach: async ({args}) => {
        location = ""
        data = resourceBoundary({
            sequent_backend_tally_sheet: args.versions ? TALLY_SHEETS : [],
            sequent_backend_tally_sheet_import: [SHEET_IMPORT],
            sequent_backend_area: AREAS,
            sequent_backend_contest: [CONTEST],
        })
        graphql = graphqlBoundary(
            {
                FetchDocument: ({variables}) => ({
                    data: {fetchDocument: {url: documentUrl(String(variables.documentId))}},
                }),
            },
            {schema: true}
        )
        await graphql.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({roles, versions: _versions, ...actions}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <LocationProbe />
            <ListTallySheetVersions tallySheet={TALLY_SHEETS[0]} reload={null} {...actions} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const versionRow = (canvasElement: HTMLElement, version: number) =>
    within(canvasElement).findByRole("row", {name: new RegExp(`^${version} `)})

/** The row action whose tooltip has this title, on its icon or on a span around it. */
function action(row: HTMLElement, title: string) {
    const labelled = within(row).getByLabelText(title)
    return (labelled.closest("button") ?? labelled.querySelector("button")) as HTMLButtonElement
}

const hasAction = (row: HTMLElement, title: string) => within(row).queryByLabelText(title) !== null

export const Populated: Story = {
    parameters: {widgets: ["ImportedVersionSourceContextProvider", "ImportedVersionSource"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Versions for ballot box")).toBeVisible()
        await expect(canvas.getByText("PAPER")).toBeVisible()
        await expect(await canvas.findByText("North district")).toBeVisible()
        await expect(await canvas.findByText("Members")).toBeVisible()
        await versionRow(canvasElement, 3)
        // Newest first, and only the paper box of the north district.
        const versions = canvas
            .getAllByRole("row")
            .slice(1)
            .map((row) => within(row).getAllByRole("cell")[0].textContent)
        expect(versions).toEqual(["3", "2", "1"])
        const latest = within(await versionRow(canvasElement, 3))
        await expect(latest.getByText("PENDING")).toBeVisible()
        await expect(await latest.findByText(`Import status: ${SHEET_IMPORT.status}`)).toBeVisible()
        const first = within(await versionRow(canvasElement, 1))
        await expect(first.getByText("reviewer.north")).toBeVisible()
        expect(first.getAllByText("-").length).toBeGreaterThan(0)
        expect(data.writes).toEqual([])
        expect(graphql.calls).toEqual([])
    },
}

export const OnlyPendingVersionsCanBeReviewed: Story = {
    play: async ({canvasElement, args}) => {
        const latest = await versionRow(canvasElement, 3)
        await userEvent.click(action(latest, "Approve"))
        expect(args.approveAction).toHaveBeenCalledWith(SHEET_IDS.latestPaper)
        await userEvent.click(action(latest, "Disapprove"))
        expect(args.disapproveAction).toHaveBeenCalledWith(SHEET_IDS.latestPaper)
        const reviewed = await versionRow(canvasElement, 2)
        expect(hasAction(reviewed, "Approve")).toBe(false)
        expect(hasAction(reviewed, "Disapprove")).toBe(false)
    },
}

export const ShowAVersion: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(action(await versionRow(canvasElement, 2), "Show"))
        expect(args.doAction).toHaveBeenCalledWith(WizardSteps.View, SHEET_IDS.secondPaper)
    },
}

export const OpenTheSourceImport: Story = {
    play: async ({canvasElement}) => {
        const latest = await versionRow(canvasElement, 3)
        await within(latest).findByText(`Import status: ${SHEET_IMPORT.status}`)
        await userEvent.click(action(latest, "Open import"))
        await waitFor(() =>
            expect(location).toBe(
                `/sequent_backend_election_event/${EVENT_ID}` +
                    `?tabId=tally-sheet-imports&tallySheetImportId=${SHEET_IDS.import}`
            )
        )
    },
}

export const DownloadTheSourceFile: Story = {
    play: async ({canvasElement}) => {
        const latest = await versionRow(canvasElement, 3)
        await within(latest).findByText(`Import status: ${SHEET_IMPORT.status}`)
        const download = action(latest, "Source file")
        await waitFor(() => expect(download).toBeEnabled())
        await userEvent.click(download)
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "north-paper.xml", href: documentUrl(SHEET_IDS.sourceDocument)},
            ])
        )
        expect(graphql.calls.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "FetchDocument",
                variables: {electionEventId: EVENT_ID, documentId: SHEET_IDS.sourceDocument},
            },
        ])
    },
}

export const WithoutImportPermission: Story = {
    args: {roles: [IPermissions.TALLY_SHEET_VIEW, IPermissions.TALLY_SHEET_REVIEW]},
    parameters: {widgets: ["ImportedVersionSource"], expectedFailure: actionDefects},
    play: async ({canvasElement}) => {
        const latest = within(await versionRow(canvasElement, 3))
        expect(latest.queryByText(/Import status/)).toBeNull()
        expect(
            data.calls.map(({args}) => args[0]).includes("sequent_backend_tally_sheet_import")
        ).toBe(false)
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.TALLY_SHEET_VIEW]},
    parameters: {widgets: ["ImportedVersionSource"], expectedFailure: actionDefects},
    play: async ({canvasElement}) => {
        const latest = await versionRow(canvasElement, 3)
        await expect(action(latest, "Show")).toBeVisible()
        expect(hasAction(latest, "Approve")).toBe(false)
    },
}

export const BackToTheBallotBoxes: Story = {
    play: async ({canvasElement, args}) => {
        await versionRow(canvasElement, 3)
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Back"}))
        expect(args.setShowVersionsTable).toHaveBeenCalledWith(false)
    },
}

export const NoVersions: Story = {
    args: {versions: false},
    parameters: {widgets: [], expectedFailure: null},
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByText("No Tally Sheet Yet.")).toBeVisible()
    },
}

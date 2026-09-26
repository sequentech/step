// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {ETallyType} from "@/types/ceremonies"
import {ETemplateType} from "@/types/templates"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {
    RESULT_DOCUMENTS,
    TALLY_IDS,
    TallyStoryContext,
    tallyData,
} from "@/resources/Tally/__stories__/TallyFixture"
import {documentUrl, recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import {exportMenuHidden} from "./__stories__/DownloadFixture"
import {ExportElectionMenu, type IResultDocumentsData} from "./ExportElectionMenu"

interface Scenario {
    /** Which results the menu exports: the event's or the council election's. */
    level: "event" | "election"
    tallyType: ETallyType
    /** Whether the deployment enables the Miru transmission exports. */
    miru: boolean
    onCreateTransmissionPackage: (value: {area_id: string; election_id: string}) => void
}

const DOCUMENTS: Record<Scenario["level"], IResultDocumentsData[]> = {
    event: [{documents: RESULT_DOCUMENTS.event, name: "Council results", class_type: "event"}],
    election: [
        {documents: RESULT_DOCUMENTS.election, name: "Council election", class_type: "election"},
    ],
}

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]

function Fixture({level, tallyType, miru, onCreateTransmissionPackage}: Scenario) {
    return (
        <AdminStoryProvider boundary={boundary} settings={{ACTIVATE_MIRU_EXPORT: miru}}>
            <WidgetsContextProvider>
                <TallyStoryContext data={tallyData()}>
                    <ExportElectionMenu
                        itemName={level === "event" ? "Council" : "Council election"}
                        documentsList={DOCUMENTS[level]}
                        electionEventId={EVENT_ID}
                        tallySessionId={STORY_IDS.tallySession}
                        tallyType={tallyType}
                        electionId={level === "election" ? STORY_IDS.election : null}
                        onCreateTransmissionPackage={onCreateTransmissionPackage}
                        tenantId={TENANT_ID}
                        resultsEventId={TALLY_IDS.resultsEvent}
                    />
                </TallyStoryContext>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/ExportElectionMenu",
    component: ExportElectionMenu,
    args: {
        level: "event",
        tallyType: ETallyType.ELECTORAL_RESULTS,
        miru: false,
        onCreateTransmissionPackage: fn(),
    },
    argTypes: {
        level: {control: "inline-radio", options: ["event", "election"]},
        tallyType: {control: "inline-radio", options: Object.values(ETallyType)},
        onCreateTransmissionPackage: {table: {disable: true}},
    },
    beforeEach: async () => {
        boundary = graphqlBoundary(
            {
                FetchDocument: ({variables}) => ({
                    data: {fetchDocument: {url: documentUrl(String(variables.documentId))}},
                }),
            },
            {schema: true}
        )
        await boundary.ready
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const exportLabel = (item: string, format: string) =>
    i18n.t("common.label.exportFormat", {item, format})
const allAreasLabel = (format: string) =>
    i18n.t("tally.exportAllAreas", {item: "Council election", format})

async function openMenu(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByLabelText("export election data"))
    const menu = await within(document.body).findByRole("menu")
    await waitFor(() => expect(menu).toBeVisible())
    return within(menu)
}

async function closeMenu() {
    await userEvent.keyboard("{Escape}")
    await exportMenuHidden()
}

const itemNames = (menu: ReturnType<typeof within>) =>
    menu.getAllByRole("menuitem").map((item: HTMLElement) => item.textContent)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        // The HTML report is also offered as a PDF rendered from it.
        expect(itemNames(menu)).toEqual([
            exportLabel("Council results", "PDF"),
            exportLabel("Council results", "HTML"),
            exportLabel("Council results", "PDF"),
            exportLabel("Council results", "JSON"),
            exportLabel("Council results", "TAR_GZ"),
            exportLabel("Council", "XLSX"),
        ])
        await closeMenu()
        expect(boundary.calls).toEqual([])
    },
}

export const DownloadThroughItsAddress: Story = {
    parameters: {widgets: ["PerformDownload"]},
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        await userEvent.click(
            menu.getByRole("menuitem", {name: exportLabel("Council results", "TAR_GZ")})
        )
        await exportMenuHidden()
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "report.tar.gz", href: documentUrl(RESULT_DOCUMENTS.event.tar_gz)},
            ])
        )
        expect(boundary.calls).toEqual([
            {
                name: "FetchDocument",
                variables: {electionEventId: EVENT_ID, documentId: RESULT_DOCUMENTS.event.tar_gz},
                headers: {},
            },
        ])
    },
}

export const ElectionOffersAllAreasExports: Story = {
    args: {level: "election"},
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        expect(itemNames(menu)).toEqual([
            exportLabel("Council election", "PDF"),
            exportLabel("Council election", "HTML"),
            exportLabel("Council election", "PDF"),
            exportLabel("Council election", "JSON"),
            exportLabel("Council election", "TAR_GZ"),
            allAreasLabel("HTML"),
            allAreasLabel("PDF"),
            allAreasLabel("JSON"),
        ])
        await closeMenu()
    },
}

export const MiruAddsTransmissionAndBallotImages: Story = {
    args: {level: "election", miru: true},
    play: async ({canvasElement, args}) => {
        const menu = await openMenu(canvasElement)
        const areaItem = i18n.t("tally.exportElectionArea", {name: "North district"})
        const report = i18n.t("tally.generateReport", {
            name: i18n.t(`template.type.${ETemplateType.BALLOT_IMAGES}`),
        })
        expect(itemNames(menu).slice(-3)).toEqual([
            areaItem,
            i18n.t("tally.exportElectionArea", {name: "South district"}),
            report,
        ])
        await userEvent.click(menu.getByRole("menuitem", {name: areaItem}))
        expect(args.onCreateTransmissionPackage).toHaveBeenCalledWith({
            area_id: STORY_IDS.area,
            election_id: STORY_IDS.election,
        })
        await exportMenuHidden()
    },
}

export const InitializationReportOmitsResultExports: Story = {
    args: {level: "election", miru: true, tallyType: ETallyType.INITIALIZATION_REPORT},
    play: async ({canvasElement}) => {
        const menu = await openMenu(canvasElement)
        expect(itemNames(menu)).toEqual([
            exportLabel("Council election", "PDF"),
            exportLabel("Council election", "HTML"),
            exportLabel("Council election", "PDF"),
            exportLabel("Council election", "JSON"),
            exportLabel("Council election", "TAR_GZ"),
        ])
        await closeMenu()
    },
}

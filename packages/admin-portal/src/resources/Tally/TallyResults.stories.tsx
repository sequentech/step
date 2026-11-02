// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {i18n, initCore} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {STORY_IDS} from "@/__stories__/fixtures"
import {documentUrl, recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import {WidgetsContextProvider} from "@/providers/WidgetsContextProvider"
import {exportMenuHidden} from "@/components/tally/__stories__/DownloadFixture"
import {
    RESULT_DOCUMENTS,
    TallyStoryContext,
    tallyData,
    tallyExecution,
    tallySession,
} from "./__stories__/TallyFixture"
import {TallyResults} from "./TallyResults"
import {
    EStoryPermissions,
    EStoryWorkflow,
    useStoryGlobals,
} from "../../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether ResultsDataLoader has loaded the tally's results. */
    loaded: boolean
    /** Elections of the tally session. */
    electionIds: string[]
    loading: boolean
    onCreateTransmissionPackage: (value: {area_id: string; election_id: string}) => void
}

let boundary: ReturnType<typeof graphqlBoundary>
let downloads: RecordedDownload[]

function Fixture({loaded, electionIds, loading, onCreateTransmissionPackage}: Scenario) {
    const {permissions, workflow} = useStoryGlobals()
    return (
        <AdminStoryProvider boundary={boundary} role={permissions}>
            <WidgetsContextProvider>
                <TallyStoryContext data={loaded ? tallyData() : null}>
                    <TallyResults
                        tally={tallySession(workflow, {election_ids: electionIds})}
                        resultsEventId={tallyExecution(workflow).results_event_id ?? null}
                        loading={loading}
                        onCreateTransmissionPackage={onCreateTransmissionPackage}
                    />
                </TallyStoryContext>
            </WidgetsContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Tally/TallyResults",
    component: TallyResults,
    args: {
        loaded: true,
        electionIds: [STORY_IDS.election, STORY_IDS.secondElection],
        loading: false,
        onCreateTransmissionPackage: fn(),
    },
    argTypes: {onCreateTransmissionPackage: {table: {disable: true}}},
    globals: {workflow: EStoryWorkflow.RESULTS},
    beforeEach: async () => {
        // Contests are ordered by sequent-core.
        await initCore()
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
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const tabNames = (canvasElement: HTMLElement, rowClass: string) =>
    within(canvasElement.querySelector(`.seq-results-selector__${rowClass}-row`) as HTMLElement)
        .getAllByRole("tab")
        .map((tab) => tab.textContent)

const candidateVotes = (canvasElement: HTMLElement, alias: RegExp) =>
    within(within(canvasElement).getByRole("row", {name: alias}))
        .getAllByRole("gridcell")
        .map((cell) => cell.textContent)
        .slice(0, 2)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        expect(tabNames(canvasElement, "election")).toEqual(["Council", "Deputy"])
        expect(tabNames(canvasElement, "contest")).toEqual(["Members"])
        expect(tabNames(canvasElement, "area")).toHaveLength(3)
        expect(tabNames(canvasElement, "area")[0]).toBe(i18n.t("tally.common.global"))
        // The global results of the first election's first contest are shown.
        expect(candidateVotes(canvasElement, /^Alice/)).toEqual(["Alice", "50"])
        expect(boundary.calls).toEqual([])
    },
}

export const ChoosingAnAreaShowsItsResults: Story = {
    play: async ({canvasElement}) => {
        const areaTabs = within(
            canvasElement.querySelector(".seq-results-selector__area-row") as HTMLElement
        ).getAllByRole("tab")
        await userEvent.click(areaTabs[2])
        await waitFor(() =>
            expect(candidateVotes(canvasElement, /^Alice/)).toEqual(["Alice", "17"])
        )
        await expect(areaTabs[2]).toHaveAttribute("aria-selected", "true")
    },
}

export const ChoosingAnElectionShowsItsContests: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("tab", {name: "Deputy"}))
        await waitFor(() => expect(tabNames(canvasElement, "contest")).toEqual(["Deputy"]))
        expect(candidateVotes(canvasElement, /Carol Example/)).toEqual(["Carol Example", "21"])
        expect(canvas.queryByRole("row", {name: /^Alice/})).toBeNull()
    },
}

export const DownloadElectionResults: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const actions = canvasElement.querySelector(
            ".seq-results-selector__election-row__actions"
        ) as HTMLElement
        await userEvent.click(within(actions).getByLabelText("export election data"))
        const menu = within(await within(document.body).findByRole("menu"))
        await userEvent.click(
            menu.getByRole("menuitem", {
                name: i18n.t("common.label.exportFormat", {item: "Council", format: "JSON"}),
            })
        )
        await exportMenuHidden()
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "report.json", href: documentUrl(RESULT_DOCUMENTS.election.json)},
            ])
        )
        expect(boundary.calls.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "FetchDocument",
                variables: {electionEventId: EVENT_ID, documentId: RESULT_DOCUMENTS.election.json},
            },
        ])
    },
}

export const WithoutExportPermission: Story = {
    globals: {permissions: EStoryPermissions.NONE},
    play: async ({canvasElement}) => {
        expect(candidateVotes(canvasElement, /^Alice/)).toEqual(["Alice", "50"])
        expect(within(canvasElement).queryByLabelText("export election data")).toBeNull()
    },
}

export const WaitingForResults: Story = {
    args: {loaded: false},
    play: async ({canvasElement}) => {
        expect(canvasElement.querySelector(".seq-admin-tally-results__loading")).not.toBeNull()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
    },
}

export const TallyWithoutElections: Story = {
    args: {electionIds: []},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText(i18n.t("common.label.noResult"))).toBeVisible()
        expect(within(canvasElement).queryByRole("tab")).toBeNull()
    },
}

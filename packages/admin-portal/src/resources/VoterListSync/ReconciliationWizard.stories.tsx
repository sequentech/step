// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {json} from "@sequentech/ui-test-kit/mocks/http"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {documentHandlers, documentUrl, recordDownloads} from "@/__stories__/downloads"
import {type FetchCall, storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ETasksExecution} from "@/types/tasksExecution"
import {ReconciliationWizard} from "./ReconciliationWizard"
import {
    APPLY_TASK_ID,
    DIFF_ID,
    ERound,
    FILE_ID,
    GENERATE_TASK_ID,
    PATCH_ID,
    UPLOAD_URL,
    envelope,
    task,
} from "./__stories__/ReconciliationFixture"

interface Scenario {
    open: boolean
    round: ERound
    /** How the diff task ends; RUNNING keeps it calculating. */
    generate: ETaskExecutionStatus
    /** How the apply task ends. */
    apply: ETaskExecutionStatus
    /** The apply task excludes rows, reporting fewer than it counted. */
    applyRowFailures: boolean
    /** The upload service hands out no address. */
    uploadUnavailable: boolean
    onClose: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let fetches: {calls: FetchCall[]}
let downloads: ReturnType<typeof recordDownloads>

const meta = {
    title: "Admin/Voter list sync/ReconciliationWizard",
    component: ReconciliationWizard,
    args: {
        open: true,
        round: ERound.SEQUENT_ONLY,
        generate: ETaskExecutionStatus.SUCCESS,
        apply: ETaskExecutionStatus.SUCCESS,
        applyRowFailures: false,
        uploadUnavailable: false,
        onClose: fn(),
    },
    argTypes: {
        round: {control: "select", options: Object.values(ERound)},
        generate: {
            control: "inline-radio",
            options: [
                ETaskExecutionStatus.SUCCESS,
                ETaskExecutionStatus.FAILED,
                ETaskExecutionStatus.IN_PROGRESS,
            ],
        },
        apply: {
            control: "inline-radio",
            options: [ETaskExecutionStatus.SUCCESS, ETaskExecutionStatus.FAILED],
        },
    },
    beforeEach: async ({args}) => {
        const applyAnnotations = args.applyRowFailures
            ? {
                  reconciliation_row_failures: [
                      {voter_id: "voter-005", reason: "Voter changed while applying"},
                  ],
                  reconciliation_row_failure_count: 3,
                  reconciliation_row_failures_truncated: true,
              }
            : {}
        graphql = graphqlBoundary(
            {
                GetUploadUrl: () => ({
                    data: {
                        get_upload_url: args.uploadUnavailable
                            ? null
                            : {url: UPLOAD_URL, document_id: FILE_ID},
                    },
                }),
                CreateExternalReconciliationImport: () => ({
                    data: {
                        create_external_reconciliation_import: {
                            task_execution: {id: GENERATE_TASK_ID},
                        },
                    },
                }),
                ApplyExternalReconciliationChanges: () => ({
                    data: {
                        apply_external_reconciliation_changes: {
                            task_execution: {id: APPLY_TASK_ID},
                        },
                    },
                }),
                GetTaskById: ({variables}) => ({
                    data: {
                        sequent_backend_tasks_execution: [
                            variables.task_id === GENERATE_TASK_ID
                                ? task(
                                      GENERATE_TASK_ID,
                                      ETasksExecution.GENERATE_RECONCILIATION_PATCHES,
                                      args.generate,
                                      {document_id: DIFF_ID}
                                  )
                                : task(
                                      APPLY_TASK_ID,
                                      ETasksExecution.APPLY_RECONCILIATION_PATCH,
                                      args.apply,
                                      applyAnnotations
                                  ),
                        ],
                    },
                }),
                ...documentHandlers({
                    [DIFF_ID]: {name: "reconciliation-diff.json"},
                    [PATCH_ID]: {name: "external-patch.csv"},
                }),
            },
            {schema: true}
        )
        fetches = storyFetch({
            [UPLOAD_URL]: () => ({status: 200}),
            [documentUrl(DIFF_ID)]: () => json(200, envelope(args.round)),
        })
        downloads = recordDownloads()
        await graphql.ready
        return () => downloads.restore()
    },
    parameters: {
        expectedFailure: {
            reason: "The wizard drawer is a dialog without an accessible name.",
            a11y: ["aria-dialog-name"],
        },
    },
    render: ({open, onClose}) => (
        <AdminStoryProvider boundary={graphql}>
            <ReconciliationWizard open={open} electionEventId={EVENT_ID} onClose={onClose} />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const diffTables = {
    expectedFailure: {
        reason: "The wizard drawer has no accessible name, and the diff tables dim struck-through values and put white chip labels on light colours.",
        a11y: ["aria-dialog-name", "color-contrast"],
    },
}

const FILE_NAME = "datafix-sequence-42.csv"
const body = () => within(document.body)
const names = () => graphql.calls.map((call) => call.name)

async function dropFile() {
    const drawer = await body().findByRole("presentation")
    const input = drawer.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("Missing reconciliation file chooser")
    await userEvent.upload(input, new File(["VoterID,Channel\n"], FILE_NAME, {type: "text/csv"}))
}

async function review() {
    await dropFile()
    await expect(await body().findByText(`${FILE_NAME} - Sequence 42, generated -`)).toBeVisible()
    expect(names()).toEqual([
        "GetUploadUrl",
        "CreateExternalReconciliationImport",
        "GetTaskById",
        "FetchDocument",
    ])
    expect(fetches.calls.map(({method, url}) => ({method, url}))).toEqual([
        {method: "PUT", url: UPLOAD_URL},
        {method: "GET", url: documentUrl(DIFF_ID)},
    ])
}

const applyButton = () => body().getByRole("button", {name: "Apply"})

const CONFIRM = "Confirm reconciliation changes"

async function confirm(button: string) {
    await userEvent.click(applyButton())
    const dialog = await body().findByRole("dialog", {name: CONFIRM})
    await userEvent.click(within(dialog).getByRole("button", {name: button}))
    await waitFor(() => expect(body().queryByRole("dialog", {name: CONFIRM})).toBeNull())
    return dialog
}

export const DropTheFile: Story = {
    play: async ({args}) => {
        await expect(await body().findByText("External reconciliation sync")).toBeVisible()
        expect(body().getByText("CSV file")).toBeVisible()
        await userEvent.click(body().getByRole("button", {name: "Cancel"}))
        expect(args.onClose).toHaveBeenCalledTimes(1)
        expect(graphql.calls).toEqual([])
    },
}

export const Closed: Story = {
    args: {open: false},
    parameters: {expectedFailure: null},
    play: async () => {
        expect(body().queryByText("External reconciliation sync")).toBeNull()
        expect(graphql.calls).toEqual([])
    },
}

export const CalculatingTheDiffs: Story = {
    args: {generate: ETaskExecutionStatus.IN_PROGRESS},
    parameters: {
        expectedFailure: {
            reason: "The wizard drawer has no accessible name, nor has its progress spinner.",
            a11y: ["aria-dialog-name", "aria-progressbar-name"],
        },
    },
    play: async () => {
        await dropFile()
        await expect(
            await body().findByText(`Uploading ${FILE_NAME} and calculating both diffs...`)
        ).toBeVisible()
        expect(body().getByRole("button", {name: "Cancel"})).toBeDisabled()
        await waitFor(() => expect(names()).toContain("GetTaskById"))
        expect(graphql.calls[0].variables).toEqual({
            name: FILE_NAME,
            media_type: "text/csv",
            size: 16,
            is_public: false,
            election_event_id: EVENT_ID,
        })
        expect(fetches.calls[0].headers["content-type"]).toBe("text/csv")
        expect(graphql.calls[1].variables).toEqual({
            election_event_id: EVENT_ID,
            document_id: FILE_ID,
        })
    },
}

export const ExternalPatchPending: Story = {
    args: {round: ERound.EXTERNAL_PENDING},
    parameters: diffTables,
    play: async () => {
        await review()
        expect(body().getByText("External diff (1)")).toBeVisible()
        expect(body().getByText("voter-003")).toBeVisible()
        // Apply waits until the external system has taken its patch.
        expect(applyButton()).toBeDisabled()
        await userEvent.click(body().getByRole("button", {name: "Download external patch"}))
        await waitFor(() =>
            expect(downloads.downloads).toEqual([
                {name: `external-reconciliation-${PATCH_ID}.csv`, href: documentUrl(PATCH_ID)},
            ])
        )
    },
}

export const ApplySequentChanges: Story = {
    play: async ({args}) => {
        await review()
        expect(body().getByText("External diff")).toBeVisible()
        expect(body().getByText("No external-side differences.")).toBeVisible()
        await userEvent.click(applyButton())
        await expect(
            await body().findByText(
                "This will apply changes that marks 1 voter(s) as voted via other channels, updates 1 profile(s)."
            )
        ).toBeVisible()
        await userEvent.click(body().getByRole("button", {name: "Apply changes"}))
        await waitFor(() => expect(body().queryByRole("dialog", {name: CONFIRM})).toBeNull())
        await expect(
            await body().findByText("All Sequent-side changes applied successfully.")
        ).toBeVisible()
        expect(
            graphql.calls.find(({name}) => name === "ApplyExternalReconciliationChanges")?.variables
        ).toEqual({election_event_id: EVENT_ID, diff_document_id: DIFF_ID})
        await userEvent.click(body().getByRole("button", {name: "Close"}))
        expect(args.onClose).toHaveBeenCalledTimes(1)
    },
}

export const CancelTheConfirmation: Story = {
    parameters: diffTables,
    play: async () => {
        await review()
        await confirm("Cancel")
        expect(applyButton()).toBeEnabled()
        expect(names()).not.toContain("ApplyExternalReconciliationChanges")
    },
}

export const AlreadyInSync: Story = {
    args: {round: ERound.IN_SYNC},
    play: async () => {
        await review()
        expect(
            body().getByText("No differences - the two systems are already in sync.")
        ).toBeVisible()
        // A round without changes is still applied, which records its Sequence.
        await userEvent.click(body().getByRole("button", {name: "Next"}))
        await expect(
            await body().findByText("All Sequent-side changes applied successfully.")
        ).toBeVisible()
        expect(names()).toContain("ApplyExternalReconciliationChanges")
    },
}

export const ConvergenceCheck: Story = {
    args: {round: ERound.CONVERGENCE_CHECK},
    parameters: diffTables,
    play: async () => {
        await review()
        expect(body().getByText(/This is a convergence check/)).toBeVisible()
        expect(applyButton()).toBeDisabled()
    },
}

export const RowFailures: Story = {
    args: {round: ERound.ROW_FAILURES, applyRowFailures: true},
    parameters: diffTables,
    play: async () => {
        await review()
        expect(body().getByText(/1 row\(s\) could not be reconciled safely/)).toBeVisible()
        expect(body().getByRole("gridcell", {name: "voter-004"})).toBeVisible()
        await confirm("Apply changes")
        await expect(await body().findByText(/3 row\(s\) were excluded/)).toBeVisible()
        expect(
            body().getByText("Showing the first 2 of 3 row failures.", {exact: false})
        ).toBeVisible()
        expect(body().getByRole("gridcell", {name: "voter-005"})).toBeVisible()
    },
}

export const BackStartsOver: Story = {
    play: async () => {
        await review()
        await userEvent.click(body().getByRole("button", {name: "Back"}))
        await expect(await body().findByText("CSV file")).toBeVisible()
        expect(body().queryByText(/Sequence 42/)).toBeNull()
    },
}

export const DiffCalculationFails: Story = {
    args: {generate: ETaskExecutionStatus.FAILED},
    play: async () => {
        await dropFile()
        await expect(
            await body().findByText(
                "Failed to calculate the reconciliation diff - see the task widget for details."
            )
        ).toBeVisible()
        expect(body().getByText("CSV file")).toBeVisible()
        expect(names()).not.toContain("FetchDocument")
    },
}

export const UploadUnavailable: Story = {
    args: {uploadUnavailable: true},
    play: async () => {
        await dropFile()
        await expect(await body().findByText("Failed to get an upload URL")).toBeVisible()
        expect(names()).toEqual(["GetUploadUrl"])
        expect(fetches.calls).toEqual([])
    },
}

export const ApplyFails: Story = {
    args: {apply: ETaskExecutionStatus.FAILED},
    play: async () => {
        await review()
        await confirm("Apply changes")
        await expect(
            await body().findByText(
                "Failed to apply the Sequent-side changes - see the task widget for details."
            )
        ).toBeVisible()
        expect(body().queryByText("All Sequent-side changes applied successfully.")).toBeNull()
        expect(body().getByRole("button", {name: "Start over"})).toBeVisible()
    },
}

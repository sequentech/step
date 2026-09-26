// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {FIXED_TIME, STORY_IDS, eventRecord, storyId} from "@/__stories__/fixtures"
import {documentUrl, recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import {storyFetch, type FetchCall} from "@/__stories__/storyNetwork"
import {IPermissions} from "@/types/keycloak"
import {
    ETallySheetImportChangeType,
    ETallySheetImportItemStatus,
    ETallySheetImportReviewDecision,
    ETallySheetImportSourceFormat,
    ETallySheetImportStatus,
} from "@/types/TallySheets"
import {TallySheetImports} from "./TallySheetImports"

interface Scenario {
    roles: string[]
    /** Whether the event has any import. */
    imports: boolean
    /** Items of the pending import; more than 100 are paged. */
    itemCount: number
    /** Status the review service gives the reviewed import. */
    reviewedStatus: ETallySheetImportStatus
    /** Whether the preview of an uploaded file reports validation errors. */
    previewErrors: boolean
    openImportId: string | null
    onOpenImportHandled: () => void
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>
let downloads: RecordedDownload[]
let uploads: {calls: FetchCall[]}

const IDS = {
    pending: storyId(0, 1),
    failed: storyId(0, 2),
    pendingSource: storyId(0, 3),
    failedSource: storyId(0, 4),
    upload: storyId(0, 5),
    created: storyId(0, 6),
}
const UPLOAD_URL = "https://s3.admin-story.invalid/uploads/tally-sheet-import"
const UPLOADED_CSV = "area,contest,channel,candidate,votes\nNorth district,Members,PAPER,Alice,32\n"
const ALL_ROLES = [
    IPermissions.TALLY_SHEET_IMPORT_VIEW,
    IPermissions.TALLY_SHEET_IMPORT_CREATE,
    IPermissions.TALLY_SHEET_IMPORT_REVIEW,
]

const scope = {tenant_id: STORY_IDS.tenant, election_event_id: EVENT_ID}

const summary = (imported: number, changed: number, added: number, errors = 0) => ({
    imported_ballot_box_count: imported,
    changed_ballot_box_count: changed,
    new_ballot_box_count: added,
    unchanged_ballot_box_count: imported - changed - added,
    conflicted_ballot_box_count: 0,
    validation_error_count: errors,
})

const CENSUS_ERROR = {
    code: "total_votes_exceeds_census",
    message: "Total votes exceed the census",
    channel: "POSTAL",
    area_name: "South district",
    params: {totalVotes: "90", census: "80"},
}

function importRecord(sourceSha256: string) {
    return [
        {
            ...scope,
            id: IDS.pending,
            source_document_id: IDS.pendingSource,
            source_file_name: "north-paper.xml",
            source_format: ETallySheetImportSourceFormat.ESS_ENHANCED_XML,
            selected_channel: "PAPER",
            status: ETallySheetImportStatus.PENDING_REVIEW,
            source_sha256: sourceSha256,
            canonical_csv_sha256: "c0ffee",
            created_at: FIXED_TIME,
            last_updated_at: FIXED_TIME,
            created_by_user_id: STORY_IDS.user,
            summary: summary(2, 1, 1),
            validation_report: [],
            labels: {},
            annotations: {},
        },
        {
            ...scope,
            id: IDS.failed,
            source_document_id: IDS.failedSource,
            source_file_name: "south-postal.csv",
            source_format: ETallySheetImportSourceFormat.CANONICAL_CSV,
            selected_channel: "POSTAL",
            status: ETallySheetImportStatus.FAILED_VALIDATION,
            source_sha256: "5a17",
            canonical_csv_sha256: null,
            created_at: FIXED_TIME,
            last_updated_at: FIXED_TIME,
            created_by_user_id: STORY_IDS.secondUser,
            summary: summary(1, 0, 0, 1),
            validation_report: [CENSUS_ERROR],
            labels: {},
            annotations: {},
        },
    ]
}

const PREVIOUS_CSV = "candidate,votes\nAlice,30\nBob,20\n"
const INCOMING_CSV = "candidate,votes\nAlice,32\nBob,20\n"

function importItems(count: number) {
    return Array.from({length: count}, (_, index) => ({
        ...scope,
        id: `${storyId(9, 0).slice(0, -3)}${String(index).padStart(3, "0")}`,
        import_id: IDS.pending,
        election_id: STORY_IDS.election,
        area_id: index ? STORY_IDS.secondArea : STORY_IDS.area,
        contest_id: STORY_IDS.contest,
        channel: "PAPER",
        generated_tally_sheet_id: null,
        baseline_approved_tally_sheet_id: index ? null : storyId(0, 7),
        baseline_approved_version: index ? null : 1,
        change_type: index ? ETallySheetImportChangeType.NEW : ETallySheetImportChangeType.CHANGED,
        status: ETallySheetImportItemStatus.PENDING_REVIEW,
        previous_csv: index ? null : PREVIOUS_CSV,
        incoming_csv: INCOMING_CSV,
        source_refs: {
            area_name: index ? `South district ${index}` : "North district",
            contest_external_id: "MEMBERS",
            candidate_external_ids: ["ALICE", "BOB"],
        },
        created_at: new Date(Date.parse(FIXED_TIME) + index * 1000).toISOString(),
        last_updated_at: FIXED_TIME,
    }))
}

const user = (id: string, username: string) => ({
    id,
    username,
    attributes: {},
    email: `${username}@example.invalid`,
    email_verified: true,
    enabled: true,
    first_name: null,
    last_name: null,
    groups: [],
    area: null,
    votes_info: null,
})

async function sha256(text: string) {
    const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text))
    return Array.from(new Uint8Array(digest))
        .map((byte) => byte.toString(16).padStart(2, "0"))
        .join("")
}

const meta = {
    title: "Admin/Tally sheet import/TallySheetImports",
    component: TallySheetImports,
    args: {
        roles: ALL_ROLES,
        imports: true,
        itemCount: 2,
        reviewedStatus: ETallySheetImportStatus.APPROVED,
        previewErrors: false,
        openImportId: null,
        onOpenImportHandled: fn(),
    },
    argTypes: {
        reviewedStatus: {control: "select", options: Object.values(ETallySheetImportStatus)},
        onOpenImportHandled: {table: {disable: true}},
    },
    parameters: {
        widgets: ["TallySheetImportsDatagrid", "ImportActions", "Status"],
    },
    beforeEach: async ({args}) => {
        const records = importRecord("0ld5ha")
        data = resourceBoundary({
            sequent_backend_tally_sheet_import: args.imports ? records : [],
            sequent_backend_tally_sheet_import_item: importItems(args.itemCount),
        })
        const uploadedSha = await sha256(UPLOADED_CSV)
        graphql = graphqlBoundary(
            {
                getUsers: ({variables}) => {
                    const users = [
                        user(STORY_IDS.user, "clerk.north"),
                        user(STORY_IDS.secondUser, "clerk.south"),
                    ].filter(({id}) => (variables.userIds as string[]).includes(id))
                    return {
                        data: {
                            get_users: {items: users, total: {aggregate: {count: users.length}}},
                        },
                    }
                },
                ReviewTallySheetImport: ({variables}) => ({
                    data: {
                        review_tally_sheet_import: {
                            import: {
                                ...records[0],
                                status:
                                    variables.decision === ETallySheetImportReviewDecision.APPROVE
                                        ? args.reviewedStatus
                                        : ETallySheetImportStatus.DISAPPROVED,
                            },
                        },
                    },
                }),
                FetchDocument: ({variables}) => ({
                    data: {fetchDocument: {url: documentUrl(String(variables.documentId))}},
                }),
                GetUploadUrl: () => ({
                    data: {get_upload_url: {url: UPLOAD_URL, document_id: IDS.upload}},
                }),
                PreviewTallySheetImport: ({variables}) => ({
                    data: {
                        preview_tally_sheet_import: {
                            preview: {
                                document_id: variables.documentId,
                                source_format: variables.sourceFormat,
                                selected_channel: variables.selectedChannel,
                                summary: summary(1, 1, 0, args.previewErrors ? 1 : 0),
                                items: [
                                    {
                                        channel: variables.selectedChannel,
                                        area_id: STORY_IDS.area,
                                        area_name: "North district",
                                        contest_id: STORY_IDS.contest,
                                        contest_name: "Members",
                                        election_id: STORY_IDS.election,
                                        baseline_tally_sheet_id: storyId(0, 7),
                                        baseline_version: 1,
                                        previous_csv: PREVIOUS_CSV,
                                        incoming_csv: INCOMING_CSV,
                                        incoming_content_hash: uploadedSha,
                                        change_type: ETallySheetImportChangeType.CHANGED,
                                    },
                                ],
                                validation_errors: args.previewErrors ? [CENSUS_ERROR] : [],
                            },
                        },
                    },
                }),
                CreateTallySheetImport: ({variables}) => ({
                    data: {
                        create_tally_sheet_import: {
                            import: {
                                ...records[0],
                                id: IDS.created,
                                source_document_id: variables.documentId,
                                source_sha256: variables.sha256,
                            },
                        },
                    },
                }),
            },
            {schema: true}
        )
        await graphql.ready
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({roles, openImportId, onOpenImportHandled}) => (
        <AdminStoryProvider boundary={graphql} dataProvider={data.provider} roles={roles}>
            <RecordContextProvider value={eventRecord()}>
                <TallySheetImports
                    openImportId={openImportId}
                    onOpenImportHandled={onOpenImportHandled}
                />
            </RecordContextProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const importRow = (canvasElement: HTMLElement, file: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(file.replace(".", "\\."))})

/** The row action whose tooltip has this title. */
const action = (row: HTMLElement, title: string) =>
    within(row).getByLabelText(title).closest("button") as HTMLButtonElement

const operations = () => graphql.calls.map(({name}) => name)

async function openDetail(canvasElement: HTMLElement, file: string) {
    const row = await importRow(canvasElement, file)
    await userEvent.click(action(row, "Review"))
    return within(await within(document.body).findByRole("presentation"))
}

async function chooseFile(canvasElement: HTMLElement) {
    await userEvent.click(
        within(canvasElement).getAllByRole("button", {name: "Import tally sheets"})[0]
    )
    const drawer = await within(document.body).findByRole("presentation")
    await userEvent.click(within(drawer).getByRole("combobox", {name: "Format"}))
    await userEvent.click(await within(document.body).findByRole("option", {name: "Canonical CSV"}))
    const input = drawer.querySelector<HTMLInputElement>('input[type="file"]')
    if (!input) throw new Error("The import drawer has no file input")
    await userEvent.upload(input, new File([UPLOADED_CSV], "north-paper.csv", {type: "text/csv"}))
    return within(drawer)
}

const drawerDefects = {
    expectedFailure: {
        reason: "The import drawers are dialogs without an accessible name.",
        a11y: ["aria-dialog-name"],
    },
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Tally sheet imports")).toBeVisible()
        const pending = within(await importRow(canvasElement, "north-paper.xml"))
        await expect(await pending.findByText("clerk.north")).toBeVisible()
        await expect(pending.getByText("ES&S Enhanced XML")).toBeVisible()
        await expect(pending.getByText("Paper")).toBeVisible()
        await expect(pending.getByText("Pending review")).toBeVisible()
        const failed = within(await importRow(canvasElement, "south-postal.csv"))
        await expect(await failed.findByText("clerk.south")).toBeVisible()
        await expect(failed.getByText("Canonical CSV")).toBeVisible()
        await expect(failed.getByText("Failed validation")).toBeVisible()
        expect(operations()).toEqual(["getUsers"])
        expect(data.writes).toEqual([])
    },
}

export const ReviewAndApproveAnImport: Story = {
    parameters: {
        widgets: [
            "ImportMetadata",
            "MetadataLine",
            "ImportSummary",
            "SummaryChip",
            "Status",
            "CsvDiffView",
        ],
        ...drawerDefects,
    },
    play: async ({canvasElement}) => {
        const drawer = await openDetail(canvasElement, "north-paper.xml")
        await expect(drawer.getByText("Tally sheet import")).toBeVisible()
        await expect(await drawer.findByText("clerk.north")).toBeVisible()
        const imported = drawer.getByText("Imported").parentElement as HTMLElement
        expect(within(imported).getByRole("heading").textContent).toBe("2")
        const item = await drawer.findByRole("button", {name: /North district \/ MEMBERS/})
        await userEvent.click(item)
        await expect(await drawer.findByText("Alice,32")).toBeVisible()
        await expect(drawer.getByText("Alice,30")).toHaveStyle({textDecoration: "line-through"})
        await expect(drawer.getByText(/Source candidate IDs: ALICE, BOB/)).toBeVisible()
        await userEvent.click(drawer.getByRole("button", {name: "Approve"}))
        await waitFor(() =>
            expect(within(document.body).getByText("Import approved")).toBeVisible()
        )
        expect(graphql.calls.filter(({name}) => name === "ReviewTallySheetImport")).toEqual([
            expect.objectContaining({
                variables: {
                    electionEventId: EVENT_ID,
                    importId: IDS.pending,
                    decision: ETallySheetImportReviewDecision.APPROVE,
                },
            }),
        ])
        // An approved import cannot be reviewed again.
        await waitFor(() => expect(drawer.queryByRole("button", {name: "Approve"})).toBeNull())
        await expect(drawer.getAllByText("Approved")[0]).toBeVisible()
    },
}

export const ApprovalFindsConflicts: Story = {
    args: {reviewedStatus: ETallySheetImportStatus.CONFLICTED},
    parameters: {
        widgets: ["ImportMetadata", "ImportSummary", "Status"],
        expectedFailure: {
            reason:
                "The import drawer is a dialog without an accessible name, and the conflicted " +
                "status chip and warning notice have white text on orange below 4.5 contrast.",
            a11y: ["aria-dialog-name", "color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const drawer = await openDetail(canvasElement, "north-paper.xml")
        await userEvent.click(await drawer.findByRole("button", {name: "Approve"}))
        await waitFor(() =>
            expect(
                within(document.body).getByText("Import has stale baseline conflicts")
            ).toBeVisible()
        )
        // A conflicted import can be reviewed again.
        await expect(drawer.getByRole("button", {name: "Approve"})).toBeEnabled()
    },
}

export const DisapproveAnImport: Story = {
    parameters: {widgets: ["ImportMetadata", "ImportSummary", "Status"], ...drawerDefects},
    play: async ({canvasElement}) => {
        const drawer = await openDetail(canvasElement, "north-paper.xml")
        await userEvent.click(await drawer.findByRole("button", {name: "Disapprove"}))
        await waitFor(() =>
            expect(within(document.body).getByText("Import disapproved")).toBeVisible()
        )
        expect(
            graphql.calls
                .filter(({name}) => name === "ReviewTallySheetImport")
                .map(({variables}) => variables.decision)
        ).toEqual([ETallySheetImportReviewDecision.DISAPPROVE])
    },
}

export const FailedValidation: Story = {
    parameters: {widgets: ["ValidationErrors", "ImportSummary", "Status"]},
    play: async ({canvasElement}) => {
        const drawer = await openDetail(canvasElement, "south-postal.csv")
        await expect(
            await drawer.findByText("Total votes (90) must not be greater than census (80)")
        ).toBeVisible()
        expect(drawer.queryByRole("button", {name: "Approve"})).toBeNull()
        await userEvent.click(drawer.getByRole("button", {name: "Close"}))
        await waitFor(() =>
            expect(within(document.body).queryByText("Tally sheet import")).toBeNull()
        )
    },
}

export const PagedImportItems: Story = {
    args: {itemCount: 101},
    parameters: {widgets: ["DetailItemsPagination", "Status"], ...drawerDefects},
    play: async ({canvasElement}) => {
        const drawer = await openDetail(canvasElement, "north-paper.xml")
        await expect(await drawer.findByText("1-100 of 101")).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Previous"})).toBeDisabled()
        await userEvent.click(drawer.getByRole("button", {name: "Next"}))
        await expect(await drawer.findByText("101-101 of 101")).toBeVisible()
        await expect(
            drawer.getByRole("button", {name: /South district 100 \/ MEMBERS/})
        ).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Next"})).toBeDisabled()
    },
}

export const OpenAnImportFromALink: Story = {
    args: {openImportId: IDS.failed},
    parameters: {widgets: ["ValidationErrors", "ImportMetadata"], ...drawerDefects},
    play: async ({args}) => {
        const drawer = within(await within(document.body).findByRole("presentation"))
        await expect(await drawer.findByText(IDS.failed)).toBeVisible()
        await waitFor(() => expect(args.onOpenImportHandled).toHaveBeenCalledTimes(1))
    },
}

export const DownloadASourceFile: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(action(await importRow(canvasElement, "north-paper.xml"), "Source"))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "north-paper.xml", href: documentUrl(IDS.pendingSource)},
            ])
        )
        expect(graphql.calls.filter(({name}) => name === "FetchDocument")).toEqual([
            expect.objectContaining({
                variables: {electionEventId: EVENT_ID, documentId: IDS.pendingSource},
            }),
        ])
    },
}

export const PreviewAndSaveAnImport: Story = {
    parameters: {widgets: ["PreviewPanel", "ImportSummary", "SummaryChip", "Status"]},
    play: async ({canvasElement}) => {
        const drawer = await chooseFile(canvasElement)
        await userEvent.click(drawer.getByRole("button", {name: "Preview"}))
        await expect(
            await drawer.findByRole("button", {name: /North district \/ Members/})
        ).toBeVisible()
        const uploadedSha = await sha256(UPLOADED_CSV)
        expect(uploads.calls.map(({method, url, body}) => ({method, url, body}))).toEqual([
            {method: "PUT", url: UPLOAD_URL, body: UPLOADED_CSV},
        ])
        await userEvent.click(drawer.getByRole("button", {name: "Save import"}))
        await waitFor(() =>
            expect(within(document.body).getByText("Tally sheet import created")).toBeVisible()
        )
        const imports = graphql.calls.filter(({name}) =>
            ["GetUploadUrl", "PreviewTallySheetImport", "CreateTallySheetImport"].includes(name)
        )
        expect(imports.map(({name, variables}) => ({name, variables}))).toEqual([
            {
                name: "GetUploadUrl",
                variables: {
                    name: "north-paper.csv",
                    media_type: "text/csv",
                    size: UPLOADED_CSV.length,
                    is_public: false,
                    election_event_id: EVENT_ID,
                },
            },
            ...["PreviewTallySheetImport", "CreateTallySheetImport"].map((name) => ({
                name,
                variables: {
                    electionEventId: EVENT_ID,
                    documentId: IDS.upload,
                    sha256: uploadedSha,
                    sourceFormat: ETallySheetImportSourceFormat.CANONICAL_CSV,
                    selectedChannel: "PAPER",
                },
            })),
        ])
    },
}

export const PreviewWithValidationErrors: Story = {
    args: {previewErrors: true},
    parameters: {widgets: ["PreviewPanel", "ValidationErrors"], ...drawerDefects},
    play: async ({canvasElement}) => {
        const drawer = await chooseFile(canvasElement)
        await userEvent.click(drawer.getByRole("button", {name: "Preview"}))
        await expect(
            await drawer.findByText("Total votes (90) must not be greater than census (80)")
        ).toBeVisible()
        await expect(drawer.getByRole("button", {name: "Save import"})).toBeDisabled()
        expect(operations()).not.toContain("CreateTallySheetImport")
    },
}

export const Empty: Story = {
    args: {imports: false},
    parameters: {widgets: [], ...drawerDefects},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("No tally sheet imports yet.")).toBeVisible()
        await userEvent.click(canvas.getAllByRole("button", {name: "Import tally sheets"})[0])
        await expect(
            await within(document.body).findByRole("combobox", {name: "Channel"})
        ).toBeVisible()
    },
}

export const ReadOnly: Story = {
    args: {roles: [IPermissions.TALLY_SHEET_IMPORT_VIEW]},
    parameters: {widgets: ["ImportMetadata", "Status"], ...drawerDefects},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await importRow(canvasElement, "north-paper.xml")
        expect(canvas.queryByRole("button", {name: "Import tally sheets"})).toBeNull()
        const drawer = await openDetail(canvasElement, "north-paper.xml")
        await drawer.findByText("clerk.north")
        expect(drawer.queryByRole("button", {name: "Approve"})).toBeNull()
    },
}

export const WithoutViewPermission: Story = {
    args: {roles: [IPermissions.TALLY_SHEET_IMPORT_CREATE]},
    parameters: {widgets: []},
    play: async ({canvasElement}) => {
        expect(within(canvasElement).queryByText("Tally sheet imports")).toBeNull()
        expect(within(canvasElement).queryByRole("button")).toBeNull()
        expect(data.calls).toEqual([])
    },
}

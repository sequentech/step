// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {formatDateTimeZone, i18n, zonedToInstant} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {MyTimeZoneProvider, adminDateTimeFormat} from "@/components/timezones/timeZoneService"
import {
    MY_TIME_ZONE,
    overseasConfiguration,
} from "@/components/timezones/__fixtures__/configurations"
import type {IScheduleImportRow} from "@/types/lifecycle"
import {ImportScheduleDrawer} from "./ImportScheduleDrawer"

interface Scenario {
    /** Whether the file has a row that needs attention (a time in a DST gap). */
    errors: boolean
    onClose: Mock<() => void>
    onImported: Mock<() => void>
}

const UPLOAD_URL = "https://s3.admin-story.invalid/upload/"
const DOCUMENT_ID = "schedule-csv"
const PRIMARY = overseasConfiguration().presentation.timezones!.primary
const TEXT = {t: i18n.t, lang: "en", formatDateTime: adminDateTimeFormat("en")}

/** Enrollment opens at 00:00 in each Post's zone; with errors, a Toronto row falls in the DST gap. */
const previewRows = (errors: boolean): Array<IScheduleImportRow> => {
    const posts = overseasConfiguration().elections
    const rows = posts.slice(0, 7).map((post, index) => ({
        row: index + 2,
        election_alias: post.name,
        election_id: post.id,
        election_name: post.name,
        event_type: "START_ENROLLMENT_PERIOD",
        local: "2028-02-09T00:00",
        time_zone: post.timezone!,
        instant: zonedToInstant("2028-02-09T00:00", post.timezone!).instant,
        primary_local: null,
        error_code: null,
    }))
    if (!errors) return rows
    return [
        ...rows,
        {
            row: 106,
            election_alias: "Toronto PCG",
            election_id: posts[7].id,
            election_name: "Toronto PCG",
            event_type: "START_TEST_VOTING",
            local: "2028-03-12T02:30",
            time_zone: "America/Toronto",
            instant: null,
            primary_local: null,
            error_code: "dst-gap",
        },
    ]
}

let graphql: ReturnType<typeof graphqlBoundary>
let uploads: ReturnType<typeof storyFetch>

const meta = {
    title: "Admin/Scheduled events/ImportScheduleDrawer",
    component: ImportScheduleDrawer,
    args: {errors: true, onClose: fn(), onImported: fn()},
    beforeEach: ({args}) => {
        graphql = graphqlBoundary(
            {
                GetUploadUrl: () => ({
                    data: {
                        get_upload_url: {
                            url: `${UPLOAD_URL}${DOCUMENT_ID}`,
                            document_id: DOCUMENT_ID,
                        },
                    },
                }),
                PreviewScheduleImport: () => {
                    const rows = previewRows(args.errors)
                    return {
                        data: {
                            preview_schedule_import: {
                                rows,
                                ok: rows.filter(({error_code}) => !error_code).length,
                                posts: rows.filter(({error_code}) => !error_code).length,
                                errors: rows.filter(({error_code}) => error_code).length,
                                primary_time_zone: PRIMARY,
                            },
                        },
                    }
                },
                ImportSchedule: () => ({data: {import_schedule: {created: 7, updated: 0}}}),
            },
            {schema: true}
        )
        uploads = storyFetch({[UPLOAD_URL]: () => ({status: 200})})
    },
    render: ({onClose, onImported}) => (
        <AdminStoryProvider boundary={graphql}>
            <MyTimeZoneProvider zone={MY_TIME_ZONE}>
                <ImportScheduleDrawer
                    electionEventId={EVENT_ID}
                    primary={PRIMARY}
                    onClose={onClose}
                    onImported={onImported}
                />
            </MyTimeZoneProvider>
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const drawer = async () => within(await within(document.body).findByRole("presentation"))

async function chooseFile() {
    const input = document.body.querySelector<HTMLInputElement>('[data-testid="schedule-file"]')
    if (!input) throw new Error("The file input is missing")
    await userEvent.upload(
        input,
        new File(["election_alias,event_type,local_date_time\n"], "ov2028-schedule.csv", {
            type: "text/csv",
        })
    )
}

const importButton = async () =>
    (await drawer()).getByRole("button", {name: i18n.t("common.label.import")})

/** tz-schedule-import: the row that needs attention comes first; Import waits for a clean file. */
export const RowNeedsAttention: Story = {
    parameters: {widgets: ["PreviewRow"]},
    play: async () => {
        await chooseFile()
        const view = await drawer()
        const summary = await view.findByTestId("import-summary")
        await expect(summary).toHaveTextContent(
            i18n.t("lifecycle.import.needsAttention", {ok: 7, posts: 7, count: 1})
        )
        const [header, first] = view.getAllByRole("row")
        expect(header).toBeDefined()
        await expect(first).toHaveAttribute("data-testid", "import-row-106")
        await expect(within(first).getByText(/does not exist in Toronto/)).toBeVisible()
        await expect(await importButton()).toBeDisabled()
        expect(uploads.calls).toEqual([
            expect.objectContaining({method: "PUT", url: `${UPLOAD_URL}${DOCUMENT_ID}`}),
        ])
    },
}

/** A clean file: each row in its zone and in the primary; Import imports it. */
export const CleanFile: Story = {
    args: {errors: false},
    play: async ({args}) => {
        await chooseFile()
        const view = await drawer()
        await expect(await view.findByTestId("import-summary")).toHaveTextContent(
            i18n.t("lifecycle.import.ready", {ok: 7, posts: 7})
        )
        const dubai = previewRows(false)[0]
        const row = within(view.getByTestId(`import-row-${dubai.row}`))
        await expect(
            row.getByText(formatDateTimeZone(dubai.instant!, dubai.time_zone, TEXT))
        ).toBeVisible()
        await expect(row.getByText(formatDateTimeZone(dubai.instant!, PRIMARY, TEXT))).toBeVisible()
        await userEvent.click(await importButton())
        await waitFor(() => expect(args.onImported).toHaveBeenCalled())
        expect(graphql.calls.find(({name}) => name === "ImportSchedule")?.variables).toEqual({
            electionEventId: EVENT_ID,
            documentId: DOCUMENT_ID,
        })
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import type {Mock} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {documentUrl, recordDownloads, type RecordedDownload} from "@/__stories__/downloads"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {FIXED_TIME, eventRecord, storyId} from "@/__stories__/fixtures"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {EStoryPermissions} from "../../../../ui-essentials/.storybook/globals"
import {ListApprovals} from "./ListApprovals"
import {
    APPLICATION_ID,
    ApprovalsScreen,
    EXPORT_DOCUMENT_ID,
    SECOND_APPLICATION_ID,
    graphqlCalls,
    listFilters,
    reads,
    setUpApprovals,
    type ApprovalServices,
} from "./__stories__/ApprovalsScreenFixture"

interface Scenario extends ApprovalServices {
    /** Whether the election event tabs have the event yet. */
    withEvent: boolean
    onViewApproval: Mock<(id: string | number) => void>
    onViewMatrix: Mock<() => void>
    onViewRule: Mock<(id: string | number) => void>
}

let downloads: RecordedDownload[]

const meta = {
    title: "Admin/Approvals/ListApprovals",
    component: ListApprovals,
    args: {
        reads: "records",
        empty: false,
        withEvent: true,
        onViewApproval: fn(),
        onViewMatrix: fn(),
        onViewRule: fn(),
    },
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    beforeEach: async ({args}) => {
        await setUpApprovals(args, {
            ExportApplication: () => ({
                data: {
                    export_application: {
                        error_msg: null,
                        document_id: EXPORT_DOCUMENT_ID,
                        task_execution: {
                            id: storyId(5, 7),
                            name: "Export applications",
                            execution_status: "IN_PROGRESS",
                            created_at: FIXED_TIME,
                            start_at: FIXED_TIME,
                            end_at: null,
                            logs: null,
                            annotations: null,
                            labels: null,
                            executed_by_user: "admin",
                            tenant_id: TENANT_ID,
                            election_event_id: EVENT_ID,
                            type: "EXPORT_APPLICATION",
                        },
                    },
                },
            }),
        })
        const recorder = recordDownloads()
        downloads = recorder.downloads
        return recorder.restore
    },
    render: ({withEvent, onViewApproval, onViewMatrix, onViewRule}) => (
        <ApprovalsScreen>
            <ListApprovals
                electionEventId={EVENT_ID}
                onViewApproval={onViewApproval}
                onViewMatrix={onViewMatrix}
                onViewRule={onViewRule}
                electionEventRecord={
                    withEvent ? (eventRecord() as Sequent_Backend_Election_Event) : undefined
                }
            />
        </ApprovalsScreen>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const applicationsRead = () => reads("getList", "sequent_backend_applications")

const row = (canvasElement: HTMLElement, applicant: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(applicant)})

const noRow = (canvasElement: HTMLElement, applicant: string) =>
    waitFor(() =>
        expect(within(canvasElement).queryByRole("row", {name: new RegExp(applicant)})).toBeNull()
    )

/** Opens the menu of a row and chooses one of its actions. */
const rowAction = async (canvasElement: HTMLElement, applicant: string, action: string) => {
    await userEvent.click(
        within(await row(canvasElement, applicant)).getByRole("button", {name: "Actions"})
    )
    await userEvent.click(await within(document.body).findByRole("menuitem", {name: action}))
}

export const Populated: Story = {
    parameters: {widgets: ["ApprovalsList", "PostCell"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Approvals")).toBeVisible()
        const alice = await row(canvasElement, "Alice Example")
        expect(
            canvas.getAllByRole("columnheader").map((header) => header.textContent?.trim())
        ).toEqual(["Voter", "What happened", "Post", "When", "Status", "Actions"])
        // A first visit shows the enrollments that wait for a person, the newest first.
        expect(canvas.getAllByRole("row").slice(1)).toEqual([
            alice,
            await row(canvasElement, "Carol Sample"),
        ])
        await expect(within(alice).getByText("alice@example.test")).toBeVisible()
        await expect(
            within(alice).getByText("Date of birth differs from the registry")
        ).toBeVisible()
        await expect(within(alice).getByText("ID scan verified")).toBeVisible()
        await expect(await within(alice).findByText("North district")).toBeVisible()
        await expect(within(alice).getByText(/^Waiting \d+ (day|days)$/)).toBeVisible()
        await expect(within(alice).getByText("Applied Jan 15, 2026")).toBeVisible()
        await expect(within(alice).getByText("Needs review")).toBeVisible()

        const carol = within(await row(canvasElement, "Carol Sample"))
        await expect(
            carol.getByText("Details typed by hand, not read from an ID scan")
        ).toBeVisible()
        await expect(carol.getByText("Needs a face-to-face check")).toBeVisible()
        await expect(await carol.findByText("South district")).toBeVisible()

        // ra-data-hasura compares the status column with `_ilike`.
        expect(applicationsRead()[0].args[1]).toMatchObject({
            filter: {"election_event_id": EVENT_ID, "status@_ilike": "pending"},
            sort: {field: "created_at", order: "DESC"},
        })
        expect(listFilters("sequent_backend_applications").at(-1)).toEqual({
            "election_event_id": EVENT_ID,
            "status@_ilike": "pending",
        })
        expect(canvas.queryByRole("progressbar")).toBeNull()
    },
}

export const ApprovedEnrollments: Story = {
    args: {storedStatus: "accepted"},
    parameters: {widgets: ["ApprovalsList", "PostCell"]},
    play: async ({canvasElement}) => {
        const bob = within(await row(canvasElement, "Bob Example"))
        await expect(bob.getByText("Approved by admin")).toBeVisible()
        await expect(bob.getByText("First name differs from the registry")).toBeVisible()
        await expect(bob.getByText("Approved")).toBeVisible()
        // Only an enrollment that waits says for how long.
        expect(bob.queryByText(/^Waiting/)).toBeNull()
        await noRow(canvasElement, "Alice Example")
        expect(listFilters("sequent_backend_applications").at(-1)).toEqual({
            "election_event_id": EVENT_ID,
            "status@_ilike": "accepted",
        })
    },
}

export const RejectedEnrollments: Story = {
    args: {storedStatus: "rejected"},
    play: async ({canvasElement}) => {
        const dan = within(await row(canvasElement, "Dan Nobody"))
        await expect(dan.getByText("Rejected automatically")).toBeVisible()
        await expect(dan.getByText("No voter found in the registry")).toBeVisible()
        // Without an area, the Post is the one the applicant named.
        await expect(dan.getByText("Paris")).toBeVisible()
        await expect(dan.getByText("Rejected")).toBeVisible()
    },
}

export const ChooseAStatus: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("combobox", {name: "Status"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Approved"}))
        await expect(await row(canvasElement, "Bob Example")).toBeVisible()
        await noRow(canvasElement, "Alice Example")
        expect(listFilters("sequent_backend_applications").at(-1)).toMatchObject({
            "status@_ilike": "accepted",
        })
        // The next visit starts from the status chosen last.
        expect(localStorage.getItem("approvals_status_filter")).toBe("accepted")
    },
}

export const SearchTheQueue: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Carol Sample")
        await userEvent.type(canvas.getByRole("textbox", {name: "Search"}), "alice")
        await noRow(canvasElement, "Carol Sample")
        await expect(await row(canvasElement, "Alice Example")).toBeVisible()
        expect(listFilters("sequent_backend_applications").at(-1)).toEqual({
            "election_event_id": EVENT_ID,
            "status@_ilike": "pending",
            "q": "alice",
        })
    },
}

export const NothingFound: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Carol Sample")
        await userEvent.type(canvas.getByRole("textbox", {name: "Search"}), "nobody at all")
        await expect(await canvas.findByText("Nothing here")).toBeVisible()
        await expect(
            canvas.getByText(
                "Enrollments with this status will appear here. Try another search or status."
            )
        ).toBeVisible()
    },
}

export const Empty: Story = {
    args: {empty: true},
    parameters: {widgets: ["ApprovalsList"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Nothing here")).toBeVisible()
        await expect(canvas.getByText("Approvals")).toBeVisible()
        expect(canvas.queryByRole("row")).toBeNull()
    },
}

export const WithoutEvent: Story = {
    args: {withEvent: false},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("progressbar", {name: "Approvals"})
        ).toBeVisible()
        expect(applicationsRead()).toEqual([])
    },
}

export const ReviewFromTheRowMenu: Story = {
    play: async ({canvasElement, args}) => {
        await rowAction(canvasElement, "Alice Example", "Review enrollment")
        expect(args.onViewApproval).toHaveBeenCalledTimes(1)
        expect(args.onViewApproval).toHaveBeenCalledWith(APPLICATION_ID)
        expect(args.onViewRule).not.toHaveBeenCalled()
        await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull())
    },
}

export const SeeTheRuleFromTheRowMenu: Story = {
    play: async ({canvasElement, args}) => {
        await rowAction(canvasElement, "Alice Example", "See the rule that decided")
        expect(args.onViewRule).toHaveBeenCalledTimes(1)
        expect(args.onViewRule).toHaveBeenCalledWith(APPLICATION_ID)
        // Using the menu doesn't open the row.
        expect(args.onViewApproval).not.toHaveBeenCalled()
        await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull())
    },
}

export const OpenADecidedEnrollment: Story = {
    args: {storedStatus: "accepted"},
    play: async ({canvasElement, args}) => {
        await userEvent.click(
            within(await row(canvasElement, "Bob Example")).getByRole("button", {name: "Actions"})
        )
        const menu = within(await within(document.body).findByRole("menu"))
        // A decided enrollment is opened, not reviewed.
        expect(menu.getAllByRole("menuitem").map((item) => item.textContent)).toEqual([
            "Open enrollment",
            "See the rule that decided",
        ])
        await userEvent.click(menu.getByRole("menuitem", {name: "Open enrollment"}))
        expect(args.onViewApproval).toHaveBeenCalledWith(SECOND_APPLICATION_ID)
        await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull())
    },
}

export const ClickARow: Story = {
    play: async ({canvasElement, args}) => {
        const carol = await row(canvasElement, "Carol Sample")
        await userEvent.click(within(carol).getByText("Carol Sample"))
        expect(args.onViewApproval).toHaveBeenCalledTimes(1)
        expect(args.onViewApproval).toHaveBeenCalledWith(storyId(9, 3))
    },
}

export const OpenTheApprovalMatrix: Story = {
    play: async ({canvasElement, args}) => {
        await row(canvasElement, "Alice Example")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Approval matrix"}))
        expect(args.onViewMatrix).toHaveBeenCalledTimes(1)
    },
}

export const ExportTheApplications: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await row(canvasElement, "Alice Example")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Export"}))
        const dialogElement = await within(document.body).findByRole("dialog")
        await waitFor(() => expect(dialogElement).toBeVisible())
        const dialog = within(dialogElement)
        await expect(dialog.getByText("Export applications")).toBeVisible()
        await userEvent.click(dialog.getByRole("button", {name: "Export"}))
        await waitFor(() =>
            expect(downloads).toEqual([
                {name: "export-applications.csv", href: documentUrl(EXPORT_DOCUMENT_ID)},
            ])
        )
        expect(graphqlCalls("ExportApplication")[0].variables).toEqual({
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
        })
        const message = await within(document.body).findByText(
            "Applications export finished successfully"
        )
        await waitFor(() => expect(message).toBeVisible())
        await waitFor(() => expect(dialogElement).not.toBeInTheDocument())
    },
}

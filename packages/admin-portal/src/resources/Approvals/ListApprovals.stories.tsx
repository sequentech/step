// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {localStorageStore, type GetListParams} from "react-admin"
import {useLocation, useNavigate} from "react-router-dom"
import {GraphQLError} from "graphql"
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
    ADMIN_ID,
    APPLICATION_ID,
    ApprovalsScreen,
    EVENT_PATH,
    EXPORT_DOCUMENT_ID,
    OTHER_ADMIN_ID,
    SECOND_APPLICATION_ID,
    graphqlCalls,
    listFilters,
    queueStoreKey,
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
    // The queue is a tab of the election event's page.
    parameters: {router: {initialEntries: [EVENT_PATH]}},
    beforeEach: async ({args}) => {
        await setUpApprovals(args, {
            FindApplicationsBySupportReference: ({variables}) => ({
                data: {
                    find_applications_by_support_reference: {
                        reference: variables.reference,
                        application_ids: [APPLICATION_ID],
                    },
                },
            }),
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

// ---------------------------------------------------------------------
// The filters each administrator remembers in each browser
// ---------------------------------------------------------------------

const APPLICATIONS = "sequent_backend_applications"
const OTHER_EVENT_ID = storyId(2, 9)
const OTHER_EVENT_PATH = `/sequent_backend_election_event/${OTHER_EVENT_ID}`
// Where the portal's react-admin store (`localStorageStore()`) keeps an administrator's queue.
const savedQueue = (pathname: string, userId = ADMIN_ID) =>
    `RaStore.${queueStoreKey(pathname, userId)}`
// The status filter the queue stored before it remembered every filter.
const LEGACY_STATUS_KEY = "approvals_status_filter"

/** Forgets everything the portal's store saved in this browser, so no story sees another's. */
const forgetSavedQueues = () => {
    for (const key of Object.keys(localStorage)) {
        if (key.startsWith("RaStore") || key === LEGACY_STATUS_KEY) localStorage.removeItem(key)
    }
}

/** The queue lists this browser stores, whoever they belong to (not the shared column choices). */
const storedQueues = () =>
    Object.keys(localStorage).filter(
        (key) => key.includes("approvals_queue") && !key.startsWith("RaStore.preferences.")
    )

/** One page load: a new store reads what this browser saved. */
function PageLoad({userId, ...args}: Scenario & {userId: string}) {
    const [store] = useState(() => localStorageStore())
    const {pathname} = useLocation()
    return (
        <ApprovalsScreen store={store} userId={userId}>
            <ListApprovals
                electionEventId={pathname === OTHER_EVENT_PATH ? OTHER_EVENT_ID : EVENT_ID}
                onViewApproval={args.onViewApproval}
                onViewMatrix={args.onViewMatrix}
                onViewRule={args.onViewRule}
                electionEventRecord={eventRecord() as Sequent_Backend_Election_Event}
            />
        </ApprovalsScreen>
    )
}

/**
 * The queue in a browser page that can be reloaded, moved to another event and
 * signed into by another administrator. `firstUser` is who is signed in when it
 * opens; empty while the profile loads.
 */
function Browser({firstUser = ADMIN_ID, ...args}: Scenario & {firstUser?: string}) {
    const [load, setLoad] = useState(0)
    const [userId, setUserId] = useState(firstUser)
    // Signing in loads the page again.
    const signIn = (admin: string) => {
        setUserId(admin)
        setLoad((count) => count + 1)
    }
    const navigate = useNavigate()
    const actions: [string, () => void][] = [
        ["Reload", () => setLoad((count) => count + 1)],
        ["Open the other event", () => navigate(OTHER_EVENT_PATH)],
        ["Open the first event", () => navigate(EVENT_PATH)],
        ["Sign in as the first administrator", () => signIn(ADMIN_ID)],
        ["Sign in as another administrator", () => signIn(OTHER_ADMIN_ID)],
        // The signed-in administrator becomes known while the page is open.
        ["The first administrator's profile loads", () => setUserId(ADMIN_ID)],
        ["Another administrator's profile loads", () => setUserId(OTHER_ADMIN_ID)],
    ]
    return (
        <>
            {actions.map(([name, act]) => (
                <button key={name} type="button" onClick={act}>
                    {name}
                </button>
            ))}
            <PageLoad key={load} userId={userId} {...args} />
        </>
    )
}

const remembered = {
    beforeEach: () => {
        forgetSavedQueues()
        return forgetSavedQueues
    },
    render: (args: Scenario) => <Browser {...args} />,
}

const applicationsReads = () => listFilters(APPLICATIONS).length

/** Clicks one of the browser's actions and waits until the queue has read its enrollments again. */
const browse = async (canvasElement: HTMLElement, action: string) => {
    const before = applicationsReads()
    await userEvent.click(within(canvasElement).getByRole("button", {name: action}))
    await waitFor(() => expect(applicationsReads()).toBeGreaterThan(before))
}
const reload = (canvasElement: HTMLElement) => browse(canvasElement, "Reload")

const chooseStatus = async (canvasElement: HTMLElement, status: string) => {
    await userEvent.click(within(canvasElement).getByRole("combobox", {name: "Status"}))
    await userEvent.click(await within(document.body).findByRole("option", {name: status}))
}

const lastRead = () => listFilters(APPLICATIONS).at(-1)

// React-admin's filter form sends what was typed after this pause.
const FILTER_FORM_PAUSE_MS = 500
/** Waits until a search typed before now would have been sent and saved. */
const afterTheFilterFormsPause = () =>
    new Promise((resolve) => setTimeout(resolve, 2 * FILTER_FORM_PAUSE_MS))

/** Checks that there were reads since the `from`-th one and that each used `filter`. */
const everyReadSince = (from: number, filter: Record<string, unknown>) => {
    const since = listFilters(APPLICATIONS).slice(from)
    expect(since).not.toHaveLength(0)
    for (const read of since) expect(read).toEqual(filter)
}

export const RememberTheFiltersAfterAReload: Story = {
    ...remembered,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        // A browser that saved nothing opens on the enrollments that wait.
        expect(lastRead()).toEqual({"election_event_id": EVENT_ID, "status@_ilike": "pending"})
        await chooseStatus(canvasElement, "Approved")
        await row(canvasElement, "Bob Example")
        await userEvent.type(canvas.getByRole("textbox", {name: "Search"}), "bob")
        const chosen = {"election_event_id": EVENT_ID, "status@_ilike": "accepted", "q": "bob"}
        await waitFor(() => expect(lastRead()).toEqual(chosen))

        await reload(canvasElement)
        await expect(await row(canvasElement, "Bob Example")).toBeVisible()
        expect(lastRead()).toEqual(chosen)
        expect(canvas.getByRole("combobox", {name: "Status"})).toHaveTextContent("Approved")
        expect(canvas.getByRole("textbox", {name: "Search"})).toHaveValue("bob")
    },
}

export const RememberAClearedStatus: Story = {
    ...remembered,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("button", {name: "Remove this filter"}))
        // Every enrollment of the event, whatever its status.
        await waitFor(() => expect(lastRead()).toEqual({election_event_id: EVENT_ID}))
        await row(canvasElement, "Bob Example")

        await reload(canvasElement)
        await row(canvasElement, "Bob Example")
        await row(canvasElement, "Alice Example")
        // Removing the status is remembered too: the queue doesn't go back to Needs review.
        expect(listFilters(APPLICATIONS).at(-1)).toEqual({election_event_id: EVENT_ID})
        expect(canvas.queryByRole("combobox", {name: "Status"})).toBeNull()
    },
}

export const EachEventKeepsItsOwnFilters: Story = {
    ...remembered,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        await chooseStatus(canvasElement, "Approved")
        await row(canvasElement, "Bob Example")

        await userEvent.click(canvas.getByRole("button", {name: "Open the other event"}))
        await waitFor(() => expect(lastRead()?.election_event_id).toBe(OTHER_EVENT_ID))
        // The other event has its own filters: none saved yet, so it opens on Needs review.
        await waitFor(() =>
            expect(canvas.getByRole("combobox", {name: "Status"})).toHaveTextContent("Needs review")
        )
        // Not even one read of the other event used the first event's filters.
        const otherReads = listFilters(APPLICATIONS).filter(
            (read) => read?.election_event_id === OTHER_EVENT_ID
        )
        expect(otherReads).not.toHaveLength(0)
        for (const read of otherReads) {
            expect(read).toEqual({"election_event_id": OTHER_EVENT_ID, "status@_ilike": "pending"})
        }

        // Back on the first event, every read uses its own filters.
        const back = applicationsReads()
        await userEvent.click(canvas.getByRole("button", {name: "Open the first event"}))
        await waitFor(() =>
            expect(lastRead()).toEqual({"election_event_id": EVENT_ID, "status@_ilike": "accepted"})
        )
        await expect(await row(canvasElement, "Bob Example")).toBeVisible()
        everyReadSince(back, {"election_event_id": EVENT_ID, "status@_ilike": "accepted"})
    },
}

/** Saves a list for a page, as the administrator left it. */
const saveQueue = (pathname: string, filter: Record<string, unknown>) =>
    localStorage.setItem(
        savedQueue(pathname),
        JSON.stringify({filter, sort: "created_at", order: "DESC", page: 1, perPage: 10})
    )

export const ReturnToAnEventSeenBefore: Story = {
    ...remembered,
    beforeEach: () => {
        forgetSavedQueues()
        saveQueue(EVENT_PATH, {status: "accepted"})
        saveQueue(OTHER_EVENT_PATH, {status: "rejected"})
        return forgetSavedQueues
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Bob Example")
        await userEvent.click(canvas.getByRole("button", {name: "Open the other event"}))
        await waitFor(() =>
            expect(lastRead()).toEqual({
                "election_event_id": OTHER_EVENT_ID,
                "status@_ilike": "rejected",
            })
        )
        // The queue stays on the page: its voter attributes are known already.
        const before = applicationsReads()
        await userEvent.click(canvas.getByRole("button", {name: "Open the first event"}))
        await row(canvasElement, "Bob Example")
        everyReadSince(before, {"election_event_id": EVENT_ID, "status@_ilike": "accepted"})
    },
}

export const LeaveWhileTypingASearch: Story = {
    ...remembered,
    beforeEach: () => {
        forgetSavedQueues()
        saveQueue(OTHER_EVENT_PATH, {status: "rejected"})
        return forgetSavedQueues
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        // Once both events' voter attributes are known, the queue stays on the page between them.
        await browse(canvasElement, "Open the other event")
        await browse(canvasElement, "Open the first event")
        await row(canvasElement, "Alice Example")
        // The search is sent after a pause; the administrator opens the other event first.
        await userEvent.type(canvas.getByRole("textbox", {name: "Search"}), "zz")
        await userEvent.click(canvas.getByRole("button", {name: "Open the other event"}))
        await afterTheFilterFormsPause()
        await waitFor(() =>
            expect(lastRead()).toEqual({
                "election_event_id": OTHER_EVENT_ID,
                "status@_ilike": "rejected",
            })
        )
        expect(JSON.parse(localStorage.getItem(savedQueue(OTHER_EVENT_PATH)) ?? "null")).toEqual({
            filter: {status: "rejected"},
            sort: "created_at",
            order: "DESC",
            page: 1,
            perPage: 10,
        })
    },
}

export const SavedFiltersNeverChooseTheScope: Story = {
    ...remembered,
    beforeEach: () => {
        forgetSavedQueues()
        localStorage.setItem(
            savedQueue(EVENT_PATH),
            JSON.stringify({
                filter: {
                    status: "accepted",
                    election_event_id: OTHER_EVENT_ID,
                    permission_label: "south",
                },
                sort: "created_at",
                order: "DESC",
            })
        )
        return forgetSavedQueues
    },
    play: async ({canvasElement}) => {
        await row(canvasElement, "Alice Example")
        // Filters that name another scope aren't the administrator's: the queue starts over.
        everyReadSince(0, {"election_event_id": EVENT_ID, "status@_ilike": "pending"})
        expect(localStorage.getItem(savedQueue(EVENT_PATH))).toBeNull()
    },
}

export const UnusableSavedFiltersStartOver: Story = {
    ...remembered,
    beforeEach: () => {
        forgetSavedQueues()
        // What the queue stored before it remembered every filter isn't read any more.
        localStorage.setItem(LEGACY_STATUS_KEY, "accepted")
        return forgetSavedQueues
    },
    play: async ({canvasElement}) => {
        const needsReview = {"election_event_id": EVENT_ID, "status@_ilike": "pending"}
        await row(canvasElement, "Alice Example")
        expect(lastRead()).toEqual(needsReview)
        const unusable = [
            "{not json",
            JSON.stringify({filter: "garbage", order: "DESC"}),
            JSON.stringify({filter: {status: "archived"}, order: "DESC"}),
            JSON.stringify({filter: {applicant_data: {retired: {_ilike: "x"}}}, order: "DESC"}),
            JSON.stringify({filter: {status: "accepted"}, displayedFilters: {retired: true}}),
        ]
        for (const saved of unusable) {
            localStorage.setItem(savedQueue(EVENT_PATH), saved)
            await reload(canvasElement)
            await row(canvasElement, "Alice Example")
            expect([saved, lastRead()]).toEqual([saved, needsReview])
            // The queue read and forgot exactly what was saved for it.
            expect([saved, localStorage.getItem(savedQueue(EVENT_PATH))]).toEqual([saved, null])
        }
    },
}

export const LookASavedSupportReferenceUpAgain: Story = {
    ...remembered,
    beforeEach: () => {
        forgetSavedQueues()
        // The applications the reference was resolved to when it was searched.
        localStorage.setItem(
            savedQueue(EVENT_PATH),
            JSON.stringify({
                filter: {
                    q: "Q5KW-8RFX",
                    support_reference: {reference: "Q5KW-8RFX", ids: [SECOND_APPLICATION_ID]},
                },
                displayedFilters: {},
                sort: "created_at",
                order: "DESC",
                page: 1,
                perPage: 10,
            })
        )
        return forgetSavedQueues
    },
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(graphqlCalls("FindApplicationsBySupportReference")).toHaveLength(1)
        )
        expect(graphqlCalls("FindApplicationsBySupportReference")[0].variables).toMatchObject({
            electionEventId: EVENT_ID,
            reference: "Q5KW-8RFX",
        })
        await waitFor(() =>
            expect(lastRead()).toEqual({
                election_event_id: EVENT_ID,
                q: "Q5KW-8RFX",
                support_reference: {reference: "Q5KW-8RFX", ids: [APPLICATION_ID]},
            })
        )
        // The saved resolution is never used: it may be out of date.
        expect(JSON.stringify(listFilters(APPLICATIONS))).not.toContain(SECOND_APPLICATION_ID)
        expect(within(canvasElement).getByRole("textbox", {name: "Search"})).toHaveValue(
            "Q5KW-8RFX"
        )
    },
}

export const EachAdministratorKeepsTheirOwnFilters: Story = {
    ...remembered,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const status = () => canvas.getByRole("combobox", {name: "Status"})
        const needsReview = {"election_event_id": EVENT_ID, "status@_ilike": "pending"}
        const approved = {"election_event_id": EVENT_ID, "status@_ilike": "accepted"}
        const rejected = {"election_event_id": EVENT_ID, "status@_ilike": "rejected"}
        await row(canvasElement, "Alice Example")
        await chooseStatus(canvasElement, "Approved")
        await waitFor(() => expect(lastRead()).toEqual(approved))
        await reload(canvasElement)
        await row(canvasElement, "Bob Example")
        expect(status()).toHaveTextContent("Approved")

        // Another administrator in this browser starts from their own first visit,
        // without a single read with the first administrator's filters.
        const beforeOther = applicationsReads()
        await browse(canvasElement, "Sign in as another administrator")
        await row(canvasElement, "Alice Example")
        everyReadSince(beforeOther, needsReview)
        expect(status()).toHaveTextContent("Needs review")
        await chooseStatus(canvasElement, "Rejected")
        await row(canvasElement, "Dan Nobody")
        await reload(canvasElement)
        await row(canvasElement, "Dan Nobody")
        expect(lastRead()).toEqual(rejected)

        // The first administrator gets their own choice back.
        const beforeFirst = applicationsReads()
        await browse(canvasElement, "Sign in as the first administrator")
        await row(canvasElement, "Bob Example")
        everyReadSince(beforeFirst, approved)
        expect(status()).toHaveTextContent("Approved")

        // A status they remove stays removed after a reload and a visit to another event.
        await userEvent.click(canvas.getByRole("button", {name: "Remove this filter"}))
        await waitFor(() => expect(lastRead()).toEqual({election_event_id: EVENT_ID}))
        await reload(canvasElement)
        await row(canvasElement, "Alice Example")
        expect(lastRead()).toEqual({election_event_id: EVENT_ID})
        await browse(canvasElement, "Open the other event")
        await browse(canvasElement, "Open the first event")
        await row(canvasElement, "Bob Example")
        expect(lastRead()).toEqual({election_event_id: EVENT_ID})
        expect(canvas.queryByRole("combobox", {name: "Status"})).toBeNull()

        // A sort by a column the administrator chose to show is remembered with the filters.
        await userEvent.click(canvas.getByRole("button", {name: "Columns"}))
        const idColumn = await within(document.body).findByRole("option", {
            name: /^\s*Application ID\s*$/,
        })
        await userEvent.click(within(idColumn).getByText("Application ID"))
        await userEvent.keyboard("{Escape}")
        const sortReads = reads("getList", APPLICATIONS).length
        await userEvent.click(
            await canvas.findByRole("button", {name: /^Sort by application ID\b/})
        )
        await waitFor(() =>
            expect(reads("getList", APPLICATIONS).length).toBeGreaterThan(sortReads)
        )
        await reload(canvasElement)
        await row(canvasElement, "Bob Example")
        const sorted = reads("getList", APPLICATIONS).at(-1)?.args[1] as GetListParams
        expect(sorted.filter).toEqual({election_event_id: EVENT_ID})
        expect(sorted.sort?.field).toBe("id")

        // The other administrator's choice is untouched.
        await browse(canvasElement, "Sign in as another administrator")
        await row(canvasElement, "Dan Nobody")
        expect(lastRead()).toEqual(rejected)
    },
}

export const NothingIsRememberedBeforeTheAdministratorIsKnown: Story = {
    ...remembered,
    beforeEach: () => {
        forgetSavedQueues()
        localStorage.setItem(
            savedQueue(EVENT_PATH),
            JSON.stringify({filter: {status: "accepted"}, order: "DESC"})
        )
        return forgetSavedQueues
    },
    // The administrator's profile is still loading.
    render: (args: Scenario) => <Browser {...args} firstUser="" />,
    play: async ({canvasElement}) => {
        await row(canvasElement, "Alice Example")
        // The queue still filters, from Needs review: nobody's saved filters are read.
        everyReadSince(0, {"election_event_id": EVENT_ID, "status@_ilike": "pending"})
        await chooseStatus(canvasElement, "Rejected")
        await row(canvasElement, "Dan Nobody")
        // And nothing is written for an administrator nobody knows yet.
        expect(storedQueues()).toEqual([savedQueue(EVENT_PATH)])
        expect(JSON.parse(localStorage.getItem(savedQueue(EVENT_PATH)) ?? "null")).toEqual({
            filter: {status: "accepted"},
            order: "DESC",
        })

        await browse(canvasElement, "Sign in as the first administrator")
        await row(canvasElement, "Bob Example")
        expect(lastRead()).toEqual({"election_event_id": EVENT_ID, "status@_ilike": "accepted"})
    },
}

export const SavedFiltersWaitForTheProfileFields: Story = {
    ...remembered,
    beforeEach: async ({args}) => {
        forgetSavedQueues()
        // The voter attributes, whose filters a saved list may use, can't be read.
        await setUpApprovals(args, {
            getUserProfileAttributes: () => ({
                errors: [new GraphQLError("Attributes unavailable")],
            }),
        })
        localStorage.setItem(
            savedQueue(EVENT_PATH),
            JSON.stringify({
                filter: {status: "accepted", applicant_data: {embassy: {_ilike: "Paris"}}},
                order: "DESC",
            })
        )
        return forgetSavedQueues
    },
    play: async ({canvasElement}) => {
        await row(canvasElement, "Alice Example")
        // The queue still filters, from Needs review, without the saved list it can't check...
        everyReadSince(0, {"election_event_id": EVENT_ID, "status@_ilike": "pending"})
        await chooseStatus(canvasElement, "Rejected")
        await row(canvasElement, "Dan Nobody")
        // ...which stays as it was for when the attributes can be read again.
        expect(JSON.parse(localStorage.getItem(savedQueue(EVENT_PATH)) ?? "null")).toEqual({
            filter: {status: "accepted", applicant_data: {embassy: {_ilike: "Paris"}}},
            order: "DESC",
        })
    },
}

export const RememberAddedFilters: Story = {
    ...remembered,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const addFilter = async (name: string) => {
            await userEvent.click(canvas.getByRole("button", {name: "Add filter"}))
            await userEvent.click(
                await within(document.body).findByRole("menuitemcheckbox", {name})
            )
        }
        await row(canvasElement, "Alice Example")
        await addFilter("Embassy")
        await userEvent.type(await canvas.findByRole("textbox", {name: "Embassy"}), "Oslo")
        await addFilter("Verification")
        await userEvent.click(await canvas.findByRole("combobox", {name: "Verification"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Manual"}))
        // Shown without a value yet.
        await addFilter("Applicant ID")
        const chosen = {
            "election_event_id": EVENT_ID,
            "status@_ilike": "pending",
            "verification_type@_ilike": "MANUAL",
            "applicant_data": {embassy: {_ilike: "Oslo"}},
        }
        await waitFor(() => expect(lastRead()).toEqual(chosen))

        await reload(canvasElement)
        await waitFor(() => expect(lastRead()).toEqual(chosen))
        expect(canvas.getByRole("textbox", {name: "Embassy"})).toHaveValue("Oslo")
        expect(canvas.getByRole("combobox", {name: "Verification"})).toHaveTextContent("Manual")
        expect(canvas.getByRole("textbox", {name: "Applicant ID"})).toHaveValue("")
    },
}

export const TheAdministratorBecomesKnownOnTheOpenPage: Story = {
    ...remembered,
    beforeEach: () => {
        forgetSavedQueues()
        localStorage.setItem(
            savedQueue(EVENT_PATH),
            JSON.stringify({filter: {status: "accepted"}, order: "DESC"})
        )
        // The other administrator's saved search, with the applications it was resolved to then.
        localStorage.setItem(
            savedQueue(EVENT_PATH, OTHER_ADMIN_ID),
            JSON.stringify({
                filter: {
                    q: "Q5KW-8RFX",
                    support_reference: {reference: "Q5KW-8RFX", ids: [SECOND_APPLICATION_ID]},
                },
                order: "DESC",
            })
        )
        return forgetSavedQueues
    },
    render: (args: Scenario) => <Browser {...args} firstUser="" />,
    play: async ({canvasElement}) => {
        await row(canvasElement, "Alice Example")
        await browse(canvasElement, "The first administrator's profile loads")
        await waitFor(() =>
            expect(lastRead()).toEqual({"election_event_id": EVENT_ID, "status@_ilike": "accepted"})
        )
        await row(canvasElement, "Bob Example")

        // Without a reload, not a single read with the first administrator's filters, not even
        // a search still being typed, and the other administrator's saved list is checked too:
        // its search is looked up again.
        const before = applicationsReads()
        await userEvent.type(within(canvasElement).getByRole("textbox", {name: "Search"}), "zz")
        await browse(canvasElement, "Another administrator's profile loads")
        await afterTheFilterFormsPause()
        const fresh = {
            election_event_id: EVENT_ID,
            q: "Q5KW-8RFX",
            support_reference: {reference: "Q5KW-8RFX", ids: [APPLICATION_ID]},
        }
        await waitFor(() => expect(lastRead()).toEqual(fresh))
        // Before the lookup answers, the search alone; never the saved applications. On a slow
        // machine the first administrator's own search may also be read; the last read and the
        // saved list below show it never reaches the other administrator.
        const looking = {election_event_id: EVENT_ID, q: "Q5KW-8RFX"}
        const typedByTheFirst = {
            "election_event_id": EVENT_ID,
            "status@_ilike": "accepted",
            "q": "zz",
        }
        for (const read of listFilters(APPLICATIONS).slice(before)) {
            expect([typedByTheFirst, looking, fresh]).toContainEqual(read)
        }
        expect(
            JSON.parse(localStorage.getItem(savedQueue(EVENT_PATH, OTHER_ADMIN_ID)) ?? "null")
                .filter
        ).toEqual({
            q: "Q5KW-8RFX",
            support_reference: {reference: "Q5KW-8RFX", ids: [APPLICATION_ID]},
        })
    },
}

let answerTheLookup: () => void = () => {}

export const ChangeAFilterWhileASavedReferenceIsLookedUp: Story = {
    ...remembered,
    beforeEach: async ({args}) => {
        forgetSavedQueues()
        // The server answers when the story says so.
        await setUpApprovals(args, {
            FindApplicationsBySupportReference: async ({variables}) => {
                await new Promise<void>((resolve) => (answerTheLookup = resolve))
                return {
                    data: {
                        find_applications_by_support_reference: {
                            reference: variables.reference,
                            application_ids: [APPLICATION_ID],
                        },
                    },
                }
            },
        })
        saveQueue(EVENT_PATH, {q: "Q5KW-8RFX", status: "pending"})
        return forgetSavedQueues
    },
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(graphqlCalls("FindApplicationsBySupportReference")).toHaveLength(1)
        )
        await chooseStatus(canvasElement, "Approved")
        await waitFor(() =>
            expect(lastRead()).toEqual({
                "election_event_id": EVENT_ID,
                "status@_ilike": "accepted",
                "q": "Q5KW-8RFX",
            })
        )
        answerTheLookup()
        // The answer keeps the status chosen meanwhile.
        await waitFor(() =>
            expect(lastRead()).toEqual({
                "election_event_id": EVENT_ID,
                "status@_ilike": "accepted",
                "q": "Q5KW-8RFX",
                "support_reference": {reference: "Q5KW-8RFX", ids: [APPLICATION_ID]},
            })
        )
        expect(within(canvasElement).getByRole("combobox", {name: "Status"})).toHaveTextContent(
            "Approved"
        )
        expect(
            JSON.parse(localStorage.getItem(savedQueue(EVENT_PATH)) ?? "null").filter
        ).toMatchObject({q: "Q5KW-8RFX", status: "accepted"})
    },
}

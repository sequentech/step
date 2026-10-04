// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, type DataProvider} from "react-admin"
import {i18n} from "@sequentech/ui-core"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, electionRecord, eventRecord, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionEventTallyContextProvider} from "@/providers/ElectionEventTallyProvider"
import {IApplicationsStatus} from "@/types/applications"
import {
    APPLICATION_ID,
    APPROVAL_ATTRIBUTES,
    SECOND_APPLICATION_ID,
    applicationRecord,
} from "@/resources/Approvals/__stories__/ApprovalsFixture"
import {matrixHandlers} from "@/resources/Approvals/__stories__/ApprovalMatrixFixture"
import {EditElectionEventApprovals} from "./EditElectionEventApprovals"
import {answerOrPending, paramsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const APPLICATIONS = "sequent_backend_applications"
const STATUS_FILTER_KEY = "approvals_status_filter"

interface Scenario {
    /** The election whose approvals are shown, or the whole event's. */
    electionId?: string
    /** The status filter the Approvals tab stores when it is chosen. */
    storedStatus?: string
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>

/** As ra-data-hasura does for String columns, the status filter matches case-insensitively. */
function hasuraStatusFilter(provider: DataProvider): DataProvider {
    return {
        ...provider,
        getList: (resource, {filter, ...params}) => {
            const {status, ...rest} = filter ?? {}
            return provider.getList(resource, {
                ...params,
                filter: status ? {...rest, "status@_ilike": status} : rest,
            })
        },
    }
}

/** The event tabs pass a new `showList` each time the Approvals tab is chosen again. */
function Fixture({electionId}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const [showList, setShowList] = useState<string>()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={hasuraStatusFilter(data.provider)}
            role={permissions}
            tenant={tenant}
        >
            <ElectionEventTallyContextProvider>
                <button type="button" onClick={() => setShowList(String(Date.now()))}>
                    Approvals tab
                </button>
                <RecordContextProvider value={eventRecord()}>
                    <EditElectionEventApprovals
                        electionEventId={EVENT_ID}
                        electionId={electionId}
                        showList={showList}
                    />
                </RecordContextProvider>
            </ElectionEventTallyContextProvider>
        </AdminStoryProvider>
    )
}

// The matching voters of an application have their own section; their reads stay loading here.
const meta = {
    title: "Admin/Election event/EditElectionEventApprovals",
    component: EditElectionEventApprovals,
    args: {},
    parameters: {
        expectedFailure: {
            reason: "The row's view action is an icon button without an accessible name and the status chips have white labels on the warning and success colours.",
            a11y: ["button-name", "color-contrast"],
        },
    },
    beforeEach: async ({args}) => {
        if (args.storedStatus) localStorage.setItem(STATUS_FILTER_KEY, args.storedStatus)
        else localStorage.removeItem(STATUS_FILTER_KEY)
        data = recordsOrPending({
            [APPLICATIONS]: [
                applicationRecord(),
                applicationRecord({
                    id: SECOND_APPLICATION_ID,
                    applicant_id: "applicant-0002",
                    status: IApplicationsStatus.ACCEPTED,
                }),
                applicationRecord({
                    id: storyId(9, 3),
                    applicant_id: "applicant-0003",
                    permission_label: "north",
                }),
            ],
            sequent_backend_election: [electionRecord(undefined, {permission_label: "north"})],
        })
        graphql = graphqlBoundary(
            answerOrPending({
                ...matrixHandlers(),
                getUserProfileAttributes: () => ({
                    data: {get_user_profile_attributes: APPROVAL_ATTRIBUTES},
                }),
            }),
            {schema: true}
        )
        await graphql.ready
        return () => localStorage.removeItem(STATUS_FILTER_KEY)
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const pendingRow = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("row", {name: /applicant-0001/})

async function viewPendingApplication(canvasElement: HTMLElement) {
    const row = await pendingRow(canvasElement)
    const buttons = within(row).getAllByRole("button")
    await userEvent.click(buttons[buttons.length - 1])
    const details = await within(canvasElement).findByRole("table", {
        name: "approvals details table",
    })
    await expect(within(details).getByText("alice@example.test")).toBeVisible()
    await waitFor(() =>
        expect(paramsOf(data, "getOne", APPLICATIONS)).toMatchObject({id: APPLICATION_ID})
    )
}

const detailsGone = (canvasElement: HTMLElement) =>
    waitFor(() =>
        expect(
            within(canvasElement).queryByRole("table", {name: "approvals details table"})
        ).toBeNull()
    )

const lastListFilter = () =>
    data.calls.filter(({method, args}) => method === "getList" && args[0] === APPLICATIONS).at(-1)
        ?.args[1]

export const Populated: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await pendingRow(canvasElement)).toBeVisible()
        // Without a stored status filter, the list keeps its pending default.
        await waitFor(() =>
            expect(lastListFilter()).toMatchObject({
                filter: {"election_event_id": EVENT_ID, "status@_ilike": "pending"},
                sort: {field: "created_at", order: "DESC"},
            })
        )
        expect(canvas.queryByRole("row", {name: /applicant-0002/})).toBeNull()
        expect(graphql.calls[0]).toMatchObject({
            name: "getUserProfileAttributes",
            variables: {electionEventId: EVENT_ID},
        })
        expect(data.calls.map(({args}) => args[0])).not.toContain("sequent_backend_election")
    },
}

export const PendingFilterFromTheTab: Story = {
    args: {storedStatus: "pending"},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await expect(await pendingRow(canvasElement)).toBeVisible()
        await waitFor(() =>
            expect(lastListFilter()).toMatchObject({filter: {"status@_ilike": "pending"}})
        )
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /applicant-0002/})).toBeNull()
        )
    },
}

export const ElectionApprovals: Story = {
    args: {electionId: STORY_IDS.election},
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        // The election's permission label narrows the applications.
        await waitFor(() =>
            expect(lastListFilter()).toMatchObject({
                filter: {election_event_id: EVENT_ID, permission_label: "north"},
            })
        )
        expect(paramsOf(data, "getOne", "sequent_backend_election")).toMatchObject({
            id: STORY_IDS.election,
        })
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("row", {name: /applicant-0003/})).toBeVisible()
        await waitFor(() => expect(canvas.queryByRole("row", {name: /applicant-0001/})).toBeNull())
    },
}

export const OpenTheApprovalMatrixAndGoBack: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await pendingRow(canvasElement)).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Approval Matrix"}))
        await expect(await canvas.findByRole("table", {name: "Rules"})).toBeVisible()
        await expect(canvas.getByRole("button", {name: "Add Rule"})).toBeVisible()
        expect(graphql.calls.find(({name}) => name === "GetApprovalMatrix")).toMatchObject({
            variables: {electionEventId: EVENT_ID},
        })
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.back")}))
        await expect(await pendingRow(canvasElement)).toBeVisible()
        expect(canvas.queryByRole("table", {name: "Rules"})).toBeNull()
    },
}

export const ApprovalMatrixWithoutThePermissionToSave: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    parameters: {
        expectedFailure: {
            reason: "The status chips have white labels on the warning and success colours.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Approval Matrix"}))
        await expect(await canvas.findByRole("table", {name: "Rules"})).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Add Rule"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Save"})).toBeNull()
    },
}

export const ViewAnApplicationAndGoBack: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await viewPendingApplication(canvasElement)
        const canvas = within(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: i18n.t("common.label.back")}))
        await detailsGone(canvasElement)
        await expect(await pendingRow(canvasElement)).toBeVisible()
    },
}

export const ChoosingTheTabAgainShowsTheList: Story = {
    globals: {permissions: EStoryPermissions.ADMIN},
    play: async ({canvasElement}) => {
        await viewPendingApplication(canvasElement)
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Approvals tab"}))
        await detailsGone(canvasElement)
        await expect(await pendingRow(canvasElement)).toBeVisible()
    },
}

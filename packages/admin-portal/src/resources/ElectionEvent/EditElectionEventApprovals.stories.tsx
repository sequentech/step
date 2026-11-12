// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {type PropsWithChildren, useContext, useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider, memoryStore, type Store} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, areaRecords, electionRecord, eventRecord, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionEventTallyContextProvider} from "@/providers/ElectionEventTallyProvider"
import {TenantContext} from "@/providers/TenantContextProvider"
import {
    APPLICATION_ID,
    MANUAL_APPLICATION_ID,
    APPROVAL_ATTRIBUTES,
    REGISTRY_VOTERS,
    applicationRecord,
    applications,
} from "@/resources/Approvals/__stories__/ApprovalsFixture"
import {matrixHandlers} from "@/resources/Approvals/__stories__/ApprovalMatrixFixture"
import {
    ADMIN_ID,
    EVENT_PATH,
    queueStoreKey,
    withServiceFilters,
} from "@/resources/Approvals/__stories__/ApprovalsScreenFixture"
import {EditElectionEventApprovals} from "./EditElectionEventApprovals"
import {answerOrPending, paramsOf, recordsOrPending} from "./__stories__/ElectionEventFixture"
import {EStoryPermissions, useStoryGlobals} from "../../../../ui-essentials/.storybook/globals"

const APPLICATIONS = "sequent_backend_applications"

interface Scenario {
    /** The election whose approvals are shown, or the whole event's. */
    electionId?: string
    /** The status filter the administrator last chose on this browser. */
    storedStatus?: string
}

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof recordsOrPending>
let preferences: Store

const OTHER_EVENT_ID = storyId(1, 9)
const OTHER_TENANT_ID = storyId(0, 9)

/**
 * The selected tenant, as TenantContextProvider keeps it: choosing another tenant changes the
 * context without remounting the page below it.
 */
function SelectedTenant({children}: PropsWithChildren) {
    const context = useContext(TenantContext)
    const [tenantId, setTenantId] = useState(context.tenantId)
    return (
        <TenantContext.Provider value={{...context, tenantId, setTenantId}}>
            <button type="button" onClick={() => setTenantId(OTHER_TENANT_ID)}>
                Other tenant
            </button>
            {children}
        </TenantContext.Provider>
    )
}

/** The event tabs pass a new `showList` each time the Approvals tab is chosen again. */
function Fixture({electionId}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const [showList, setShowList] = useState<string>()
    const [eventId, setEventId] = useState(EVENT_ID)
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={withServiceFilters(data.provider)}
            role={permissions}
            tenant={tenant}
            store={preferences}
            auth={{userId: ADMIN_ID}}
        >
            <ElectionEventTallyContextProvider>
                <button type="button" onClick={() => setShowList(String(Date.now()))}>
                    Approvals tab
                </button>
                <button type="button" onClick={() => setEventId(OTHER_EVENT_ID)}>
                    Other event
                </button>
                <SelectedTenant>
                    <RecordContextProvider value={eventRecord()}>
                        <EditElectionEventApprovals
                            electionEventId={eventId}
                            electionId={electionId}
                            showList={showList}
                        />
                    </RecordContextProvider>
                </SelectedTenant>
            </ElectionEventTallyContextProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Election event/EditElectionEventApprovals",
    component: EditElectionEventApprovals,
    args: {},
    globals: {permissions: EStoryPermissions.ADMIN},
    // The Approvals tab of the election event's page.
    parameters: {router: {initialEntries: [EVENT_PATH]}},
    beforeEach: async ({args}) => {
        preferences = memoryStore(
            args.storedStatus
                ? {[queueStoreKey(EVENT_PATH)]: {filter: {status: args.storedStatus}}}
                : {}
        )
        data = recordsOrPending({
            [APPLICATIONS]: [
                ...applications(),
                // Erin enrolled for the election of the north district.
                applicationRecord({
                    id: storyId(9, 5),
                    applicant_id: "applicant-0005",
                    applicant_data: {
                        firstName: "Erin",
                        lastName: "North",
                        email: "erin@example.test",
                        dateOfBirth: "1982-03-09",
                        embassy: "Oslo",
                    },
                    permission_label: "north",
                    created_at: "2026-01-10T10:00:00.000Z",
                }),
            ],
            sequent_backend_election: [electionRecord(undefined, {permission_label: "north"})],
            sequent_backend_area: areaRecords(),
            user: REGISTRY_VOTERS,
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
    },
    render: (args, {globals}) => <Fixture key={JSON.stringify(globals)} {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const row = (canvasElement: HTMLElement, applicant: string) =>
    within(canvasElement).findByRole("row", {name: new RegExp(applicant)})

/** Opens the menu of a row of the queue and chooses one of its actions. */
const rowAction = async (canvasElement: HTMLElement, applicant: string, action: string) => {
    await userEvent.click(
        within(await row(canvasElement, applicant)).getByRole("button", {name: "Actions"})
    )
    await userEvent.click(await within(document.body).findByRole("menuitem", {name: action}))
}

/** The review of an enrollment, once it has loaded. */
const review = (canvasElement: HTMLElement, applicant: string) =>
    within(canvasElement).findByRole("heading", {name: applicant, level: 2})

const noReview = (canvasElement: HTMLElement) =>
    waitFor(() => expect(within(canvasElement).queryByRole("heading", {level: 2})).toBeNull())

/** The rule cards of the approval matrix, in order. */
const ruleCards = async (canvasElement: HTMLElement) =>
    within(await within(canvasElement).findByRole("list", {name: "Rules"})).getAllByRole("listitem")

const lastListFilter = () =>
    data.calls.filter(({method, args}) => method === "getList" && args[0] === APPLICATIONS).at(-1)
        ?.args[1]

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await row(canvasElement, "Alice Example")).toBeVisible()
        await expect(await row(canvasElement, "Carol Sample")).toBeVisible()
        // Without a stored status filter, the queue shows the enrollments that wait.
        await waitFor(() =>
            expect(lastListFilter()).toMatchObject({
                filter: {"election_event_id": EVENT_ID, "status@_ilike": "pending"},
                sort: {field: "created_at", order: "DESC"},
            })
        )
        expect(canvas.queryByRole("row", {name: /Bob Example/})).toBeNull()
        expect(graphql.calls[0]).toMatchObject({
            name: "getUserProfileAttributes",
            variables: {electionEventId: EVENT_ID},
        })
        expect(data.calls.map(({args}) => args[0])).not.toContain("sequent_backend_election")
    },
}

export const StatusChosenLast: Story = {
    args: {storedStatus: "accepted"},
    play: async ({canvasElement}) => {
        await expect(await row(canvasElement, "Bob Example")).toBeVisible()
        await waitFor(() =>
            expect(lastListFilter()).toMatchObject({filter: {"status@_ilike": "accepted"}})
        )
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("row", {name: /Alice Example/})).toBeNull()
        )
    },
}

export const ElectionApprovals: Story = {
    args: {electionId: STORY_IDS.election},
    play: async ({canvasElement}) => {
        // The election's permission label narrows the enrollments.
        await waitFor(() =>
            expect(lastListFilter()).toMatchObject({
                filter: {election_event_id: EVENT_ID, permission_label: "north"},
            })
        )
        expect(paramsOf(data, "getOne", "sequent_backend_election")).toMatchObject({
            id: STORY_IDS.election,
        })
        const canvas = within(canvasElement)
        await expect(await row(canvasElement, "Erin North")).toBeVisible()
        await waitFor(() => expect(canvas.queryByRole("row", {name: /Alice Example/})).toBeNull())
    },
}

const ELECTION_PATH = `/sequent_backend_election/${STORY_IDS.election}`

export const EachElectionKeepsItsOwnFilters: Story = {
    // The event's queue was left on Approved; the election's page has filters of its own.
    args: {electionId: STORY_IDS.election, storedStatus: "accepted"},
    parameters: {router: {initialEntries: [ELECTION_PATH]}},
    play: async ({canvasElement}) => {
        await row(canvasElement, "Erin North")
        expect(lastListFilter()).toMatchObject({
            filter: {
                "election_event_id": EVENT_ID,
                "permission_label": "north",
                "status@_ilike": "pending",
            },
        })
        await userEvent.click(within(canvasElement).getByRole("combobox", {name: "Status"}))
        await userEvent.click(await within(document.body).findByRole("option", {name: "Rejected"}))
        await waitFor(() =>
            expect(preferences.getItem(queueStoreKey(ELECTION_PATH))).toMatchObject({
                filter: {status: "rejected"},
            })
        )
        // The election's label stays the page's own, not a saved filter.
        expect(preferences.getItem(queueStoreKey(ELECTION_PATH)).filter).toEqual({
            status: "rejected",
        })
        await waitFor(() =>
            expect(lastListFilter()).toMatchObject({
                filter: {"permission_label": "north", "status@_ilike": "rejected"},
            })
        )
        expect(preferences.getItem(queueStoreKey(EVENT_PATH))).toEqual({
            filter: {status: "accepted"},
        })
    },
}

export const OpenTheApprovalMatrixAndGoBack: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("button", {name: "Approval matrix"}))
        expect(await ruleCards(canvasElement)).toHaveLength(8)
        await expect(canvas.getByRole("button", {name: "Add rule"})).toBeVisible()
        // Opened from the toolbar, the example is the sample and there is no enrollment to go back to.
        await expect(await canvas.findByText("Sample enrollment")).toBeVisible()
        await expect(canvas.getByRole("region", {name: "Try an example"})).toBeVisible()
        expect(canvas.queryByRole("button", {name: /^Back to the enrollment/})).toBeNull()
        expect(graphql.calls.find(({name}) => name === "GetApprovalMatrix")).toMatchObject({
            variables: {electionEventId: EVENT_ID},
        })
        await userEvent.click(canvas.getByRole("button", {name: "Approvals"}))
        await expect(await row(canvasElement, "Alice Example")).toBeVisible()
        expect(canvas.queryByRole("list", {name: "Rules"})).toBeNull()
    },
}

export const ApprovalMatrixWithoutThePermissionToSave: Story = {
    globals: {permissions: EStoryPermissions.ADMIN_LIGHT},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await userEvent.click(await canvas.findByRole("button", {name: "Approval matrix"}))
        expect(await ruleCards(canvasElement)).toHaveLength(8)
        await expect(canvas.getByText("View only")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Add rule"})).toBeNull()
        expect(canvas.queryByRole("button", {name: /^Edit rule|^Edit the last rule/})).toBeNull()
        // Trying an example needs no permission to change the rules.
        await expect(canvas.getByRole("button", {name: "Edit example"})).toBeVisible()
    },
}

export const ReviewAnEnrollmentAndGoBack: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Alice Example", "Review enrollment")
        await expect(await review(canvasElement, "Alice Example")).toBeVisible()
        await waitFor(() =>
            expect(paramsOf(data, "getOne", APPLICATIONS)).toMatchObject({id: APPLICATION_ID})
        )
        await expect(canvas.getByText("Why this needs a person")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Back to Approvals"}))
        await noReview(canvasElement)
        await expect(await row(canvasElement, "Alice Example")).toBeVisible()
    },
}

export const ClickARowToReviewIt: Story = {
    play: async ({canvasElement}) => {
        const carol = await row(canvasElement, "Carol Sample")
        await userEvent.click(within(carol).getByText("Carol Sample"))
        await expect(await review(canvasElement, "Carol Sample")).toBeVisible()
        await expect(
            within(canvasElement).getByText("Check them face to face before approving")
        ).toBeVisible()
    },
}

/** The highlighted rule cards' positions, `otherwise` for the last rule. */
const highlighted = (canvasElement: HTMLElement) =>
    Array.from(canvasElement.querySelectorAll("li[data-highlighted='true']")).map((card) =>
        card.id.replace("approval-matrix-rule-", "")
    )

export const SeeTheRuleFromAReviewAndGoBack: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Alice Example", "Review enrollment")
        await review(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("button", {name: "See the rule"}))

        // The example is Alice's recorded enrollment; only the rule deciding it is highlighted.
        await ruleCards(canvasElement)
        await expect(
            await canvas.findByText("Simulating the enrollment of Alice Example")
        ).toBeVisible()
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["5"]))
        // Its way back leads to Alice's enrollment.
        await userEvent.click(
            canvas.getByRole("button", {name: "Back to the enrollment of Alice Example"})
        )
        await expect(await review(canvasElement, "Alice Example")).toBeVisible()
        expect(canvas.queryByRole("list", {name: "Rules"})).toBeNull()
    },
}

export const SeeTheRuleFromTheQueueAndGoBack: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Carol Sample", "See the rule that decided")
        await ruleCards(canvasElement)
        await expect(
            await canvas.findByText("Simulating the enrollment of Carol Sample")
        ).toBeVisible()
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["2"]))
        // The matrix doesn't read the enrollment again: the queue's record is used.
        expect(paramsOf(data, "getOne", APPLICATIONS)).toBeUndefined()
        // The button at the top of the example returns to Carol's enrollment, not the queue.
        await userEvent.click(
            canvas.getByRole("button", {name: "Back to the enrollment of Carol Sample"})
        )
        await expect(await review(canvasElement, "Carol Sample")).toBeVisible()
        await waitFor(() =>
            expect(paramsOf(data, "getOne", APPLICATIONS)).toMatchObject({
                id: MANUAL_APPLICATION_ID,
            })
        )
    },
}

export const SeeTheRuleFromTheQueueAndGoToTheQueue: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Carol Sample", "See the rule that decided")
        await ruleCards(canvasElement)
        await userEvent.click(canvas.getByRole("button", {name: "Approvals"}))
        await expect(await row(canvasElement, "Carol Sample")).toBeVisible()
        await noReview(canvasElement)
        // The toolbar opens the sample: nothing is left of Carol's enrollment.
        await userEvent.click(canvas.getByRole("button", {name: "Approval matrix"}))
        await expect(await canvas.findByText("Sample enrollment")).toBeVisible()
        await expect(canvas.getByRole("region", {name: "Try an example"})).toBeVisible()
        expect(canvas.queryByText(/Carol Sample/)).toBeNull()
    },
}

export const ChoosingTheTabForgetsTheEnrollment: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Carol Sample", "See the rule that decided")
        await canvas.findByText("Simulating the enrollment of Carol Sample")
        await userEvent.click(canvas.getByRole("button", {name: "Approvals tab"}))
        await expect(await row(canvasElement, "Carol Sample")).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Approval matrix"}))
        await expect(await canvas.findByText("Sample enrollment")).toBeVisible()
        await expect(canvas.getByRole("region", {name: "Try an example"})).toBeVisible()
        expect(canvas.queryByText(/Carol Sample/)).toBeNull()
    },
}

/** Switching the event or the tenant never shows or decides the enrollment of the previous one. */
const switchWhileOpen = (button: string): Story => ({
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Carol Sample", "See the rule that decided")
        await canvas.findByText("Simulating the enrollment of Carol Sample")
        await waitFor(() => expect(highlighted(canvasElement)).toEqual(["2"]))
        const before = graphql.calls.filter(({name}) => name === "EvaluateApprovalMatrix").length
        const reads = graphql.calls.filter(({name}) => name === "GetApprovalMatrix").length
        let shownAfter = false
        const observer = new MutationObserver(() => {
            shownAfter ||=
                canvasElement.textContent?.includes("Simulating the enrollment of Carol Sample") ??
                false
        })
        observer.observe(canvasElement, {subtree: true, childList: true, characterData: true})
        await userEvent.click(canvas.getByRole("button", {name: button}))
        // In the same render the matrix and Carol's enrollment are gone.
        expect(canvas.queryByRole("list", {name: "Rules"})).toBeNull()
        expect(canvas.queryByText("Simulating the enrollment of Carol Sample")).toBeNull()
        await waitFor(() => expect(canvas.queryByRole("progressbar")).toBeNull())
        observer.disconnect()
        expect(shownAfter).toBe(false)
        expect(
            graphql.calls.filter(({name}) => name === "EvaluateApprovalMatrix").slice(before)
        ).toEqual([])
        // No matrix was mounted for the new context: a render of it would read the matrix again.
        expect(graphql.calls.filter(({name}) => name === "GetApprovalMatrix")).toHaveLength(reads)
    },
})

export const SwitchingTheEventClosesTheEnrollmentsMatrix: Story = switchWhileOpen("Other event")
export const SwitchingTheTenantClosesTheEnrollmentsMatrix: Story = switchWhileOpen("Other tenant")

export const ChoosingTheTabAgainShowsTheQueue: Story = {
    play: async ({canvasElement}) => {
        await rowAction(canvasElement, "Alice Example", "Review enrollment")
        await review(canvasElement, "Alice Example")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Approvals tab"}))
        await noReview(canvasElement)
        await expect(await row(canvasElement, "Alice Example")).toBeVisible()
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {RecordContextProvider} from "react-admin"
import {AdminStoryProvider, EVENT_ID, graphqlBoundary} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS, areaRecords, electionRecord, eventRecord, storyId} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {ElectionEventTallyContextProvider} from "@/providers/ElectionEventTallyProvider"
import {
    APPLICATION_ID,
    APPROVAL_ATTRIBUTES,
    REGISTRY_VOTERS,
    applicationRecord,
    applications,
} from "@/resources/Approvals/__stories__/ApprovalsFixture"
import {matrixHandlers} from "@/resources/Approvals/__stories__/ApprovalMatrixFixture"
import {withServiceFilters} from "@/resources/Approvals/__stories__/ApprovalsScreenFixture"
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

/** The event tabs pass a new `showList` each time the Approvals tab is chosen again. */
function Fixture({electionId}: Scenario) {
    const {permissions, tenant} = useStoryGlobals()
    const [showList, setShowList] = useState<string>()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={withServiceFilters(data.provider)}
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

const meta = {
    title: "Admin/Election event/EditElectionEventApprovals",
    component: EditElectionEventApprovals,
    args: {},
    globals: {permissions: EStoryPermissions.ADMIN},
    beforeEach: async ({args}) => {
        if (args.storedStatus) localStorage.setItem(STATUS_FILTER_KEY, args.storedStatus)
        else localStorage.removeItem(STATUS_FILTER_KEY)
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
        return () => localStorage.removeItem(STATUS_FILTER_KEY)
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

export const StatusFilterFromTheTab: Story = {
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

export const OpenTheApprovalMatrixAndGoBack: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await row(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("button", {name: "Approval matrix"}))
        expect(await ruleCards(canvasElement)).toHaveLength(8)
        await expect(canvas.getByRole("button", {name: "Add rule"})).toBeVisible()
        // Opened from the queue, the matrix points at no enrollment's rule.
        expect(canvas.queryByText("Decided the enrollment you came from")).toBeNull()
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
        expect(canvas.queryByRole("button", {name: /^Edit/})).toBeNull()
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

export const SeeTheRuleFromAReviewAndGoBack: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Alice Example", "Review enrollment")
        await review(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("button", {name: "See the rule"}))

        // The matrix opens on the rule that sent the enrollment to a person.
        const cards = await ruleCards(canvasElement)
        await expect(
            within(cards[4]).getByText("Decided the enrollment you came from")
        ).toBeVisible()
        await expect(cards[4]).toHaveAttribute("data-highlighted", "true")
        // Its way back leads to the enrollment, not to the queue.
        await userEvent.click(canvas.getByRole("button", {name: "Approvals"}))
        await expect(await review(canvasElement, "Alice Example")).toBeVisible()
        expect(canvas.queryByRole("list", {name: "Rules"})).toBeNull()
    },
}

export const SeeTheRuleFromTheQueueAndGoBack: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Carol Sample", "See the rule that decided")
        const cards = await ruleCards(canvasElement)
        await expect(
            within(cards[1]).getByText("Decided the enrollment you came from")
        ).toBeVisible()
        await userEvent.click(canvas.getByRole("button", {name: "Approvals"}))
        await expect(await row(canvasElement, "Carol Sample")).toBeVisible()
        await noReview(canvasElement)
    },
}

export const SeeTheRuleFromTheQueueAfterAReview: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await rowAction(canvasElement, "Alice Example", "Review enrollment")
        await review(canvasElement, "Alice Example")
        await userEvent.click(canvas.getByRole("button", {name: "Back to Approvals"}))
        await noReview(canvasElement)

        await rowAction(canvasElement, "Carol Sample", "See the rule that decided")
        await ruleCards(canvasElement)
        // The matrix was opened from the queue, so it goes back to the queue.
        await userEvent.click(canvas.getByRole("button", {name: "Approvals"}))
        await expect(await row(canvasElement, "Carol Sample")).toBeVisible()
        await noReview(canvasElement)
    },
}

export const ChoosingTheTabAgainShowsTheQueue: Story = {
    play: async ({canvasElement}) => {
        await rowAction(canvasElement, "Alice Example", "Review enrollment")
        await review(canvasElement, "Alice Example")
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Approvals tab"}))
        await noReview(canvasElement)
        await expect(await row(canvasElement, "Alice Example")).toBeVisible()
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useRef, useState} from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {ETaskExecutionStatus} from "@sequentech/ui-core"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {DataTreeMenuType, ResourceName} from "../ElectionEvents"
import {
    TreeStory,
    treeData,
    treeServices,
    type TreeServices,
} from "../__stories__/ElectionTreeFixture"
import MenuAction from "./MenuActions"
import {useStoryGlobals} from "../../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    resourceType: ResourceName
    isArchivedTab: boolean
    roles?: string[]
    writeFailure: boolean
    deleteTask: ETaskExecutionStatus
    reloadTree: () => void
}

let services: TreeServices

const RESOURCES: Record<ResourceName, {id: string; name: string; parent: DataTreeMenuType}> = {
    sequent_backend_election_event: {
        id: EVENT_ID,
        name: "Council",
        // An event's parent is the whole tree.
        parent: treeData("events") as DataTreeMenuType,
    },
    sequent_backend_election: {
        id: STORY_IDS.election,
        name: "Seats",
        parent: {__typename: "sequent_backend_election_event", id: EVENT_ID, name: "Council"},
    },
    sequent_backend_contest: {
        id: STORY_IDS.contest,
        name: "Members",
        parent: {
            __typename: "sequent_backend_election",
            id: STORY_IDS.election,
            election_event_id: EVENT_ID,
            name: "",
        } as DataTreeMenuType,
    },
    sequent_backend_candidate: {
        id: STORY_IDS.candidate,
        name: "Alice",
        parent: {
            __typename: "sequent_backend_contest",
            id: STORY_IDS.contest,
            election_event_id: EVENT_ID,
            name: "",
        } as DataTreeMenuType,
    },
}

/** The tree row the actions belong to, which anchors their menu as TreeMenuItem does. */
function TreeRow({resourceType, isArchivedTab, reloadTree}: Scenario) {
    const rowRef = useRef<HTMLDivElement | null>(null)
    const [anchorEl, setAnchorEl] = useState<HTMLParagraphElement | null>(null)
    const {id, name, parent} = RESOURCES[resourceType]
    return (
        <div ref={rowRef} style={{display: "flex", alignItems: "center", gap: 8, width: 240}}>
            <span>{name}</span>
            <MenuAction
                isArchivedTab={isArchivedTab}
                resourceId={id}
                resourceName={name}
                resourceType={resourceType}
                parentData={parent}
                menuItemRef={rowRef}
                setAnchorEl={setAnchorEl}
                anchorEl={anchorEl}
                reloadTree={reloadTree}
            />
        </div>
    )
}

function Fixture(args: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <TreeStory services={services} role={permissions} roles={args.roles}>
            <TreeRow {...args} />
        </TreeStory>
    )
}

const meta = {
    title: "Admin/Menu/Items/Election events/MenuAction",
    component: MenuAction,
    args: {
        resourceType: "sequent_backend_election_event",
        isArchivedTab: false,
        writeFailure: false,
        deleteTask: ETaskExecutionStatus.SUCCESS,
        reloadTree: fn(),
    },
    argTypes: {
        resourceType: {control: "select", options: Object.keys(RESOURCES)},
        deleteTask: {
            control: "inline-radio",
            options: [ETaskExecutionStatus.SUCCESS, ETaskExecutionStatus.FAILED],
        },
    },
    beforeEach: async ({args}) => {
        services = treeServices({writeFailure: args.writeFailure, deleteTask: args.deleteTask})
        await services.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const body = () => within(document.body)
// The actions read the event tree to refresh it after a change.
const operations = () =>
    services.graphql.calls.map(({name}) => name).filter((name) => name !== "election_events_tree")
const notified = (message: string | RegExp) =>
    waitFor(() => expect(body().getByText(message)).toBeVisible())
const currentLocation = (canvasElement: HTMLElement) =>
    within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})
const openedDrawer = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("status", {name: "Opened drawer"})

async function openActions(canvasElement: HTMLElement) {
    await userEvent.click(within(canvasElement).getByTestId("MoreHorizIcon"))
    return body().findByRole("menu")
}

const actionNames = (menu: HTMLElement) =>
    within(menu)
        .getAllByRole("menuitem")
        .map((item) => item.textContent)

async function confirm(action: string, prompt: RegExp) {
    const dialog = await body().findByRole("dialog")
    await waitFor(() => expect(within(dialog).getByText(prompt)).toBeVisible())
    await userEvent.click(within(dialog).getByRole("button", {name: action}))
}

export const EventActions: Story = {
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        expect(actionNames(menu)).toEqual([
            "Create an Election Event",
            "Import an Election Event",
            "Archive this Election Event",
            "Remove this Election Event",
        ])
        expect(operations()).toEqual([])
    },
}

export const CreateAnEvent: Story = {
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Create an Election Event"})
        )
        await waitFor(() => expect(openedDrawer(canvasElement)).toHaveTextContent("create"))
        await waitFor(() => expect(body().queryByRole("menu")).toBeNull())
    },
}

export const ImportAnEvent: Story = {
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Import an Election Event"})
        )
        await waitFor(() => expect(openedDrawer(canvasElement)).toHaveTextContent("import"))
    },
}

export const ArchiveTheEvent: Story = {
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Archive this Election Event"})
        )
        await confirm("Archive", /Are you sure to archive this item/)
        await notified("The item has been archived")
        expect(services.data.writes).toEqual([
            expect.objectContaining({
                method: "update",
                resource: "sequent_backend_election_event",
                params: expect.objectContaining({id: EVENT_ID, data: {is_archived: true}}),
            }),
        ])
        // The menu refreshes the tree after archiving.
        await waitFor(() =>
            expect(
                services.graphql.calls.filter(({name}) => name === "election_events_tree")
            ).toHaveLength(2)
        )
    },
}

export const ArchiveFails: Story = {
    args: {writeFailure: true},
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Archive this Election Event"})
        )
        await confirm("Archive", /Are you sure to archive this item/)
        await notified("Error while trying to archive this item")
        expect(services.data.writes).toEqual([
            expect.objectContaining({method: "update", resource: "sequent_backend_election_event"}),
        ])
    },
}

export const ArchivedEvent: Story = {
    args: {isArchivedTab: true},
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        expect(actionNames(menu)).toEqual([
            "Unarchive this Election Event",
            "Remove this Election Event",
        ])
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Unarchive this Election Event"})
        )
        await confirm("Unarchive", /Are you sure to unarchive this item/)
        await notified("The item has been unarchived")
        expect(services.data.writes).toEqual([
            expect.objectContaining({
                method: "update",
                params: expect.objectContaining({id: EVENT_ID, data: {is_archived: false}}),
            }),
        ])
    },
}

export const DeleteTheEvent: Story = {
    parameters: {
        expectedFailure: {
            reason: "The deletion task widget's white SUCCESS chip label lacks contrast.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement, args}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Remove this Election Event"})
        )
        await confirm("Delete", /Are you sure to delete this item/)
        // The deletion runs as a task, which the widget stack follows.
        await waitFor(() => expect(args.reloadTree).toHaveBeenCalledTimes(1))
        expect(services.graphql.calls.find(({name}) => name === "DeleteElectionEvent")).toEqual(
            expect.objectContaining({
                name: "DeleteElectionEvent",
                variables: {electionEventId: EVENT_ID},
                headers: expect.objectContaining({"x-hasura-role": "election-event-delete"}),
            })
        )
        expect(operations()).toContain("GetTaskById")
        expect(services.data.writes).toEqual([])
    },
}

export const DeletionTaskFails: Story = {
    args: {deleteTask: ETaskExecutionStatus.FAILED},
    play: async ({canvasElement, args}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Remove this Election Event"})
        )
        await confirm("Delete", /Are you sure to delete this item/)
        await waitFor(() => expect(operations()).toContain("GetTaskById"))
        await notified(/Delete election event/i)
        expect(args.reloadTree).not.toHaveBeenCalled()
    },
}

export const DeletionServiceFails: Story = {
    args: {writeFailure: true},
    play: async ({canvasElement, args}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Remove this Election Event"})
        )
        await confirm("Delete", /Are you sure to delete this item/)
        await waitFor(() => expect(operations()).toEqual(["DeleteElectionEvent"]))
        await notified(/Delete election event/i)
        expect(args.reloadTree).not.toHaveBeenCalled()
    },
}

export const CancelTheDeletion: Story = {
    play: async ({canvasElement, args}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Remove this Election Event"})
        )
        await confirm("Cancel", /Are you sure to delete this item/)
        await waitFor(() => expect(body().queryByRole("dialog")).toBeNull())
        expect(operations()).toEqual([])
        expect(args.reloadTree).not.toHaveBeenCalled()
    },
}

export const ElectionActions: Story = {
    args: {resourceType: "sequent_backend_election"},
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        expect(actionNames(menu)).toEqual(["Create an Election", "Remove this Election"])
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Create an Election"}))
        await waitFor(() =>
            expect(currentLocation(canvasElement)).toHaveTextContent(
                `/sequent_backend_election/create?electionEventId=${EVENT_ID}`
            )
        )
        expect(operations()).toEqual([])
    },
}

export const RemoveAContest: Story = {
    args: {resourceType: "sequent_backend_contest"},
    play: async ({canvasElement, args}) => {
        const menu = await openActions(canvasElement)
        expect(actionNames(menu)).toEqual(["Create a Contest", "Remove this Contest"])
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Remove this Contest"}))
        await confirm("Delete", /Are you sure to delete this item/)
        await notified("The item has been deleted")
        expect(services.data.writes).toEqual([
            expect.objectContaining({
                method: "delete",
                resource: "sequent_backend_contest",
                params: expect.objectContaining({id: STORY_IDS.contest}),
            }),
        ])
        expect(args.reloadTree).toHaveBeenCalledTimes(1)
        // The administrator lands on the contest's election.
        await waitFor(() =>
            expect(currentLocation(canvasElement)).toHaveTextContent(
                `/sequent_backend_election/${STORY_IDS.election}`
            )
        )
    },
}

export const RemoveFails: Story = {
    args: {resourceType: "sequent_backend_candidate", writeFailure: true},
    play: async ({canvasElement, args}) => {
        const menu = await openActions(canvasElement)
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Remove this Candidate"}))
        await confirm("Delete", /Are you sure to delete this item/)
        await notified("Error while trying to delete this item")
        expect(services.data.writes).toEqual([
            expect.objectContaining({method: "delete", resource: "sequent_backend_candidate"}),
        ])
        expect(args.reloadTree).not.toHaveBeenCalled()
        await waitFor(() => expect(currentLocation(canvasElement)).toHaveTextContent(/^\/$/))
    },
}

export const CreateAndDeleteOnlyEvents: Story = {
    args: {roles: ["election-event-create", "election-event-delete"]},
    play: async ({canvasElement}) => {
        const menu = await openActions(canvasElement)
        expect(actionNames(menu)).toEqual([
            "Create an Election Event",
            "Import an Election Event",
            "Remove this Election Event",
        ])
    },
}

export const WithoutCandidatePermissions: Story = {
    args: {resourceType: "sequent_backend_candidate", roles: ["candidate-read"]},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("Alice")).toBeVisible()
        expect(within(canvasElement).queryByTestId("MoreHorizIcon")).toBeNull()
        expect(operations()).toEqual([])
    },
}

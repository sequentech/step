// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {TREE_RESOURCE_NAMES, type DynEntityType} from "../ElectionEvents"
import {
    IMAGE_DOCUMENT_ID,
    SECOND_EVENT_ID,
    TreeStory,
    archivedTreeData,
    treeData,
    treeServices,
    type TreeDepth,
    type TreeServices,
} from "../__stories__/ElectionTreeFixture"
import {TreeMenu} from "./TreeMenu"
import {useStoryGlobals} from "../../../../../../ui-essentials/.storybook/globals"

type Tree = TreeDepth | "archived" | "none"

interface Scenario {
    tree: Tree
    isArchivedElectionEvents: boolean
    sidebarOpen: boolean
    roles?: string[]
    onArchiveElectionEventsSelect: (val: number) => void
    reloadTree: () => void
}

let services: TreeServices

const TREES: Record<Tree, () => DynEntityType> = {
    events: () => treeData("events"),
    event: () => treeData("event"),
    contest: () => treeData("contest"),
    candidate: () => treeData("candidate"),
    archived: archivedTreeData,
    none: () => ({electionEvents: []}),
}

function Fixture({tree, sidebarOpen, roles, ...props}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <TreeStory services={services} role={permissions} roles={roles} sidebarOpen={sidebarOpen}>
            <TreeMenu data={TREES[tree]()} treeResourceNames={TREE_RESOURCE_NAMES} {...props} />
        </TreeStory>
    )
}

const meta = {
    title: "Admin/Menu/Items/Election events/TreeMenu",
    component: TreeMenu,
    args: {
        tree: "contest",
        isArchivedElectionEvents: false,
        sidebarOpen: true,
        onArchiveElectionEventsSelect: fn(),
        reloadTree: fn(),
    },
    argTypes: {tree: {control: "select", options: Object.keys(TREES)}},
    beforeEach: async () => {
        services = treeServices()
        await services.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const currentLocation = (canvasElement: HTMLElement) =>
    within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})
const item = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("link", {name})
/** The tree row of an item, holding its expander and its actions. */
const row = async (canvasElement: HTMLElement, name: string) => {
    const link = await item(canvasElement, name)
    const container = link.parentElement
    if (!container) throw new Error(`Missing the row of ${name}`)
    return container
}
const documentReads = () =>
    services.data.calls.filter(
        ({method, args}) => method === "getOne" && args[0] === "sequent_backend_document"
    )
/** Opens the menu of the new election event's two ways. */
const createEventMenu = async (canvasElement: HTMLElement) => {
    await userEvent.click(
        within(canvasElement).getByRole("button", {name: "Create an Election Event"})
    )
    return within(document.body).findByRole("menu")
}

// The election image repeats its name as alt text beside the same name.
const imageAlt = {
    reason: "The election's image repeats the election name next to it as alt text.",
    a11y: ["image-redundant-alt"],
}

export const Populated: Story = {
    parameters: {widgets: ["TreeLeaves", "TreeMenuItem"], expectedFailure: imageAlt},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await item(canvasElement, "Council")).toBeVisible()
        await expect(await item(canvasElement, "Budget")).toBeVisible()
        // The selected branch opens down to the contest's candidates.
        await expect(await item(canvasElement, "Seats")).toBeVisible()
        await expect(await item(canvasElement, "Members")).toBeVisible()
        await expect(await item(canvasElement, "Alice")).toBeVisible()
        await expect(await item(canvasElement, "Bob")).toBeVisible()
        expect(canvas.getByRole("button", {name: "Create an Election Event"})).toBeVisible()
        expect(canvas.getByRole("link", {name: "Create an Election"})).toHaveAttribute(
            "href",
            `/sequent_backend_election/create?electionEventId=${EVENT_ID}`
        )
        expect(canvas.getByRole("link", {name: "Create a Contest"})).toHaveAttribute(
            "href",
            `/sequent_backend_contest/create?electionEventId=${EVENT_ID}&electionId=${STORY_IDS.election}`
        )
        expect(canvas.getByRole("link", {name: "Create a Candidate"})).toHaveAttribute(
            "href",
            `/sequent_backend_candidate/create?electionEventId=${EVENT_ID}&contestId=${STORY_IDS.contest}`
        )
        expect(services.graphql.calls.map(({name}) => name)).not.toContain("DeleteElectionEvent")
    },
}

export const ElectionImage: Story = {
    parameters: {widgets: ["TreeMenuItem"], expectedFailure: imageAlt},
    play: async ({canvasElement}) => {
        const image = await within(canvasElement).findByRole("img", {name: "Mayor"})
        expect(image).toHaveAttribute(
            "src",
            expect.stringContaining(`document-${IMAGE_DOCUMENT_ID}/mayor.png`)
        )
        expect(documentReads()).toEqual([
            expect.objectContaining({
                args: [
                    "sequent_backend_document",
                    expect.objectContaining({id: IMAGE_DOCUMENT_ID}),
                ],
            }),
        ])
    },
}

export const EventsOnly: Story = {
    args: {tree: "events"},
    parameters: {widgets: ["TreeLeaves", "TreeMenuItem"]},
    play: async ({canvasElement}) => {
        await expect(await item(canvasElement, "Council")).toBeVisible()
        expect(within(canvasElement).queryByRole("link", {name: "Seats"})).toBeNull()
        expect(within(canvasElement).queryByRole("link", {name: "Create an Election"})).toBeNull()
    },
}

export const OpenAnEvent: Story = {
    args: {tree: "events"},
    parameters: {widgets: ["TreeMenuItem"]},
    play: async ({canvasElement, args}) => {
        const budget = await row(canvasElement, "Budget")
        await userEvent.click(within(budget).getByTestId("ChevronRightIcon"))
        await waitFor(() =>
            expect(currentLocation(canvasElement)).toHaveTextContent(
                `/sequent_backend_election_event/${SECOND_EVENT_ID}`
            )
        )
        expect(args.reloadTree).not.toHaveBeenCalled()
    },
}

export const SelectAnEvent: Story = {
    args: {tree: "events"},
    parameters: {widgets: ["TreeMenuItem"]},
    play: async ({canvasElement, args}) => {
        await userEvent.click(await item(canvasElement, "Budget"))
        await waitFor(() =>
            expect(currentLocation(canvasElement)).toHaveTextContent(
                `/sequent_backend_election_event/${SECOND_EVENT_ID}`
            )
        )
        // A closed item's label reloads the tree for the event it opens.
        expect(args.reloadTree).toHaveBeenCalledTimes(1)
    },
}

export const CollapseTheSelectedEvent: Story = {
    parameters: {widgets: ["TreeMenuItem"]},
    play: async ({canvasElement, args}) => {
        await item(canvasElement, "Seats")
        const council = await row(canvasElement, "Council")
        await userEvent.click(within(council).getByTestId("ExpandMoreIcon"))
        await waitFor(() =>
            expect(within(canvasElement).queryByRole("link", {name: "Seats"})).toBeNull()
        )
        expect(currentLocation(canvasElement)).toHaveTextContent(/^\/$/)
        expect(args.reloadTree).not.toHaveBeenCalled()
    },
}

export const CreateAnEvent: Story = {
    args: {tree: "events"},
    parameters: {widgets: ["TreeLeaves"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const menu = await createEventMenu(canvasElement)
        expect(within(menu).getByRole("menuitem", {name: "Import Election Event"})).toBeVisible()
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Create an Election Event"})
        )
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Opened drawer"})).toHaveTextContent("create")
        )
        expect(services.graphql.calls.map(({name}) => name)).not.toContain("CreateElectionEvent")
    },
}

export const ImportAnEvent: Story = {
    args: {tree: "events"},
    parameters: {widgets: ["TreeLeaves"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const menu = await createEventMenu(canvasElement)
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Import Election Event"}))
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Opened drawer"})).toHaveTextContent("import")
        )
    },
}

export const CreateAnElection: Story = {
    args: {tree: "event"},
    parameters: {widgets: ["TreeLeaves"], expectedFailure: imageAlt},
    play: async ({canvasElement}) => {
        await userEvent.click(await item(canvasElement, "Create an Election"))
        await waitFor(() =>
            expect(currentLocation(canvasElement)).toHaveTextContent(
                `/sequent_backend_election/create?electionEventId=${EVENT_ID}`
            )
        )
    },
}

export const SwitchToArchived: Story = {
    args: {tree: "events"},
    play: async ({canvasElement, args}) => {
        await userEvent.click(within(canvasElement).getByText("Archived"))
        expect(args.onArchiveElectionEventsSelect).toHaveBeenCalledWith(1)
        await userEvent.click(within(canvasElement).getByText("Active"))
        expect(args.onArchiveElectionEventsSelect).toHaveBeenLastCalledWith(0)
    },
}

export const ArchivedEvents: Story = {
    args: {tree: "archived", isArchivedElectionEvents: true},
    parameters: {widgets: ["TreeLeaves", "TreeMenuItem"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await item(canvasElement, "Old council")).toBeVisible()
        // Archived events take no new resources.
        expect(canvas.queryByText("Create an Election Event")).toBeNull()
    },
}

export const NoArchivedEvents: Story = {
    args: {tree: "none", isArchivedElectionEvents: true},
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByText("No Result")).toBeVisible()
        expect(within(canvasElement).queryByRole("link")).toBeNull()
    },
}

export const ElectionReader: Story = {
    args: {roles: ["election-read"]},
    parameters: {widgets: ["TreeLeaves", "TreeMenuItem"], expectedFailure: imageAlt},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await item(canvasElement, "Seats")).toBeVisible()
        // Without contest permissions the elections do not open.
        expect(canvas.queryByRole("link", {name: "Members"})).toBeNull()
        expect(canvas.queryByText("Create an Election Event")).toBeNull()
        expect(canvas.queryByRole("link", {name: "Create an Election"})).toBeNull()
        expect(canvas.queryByRole("button", {name: /^Actions/})).toBeNull()
    },
}

export const CollapsedSidebar: Story = {
    args: {tree: "events", sidebarOpen: false},
    parameters: {widgets: ["TreeMenuItem"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(canvas.getAllByTestId("ChevronRightIcon")).toHaveLength(2)
        expect(canvas.queryByRole("link", {name: "Council"})).toBeNull()
    },
}

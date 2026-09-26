// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {Menu} from "react-admin"
import {initCore} from "@sequentech/ui-core"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {EVENT_ID, TENANT_ID} from "@/__stories__/AdminStoryProvider"
import {STORY_IDS} from "@/__stories__/fixtures"
import type {ReadState} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import {
    IMAGE_DOCUMENT_ID,
    TreeStory,
    treeServices,
    type TreeServices,
} from "./__stories__/ElectionTreeFixture"
import ElectionEvents from "./ElectionEvents"
import {useStoryGlobals} from "../../../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    sidebarOpen: boolean
    roles?: string[]
}

let services: TreeServices

// The side menu is react-admin's menu, whose role only admits menu items, yet the
// event section puts its search field and tree inside it.
const menuDefect = (...more: string[]) => ({
    reason: "The event tree and its search field sit inside a menu that may only own menu items.",
    a11y: ["aria-required-children", ...more],
})

function Fixture({sidebarOpen, roles}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <TreeStory services={services} role={permissions} roles={roles} sidebarOpen={sidebarOpen}>
            <Menu>
                <ElectionEvents />
            </Menu>
        </TreeStory>
    )
}

const meta = {
    title: "Admin/Menu/Items/ElectionEvents",
    component: ElectionEvents,
    args: {reads: "records", sidebarOpen: true},
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {expectedFailure: menuDefect()},
    beforeEach: async ({args}) => {
        await initCore()
        services = treeServices({reads: args.reads})
        await services.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const item = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).findByRole("link", {name})
const currentLocation = (canvasElement: HTMLElement) =>
    within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})
const calls = (name: string) =>
    services.graphql.calls.filter((call) => call.name === name).map(({variables}) => variables)

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await item(canvasElement, "Council")).toBeVisible()
        await expect(await item(canvasElement, "Budget")).toBeVisible()
        expect(canvas.queryByRole("link", {name: "Old council"})).toBeNull()
        expect(canvas.getByRole("textbox", {name: "Search"})).toBeVisible()
        expect(calls("election_events_tree")).toContainEqual({
            tenantId: TENANT_ID,
            isArchived: false,
        })
        expect(calls("election_tree")).toEqual([])
    },
}

export const Loading: Story = {
    args: {reads: "loading"},
    parameters: {
        expectedFailure: {
            ...menuDefect("aria-progressbar-name"),
            reason: "The tree's loading spinner has no accessible name, inside a menu that may only own menu items.",
        },
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByRole("progressbar")).toBeVisible()
        expect(within(canvasElement).queryByRole("link", {name: "Council"})).toBeNull()
        expect(calls("election_events_tree")).not.toEqual([])
    },
}

export const TreeUnavailable: Story = {
    args: {reads: "error"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        // A failed read leaves an empty tree, still open to new events.
        await expect(
            await canvas.findByRole("button", {name: "Create an Election Event"})
        ).toBeVisible()
        expect(canvas.queryByRole("link", {name: "Council"})).toBeNull()
        expect(calls("election_events_tree")).not.toEqual([])
    },
}

export const OnAContestPage: Story = {
    parameters: {
        router: {initialEntries: [`/sequent_backend_contest/${STORY_IDS.contest}`]},
        expectedFailure: {
            ...menuDefect("image-redundant-alt"),
            reason: "The event tree sits inside a menu that may only own menu items, and an election image repeats its name as alt text.",
        },
    },
    play: async ({canvasElement}) => {
        // The contest's branch loads and opens down to its candidates.
        await expect(await item(canvasElement, "Seats")).toBeVisible()
        await expect(await item(canvasElement, "Members")).toBeVisible()
        await expect(await item(canvasElement, "Alice")).toBeVisible()
        const image = await within(canvasElement).findByRole("img", {name: "Mayor"})
        expect(image).toHaveAttribute(
            "src",
            expect.stringContaining(`document-${IMAGE_DOCUMENT_ID}/mayor.png`)
        )
        expect(services.data.calls).toContainEqual({
            method: "getOne",
            args: ["sequent_backend_contest", expect.objectContaining({id: STORY_IDS.contest})],
        })
        expect(calls("election_tree")).toContainEqual({
            tenantId: TENANT_ID,
            electionEventId: EVENT_ID,
        })
        expect(calls("contest_tree")).toContainEqual({
            tenantId: TENANT_ID,
            electionId: STORY_IDS.election,
        })
        expect(calls("candidate_tree")).toContainEqual({
            tenantId: TENANT_ID,
            contestId: STORY_IDS.contest,
        })
    },
}

export const SearchTheTree: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await item(canvasElement, "Council")
        await userEvent.type(canvas.getByRole("textbox", {name: "Search"}), "budg")
        await waitFor(() => expect(canvas.queryByRole("link", {name: "Council"})).toBeNull())
        await expect(canvas.getByRole("link", {name: "Budget"})).toBeVisible()
    },
}

export const ShowArchivedEvents: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await item(canvasElement, "Council")
        await userEvent.click(canvas.getByText("Archived"))
        await expect(await item(canvasElement, "Old council")).toBeVisible()
        expect(canvas.queryByRole("link", {name: "Council"})).toBeNull()
        expect(calls("election_events_tree")).toContainEqual({
            tenantId: TENANT_ID,
            isArchived: true,
        })
        expect(currentLocation(canvasElement)).toHaveTextContent("/sequent_backend_election_event/")
        // The events list has no event id, so no event is read.
        expect(services.invalidReads).toEqual([])
    },
}

export const CreateAnEvent: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await item(canvasElement, "Council")
        await userEvent.click(canvas.getByRole("button", {name: "Add"}))
        const menu = await within(document.body).findByRole("menu")
        await userEvent.click(
            within(menu).getByRole("menuitem", {name: "Create an Election Event"})
        )
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Opened drawer"})).toHaveTextContent("create")
        )
    },
}

export const ImportAnEvent: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await item(canvasElement, "Council")
        await userEvent.click(canvas.getByRole("button", {name: "Add"}))
        const menu = await within(document.body).findByRole("menu")
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Import Election Event"}))
        await waitFor(() =>
            expect(canvas.getByRole("status", {name: "Opened drawer"})).toHaveTextContent("import")
        )
    },
}

export const WithoutCreatePermission: Story = {
    args: {roles: ["election-event-read", "election-read"]},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await item(canvasElement, "Council")).toBeVisible()
        expect(canvas.queryByRole("button", {name: "Add"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Create an Election Event"})).toBeNull()
    },
}

export const CollapsedSidebar: Story = {
    args: {sidebarOpen: false},
    parameters: {
        expectedFailure: {
            reason: "The collapsed sidebar's election events entry is an icon link without a name.",
            a11y: ["link-name"],
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await waitFor(() => expect(calls("election_events_tree")).not.toEqual([]))
        expect(canvas.getByRole("menuitem")).toHaveAttribute(
            "href",
            "/sequent_backend_election_event"
        )
        expect(canvas.queryByRole("textbox", {name: "Search"})).toBeNull()
        expect(canvas.queryByRole("link", {name: "Council"})).toBeNull()
        expect(canvas.queryByRole("button", {name: "Add"})).toBeNull()
    },
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {initCore, type ITenantSettings} from "@sequentech/ui-core"
import {EVENT_ID} from "@/__stories__/AdminStoryProvider"
import {tenantRecord} from "@/__stories__/fixtures"
import {openedWindows} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {
    TreeStory,
    treeServices,
    type TreeServices,
} from "./menu/items/__stories__/ElectionTreeFixture"
import {CustomMenu} from "./CustomMenu"
import {EStoryPermissions, useStoryGlobals} from "../../../ui-essentials/.storybook/globals"

interface Scenario {
    /** Whether the application has loaded the selected tenant. */
    tenantLoaded: boolean
    helpLinks: boolean
    sidebarOpen: boolean
    /** A role group other than the permissions global's. */
    group?: EStoryPermissions
}

let services: TreeServices

const GUIDE_URL = "${PUBLIC_BUCKET_URL}guides/admin-guide.pdf"

const settings: ITenantSettings = {
    ...(tenantRecord.settings as ITenantSettings),
    help_links: [
        {
            url: GUIDE_URL,
            title: "Administrator guide",
            i18n: {es: {title: "Guía del administrador"}},
        },
        {url: "https://support.example.invalid/", title: "Support"},
    ],
}

const tenant = (helpLinks: boolean) =>
    ({
        ...tenantRecord,
        settings: helpLinks ? settings : tenantRecord.settings,
    }) as Sequent_Backend_Tenant

function Fixture({tenantLoaded, helpLinks, sidebarOpen, group}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <TreeStory
            services={services}
            role={group ?? permissions}
            sidebarOpen={sidebarOpen}
            tenant={tenantLoaded ? tenant(helpLinks) : undefined}
        >
            <CustomMenu />
        </TreeStory>
    )
}

// React-admin's menu may only own menu items, yet it holds the tenant selector,
// the event tree and the help button.
const menuDefect = (...more: string[]) => ({
    reason: "The side menu holds the tenant selector, event tree and buttons in a menu that may only own menu items.",
    a11y: ["aria-required-children", ...more],
})

const meta = {
    title: "Admin/Components/CustomMenu",
    component: CustomMenu,
    args: {tenantLoaded: true, helpLinks: true, sidebarOpen: true},
    argTypes: {group: {control: "select", options: Object.values(EStoryPermissions)}},
    parameters: {expectedFailure: menuDefect()},
    beforeEach: async () => {
        await initCore()
        services = treeServices()
        await services.graphql.ready
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const entry = (canvasElement: HTMLElement, name: string) =>
    within(canvasElement).queryByRole("menuitem", {name})
const currentLocation = (canvasElement: HTMLElement) =>
    within(canvasElement.ownerDocument.body).getByRole("status", {name: "Current location"})
const toggle = (canvasElement: HTMLElement) => {
    const icon = canvasElement.querySelector(
        '[data-icon="angles-left"], [data-icon="angles-right"]'
    )
    const button = icon?.closest("button")
    if (!button) throw new Error("Missing the sidebar toggle")
    return button
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("example-council")).toBeVisible()
        await expect(await canvas.findByRole("link", {name: "Council"})).toBeVisible()
        expect(entry(canvasElement, "Users and Roles")).toHaveAttribute("href", "/user-roles")
        expect(entry(canvasElement, "Settings")).toHaveAttribute("href", "/settings")
        expect(entry(canvasElement, "Templates")).toHaveAttribute(
            "href",
            "/sequent_backend_template"
        )
        expect(entry(canvasElement, "Trustee Dashboard")).toBeNull()
        expect(canvas.getByRole("button", {name: "Help"})).toBeVisible()
        expect(services.graphql.calls.map(({name}) => name)).toContain("election_events_tree")
    },
}

export const OpenAHelpLink: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "Help"}))
        const menu = await within(document.body).findByRole("menu", {hidden: false})
        expect(
            within(menu)
                .getAllByRole("menuitem")
                .map((item) => item.textContent)
        ).toEqual(["Administrator guide", "Support"])
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Administrator guide"}))
        // The link's bucket placeholder becomes the public bucket's address.
        expect(openedWindows()).toEqual([
            `${globalThis.location.origin}/story-bucket/guides/admin-guide.pdf`,
        ])
    },
}

export const WithoutHelpLinks: Story = {
    args: {helpLinks: false},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("example-council")
        expect(within(canvasElement).queryByRole("button", {name: "Help"})).toBeNull()
    },
}

export const TenantNotLoaded: Story = {
    args: {tenantLoaded: false},
    play: async ({canvasElement}) => {
        // Until the tenant loads, only its selector and the event tree show.
        await expect(await within(canvasElement).findByText("example-council")).toBeVisible()
        expect(entry(canvasElement, "Users and Roles")).toBeNull()
        expect(entry(canvasElement, "Settings")).toBeNull()
        expect(entry(canvasElement, "Templates")).toBeNull()
        expect(within(canvasElement).queryByRole("button", {name: "Help"})).toBeNull()
    },
}

export const Trustee: Story = {
    args: {group: EStoryPermissions.TRUSTEE},
    play: async ({canvasElement}) => {
        await expect(
            await within(canvasElement).findByRole("menuitem", {name: "Trustee Dashboard"})
        ).toHaveAttribute("href", "/trustee")
        expect(entry(canvasElement, "Users and Roles")).toBeNull()
        expect(entry(canvasElement, "Settings")).toBeNull()
        expect(entry(canvasElement, "Templates")).toBeNull()
    },
}

export const CollapseTheSidebar: Story = {
    parameters: {
        expectedFailure: {
            ...menuDefect("link-name"),
            reason: "Collapsed, the menu's entries are icon links without names, in a menu that may only own menu items.",
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByText("example-council")
        await userEvent.click(toggle(canvasElement))
        await waitFor(() => expect(entry(canvasElement, "Settings")).toBeNull())
        expect(within(canvasElement).queryByText("example-council")).toBeNull()
        expect(canvasElement.querySelector('[data-icon="angles-right"]')).not.toBeNull()
    },
}

export const Collapsed: Story = {
    args: {sidebarOpen: false},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(services.graphql.calls.map(({name}) => name)).toContain("election_events_tree")
        )
        expect(within(canvasElement).queryByText("Settings")).toBeNull()
        await userEvent.click(toggle(canvasElement))
        await expect(await within(canvasElement).findByText("Settings")).toBeVisible()
    },
}

export const LeaveTheEventList: Story = {
    parameters: {router: {initialEntries: ["/sequent_backend_election_event"]}},
    play: async ({canvasElement}) => {
        // The event list is not a page of its own; the menu returns to the start page.
        await waitFor(() => expect(currentLocation(canvasElement)).toHaveTextContent(/^\/$/))
    },
}

export const StayOnAnEvent: Story = {
    parameters: {
        router: {initialEntries: [`/sequent_backend_election_event/${EVENT_ID}`]},
        expectedFailure: {
            ...menuDefect("image-redundant-alt"),
            reason: "The side menu owns more than menu items, and an election image repeats its name as alt text.",
        },
    },
    play: async ({canvasElement}) => {
        await expect(await within(canvasElement).findByRole("link", {name: "Seats"})).toBeVisible()
        expect(currentLocation(canvasElement)).toHaveTextContent(
            `/sequent_backend_election_event/${EVENT_ID}`
        )
    },
}

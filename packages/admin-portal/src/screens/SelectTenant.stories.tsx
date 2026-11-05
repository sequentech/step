// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within, type Mock} from "storybook/test"
import {i18n} from "@sequentech/ui-core"
import {
    AdminStoryProvider,
    STORY_SETTINGS,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {storyFetch} from "@/__stories__/storyNetwork"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import {SelectTenant} from "./SelectTenant"
import {
    DEFAULT_TENANT_CSS,
    HASURA_URL,
    REALM_CONFIGURATION_URL,
    TENANT_SLUG,
    realmConfiguration,
    tenantLookup,
} from "./__stories__/SelectTenantFixture"

interface Scenario {
    /** Whether a tenant has the council slug. */
    tenantExists: boolean
    discovery: "normal" | "unavailable"
    /** Whether the tenant's Keycloak realm exists. */
    realmExists: boolean
    /** Whether five failed attempts locked the screen a minute ago. */
    locked: boolean
    /** Whether the user has already signed in. */
    authenticated: boolean
    initKeycloak: Mock<(tenantId: string) => Promise<boolean>>
}

const ATTEMPT_KEY = "next-tenant-attempt"

let graphql: ReturnType<typeof graphqlBoundary>
let network: ReturnType<typeof storyFetch>

const meta = {
    title: "Admin/Screens/SelectTenant",
    component: SelectTenant,
    args: {
        tenantExists: true,
        discovery: "normal",
        realmExists: true,
        locked: false,
        authenticated: false,
        initKeycloak: fn(async () => true),
    },
    parameters: {router: {path: "/tenant", initialEntries: ["/tenant"]}},
    beforeEach: async ({args}) => {
        graphql = graphqlBoundary({})
        network = storyFetch({
            [HASURA_URL]: (request) => {
                const body = JSON.parse(request.body ?? "{}") as {variables?: {id?: string}}
                if (args.discovery === "unavailable" && body.variables?.id) {
                    throw new TypeError("Default tenant discovery cancelled")
                }
                return tenantLookup(args.tenantExists)(request)
            },
            [REALM_CONFIGURATION_URL]: realmConfiguration(args.realmExists),
        })
        const previous = localStorage.getItem(ATTEMPT_KEY)
        if (args.locked) {
            localStorage.setItem(ATTEMPT_KEY, new Date(Date.now() - 60_000).toISOString())
        } else {
            localStorage.removeItem(ATTEMPT_KEY)
        }
        return () => {
            if (previous === null) localStorage.removeItem(ATTEMPT_KEY)
            else localStorage.setItem(ATTEMPT_KEY, previous)
        }
    },
    render: ({authenticated, initKeycloak}) => (
        <AdminStoryProvider
            boundary={graphql}
            auth={{isAuthenticated: authenticated, initKeycloak}}
        >
            <SelectTenant />
        </AdminStoryProvider>
    ),
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const nameField = (canvasElement: HTMLElement) =>
    within(canvasElement).findByRole("textbox", {name: i18n.t("common.label.tenantName")})

/** The GraphQL queries the screen sent, in order. */
const queries = () =>
    network.calls
        .filter(({url}) => url === HASURA_URL)
        .map(
            ({body}) => (JSON.parse(String(body)) as {variables: Record<string, string>}).variables
        )

async function selectTenant(canvasElement: HTMLElement, name: string) {
    await userEvent.type(await nameField(canvasElement), name)
    await userEvent.click(
        within(canvasElement).getByRole("button", {name: i18n.t("common.label.continue")})
    )
}

async function expectAlert(text: string) {
    const alert = await within(document.body).findByRole("alert")
    await waitFor(() => expect(alert).toHaveTextContent(text))
}

export const Populated: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await nameField(canvasElement)).toBeEnabled()
        await expect(canvas.getByText(i18n.t("common.label.selectTenant"))).toBeVisible()
        // The default tenant's look and feel styles the screen.
        await waitFor(() => expect(queries()).toEqual([{id: STORY_SETTINGS.DEFAULT_TENANT_ID}]))
        expect(DEFAULT_TENANT_CSS).toContain("letter-spacing: 3px")
        await waitFor(() =>
            expect(getComputedStyle(canvas.getByRole("heading", {level: 1})).letterSpacing).toBe(
                "3px"
            )
        )
    },
}

export const SignInToATenant: Story = {
    play: async ({canvasElement, args}) => {
        await selectTenant(canvasElement, ` ${TENANT_SLUG} `)
        await waitFor(() => expect(args.initKeycloak).toHaveBeenCalledWith(TENANT_ID))
        expect(queries().at(-1)).toEqual({slug: TENANT_SLUG})
        expect(network.calls.map(({method, url}) => [method, url])).toContainEqual([
            "GET",
            REALM_CONFIGURATION_URL,
        ])
        // The button waits for Keycloak's redirect.
        await expect(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.processing")})
        ).toBeDisabled()
    },
}

export const UnknownTenant: Story = {
    args: {tenantExists: false},
    play: async ({canvasElement, args}) => {
        await selectTenant(canvasElement, "elsewhere")
        await expectAlert("Tenant not found")
        expect(queries().at(-1)).toEqual({slug: "elsewhere"})
        expect(network.calls.map(({url}) => url)).not.toContain(REALM_CONFIGURATION_URL)
        expect(args.initKeycloak).not.toHaveBeenCalled()
        await expect(
            within(canvasElement).getByRole("button", {name: i18n.t("common.label.continue")})
        ).toBeEnabled()
    },
}

export const MissingRealm: Story = {
    args: {realmExists: false},
    play: async ({canvasElement, args}) => {
        await selectTenant(canvasElement, TENANT_SLUG)
        await expectAlert("Tenant realm not found")
        expect(args.initKeycloak).not.toHaveBeenCalled()
    },
}

export const BlankName: Story = {
    play: async ({canvasElement}) => {
        await selectTenant(canvasElement, "   ")
        await expectAlert("Please enter a tenant name")
        expect(queries()).toEqual([{id: STORY_SETTINGS.DEFAULT_TENANT_ID}])
    },
}

export const LockedAfterTooManyAttempts: Story = {
    args: {locked: true},
    play: async ({canvasElement}) => {
        await expect(await nameField(canvasElement)).toBeDisabled()
        await expectAlert("Too many attempts. Please try again later.")
    },
}

export const AlreadySignedIn: Story = {
    args: {authenticated: true},
    // The screen stays mounted on the application's root, where it sends the user.
    parameters: {router: {path: "*", initialEntries: ["/tenant"]}},
    play: async ({canvasElement}) => {
        await waitFor(() =>
            expect(
                within(canvasElement).getByRole("status", {name: "Current location"})
            ).toHaveTextContent(/^\/$/)
        )
        await nameField(canvasElement)
    },
}

export const Loading: Story = {
    parameters: {
        expectedFailure: {
            reason: "The screen's loading spinner is a progress bar without a name.",
            a11y: ["aria-progressbar-name"],
        },
    },
    play: async ({canvasElement}) => {
        await expect(within(canvasElement).getByRole("progressbar")).toBeVisible()
        expect(within(canvasElement).queryByRole("textbox")).toBeNull()
    },
}

export const DefaultTenantDiscoveryUnavailable: Story = {
    args: {discovery: "unavailable"},
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await nameField(canvasElement)
        await waitFor(() =>
            expect(canvas.getByRole("img", {name: "Logo Image"})).toHaveAttribute(
                "src",
                SequentLogo
            )
        )
        // Branding discovery never authorizes a login; normal lookup and realm checks still run.
        expect(args.initKeycloak).not.toHaveBeenCalled()
        await selectTenant(canvasElement, TENANT_SLUG)
        await waitFor(() => expect(args.initKeycloak).toHaveBeenCalledWith(TENANT_ID))
        expect(queries().at(-1)).toEqual({slug: TENANT_SLUG})
    },
}

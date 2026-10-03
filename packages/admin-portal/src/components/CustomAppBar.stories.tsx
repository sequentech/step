// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import type {StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, waitFor, within} from "storybook/test"
import i18n from "i18next"
import {USER_LANGUAGE_COOKIE_NAME, getValueFromCookie} from "@sequentech/ui-core"
import SequentLogo from "@sequentech/ui-essentials/public/Sequent_logo.svg"
import BlankLogoImg from "@sequentech/ui-essentials/public/blank_logo.svg"
import {
    AdminStoryProvider,
    EVENT_ID,
    TENANT_ID,
    graphqlBoundary,
} from "@/__stories__/AdminStoryProvider"
import {tenantRecord} from "@/__stories__/fixtures"
import {type ReadState, resourceBoundary} from "@/__stories__/resourceBoundary"
import type {WidgetMeta} from "@/__stories__/widgetStory"
import type {Sequent_Backend_Tenant} from "@/gql/graphql"
import {CustomAppBar} from "./CustomAppBar"
import {SigningProvider} from "./signing/SigningProvider"
import {
    idleSigningApi,
    signingHandlers,
    organizationOf,
    Organization,
} from "@/resources/ElectionEvent/Signatures/__stories__/SignaturesFixture"
import {useStoryGlobals} from "../../../ui-essentials/.storybook/globals"

interface Scenario {
    reads: ReadState
    roles?: string[]
    requests?: "empty" | "waiting"
    /** The tenant's own logo, when it has one. */
    logoUrl?: string
    signedIn: boolean
    logout: () => void
    openProfileLink: () => Promise<void>
}

const TENANT_LOGO = `${globalThis.location.origin}/story-bucket/council-logo.svg`

// React-admin's app bar is a banner landmark and the shared header another one inside it.
const nestedBanners = (...more: string[]) => ({
    reason: "The shared header's banner landmark is nested in react-admin's app bar banner.",
    a11y: [
        "landmark-banner-is-top-level",
        "landmark-no-duplicate-banner",
        "landmark-unique",
        ...more,
    ],
})

let graphql: ReturnType<typeof graphqlBoundary>
let data: ReturnType<typeof resourceBoundary>

const tenant = (logoUrl?: string) =>
    ({
        ...tenantRecord,
        annotations: logoUrl ? {logo_url: logoUrl} : {},
    }) as Sequent_Backend_Tenant

function Fixture({logoUrl, signedIn, logout, openProfileLink, roles}: Scenario) {
    const {permissions} = useStoryGlobals()
    return (
        <AdminStoryProvider
            boundary={graphql}
            dataProvider={data.provider}
            role={permissions}
            roles={roles}
            tenantRecord={tenant(logoUrl)}
            auth={{
                isAuthenticated: signedIn,
                email: "admin@example.invalid",
                logout,
                openProfileLink,
            }}
        >
            <SigningProvider api={idleSigningApi()}>
                <CustomAppBar />
            </SigningProvider>
        </AdminStoryProvider>
    )
}

const meta = {
    title: "Admin/Components/CustomAppBar",
    component: CustomAppBar,
    args: {
        reads: "records",
        signedIn: true,
        logout: fn(),
        openProfileLink: fn(async () => {}),
    },
    argTypes: {reads: {control: "inline-radio", options: ["records", "loading", "error"]}},
    parameters: {expectedFailure: nestedBanners()},
    beforeEach: async ({args}) => {
        const organization = organizationOf(Organization.Overseas)
        data = resourceBoundary(
            {
                sequent_backend_tenant: [tenant(args.logoUrl) as typeof tenantRecord],
                sequent_backend_election: organization.posts.map(({id, name}) => ({id, name})),
                sequent_backend_area: organization.countries.map(({id, name}) => ({id, name})),
            },
            {reads: args.reads}
        )
        graphql = graphqlBoundary(
            {
                GetWaitingSigningRequests:
                    args.requests === "waiting"
                        ? signingHandlers(organization).GetWaitingSigningRequests
                        : () => ({data: {sequent_backend_signing_request: []}}),
                SigningEventInfo: () => ({data: {signingEventInfo: {time_zone: null, titles: {}}}}),
            },
            {schema: true}
        )
        await graphql.ready
        const cookie = getValueFromCookie(USER_LANGUAGE_COOKIE_NAME)
        // The language menu changes the page's language and remembers it in a cookie.
        return async () => {
            await i18n.changeLanguage("en")
            document.cookie = cookie
                ? `${USER_LANGUAGE_COOKIE_NAME}=${cookie}; Path=/`
                : `${USER_LANGUAGE_COOKIE_NAME}=; Path=/; Max-Age=0`
        }
    },
    render: (args) => <Fixture {...args} />,
} satisfies WidgetMeta<Scenario>
export default meta
type Story = StoryObj<Scenario>

const logo = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("img", {name: "Logo Image"})
const tenantReads = () =>
    data.calls.filter(
        ({method, args}) => method === "getOne" && args[0] === "sequent_backend_tenant"
    )
const profileButton = (canvasElement: HTMLElement) =>
    within(canvasElement).getByRole("button", {name: /Welcome,\s*admin/})

export const Populated: Story = {
    play: async ({canvasElement}) => {
        await waitFor(() => expect(logo(canvasElement)).toHaveAttribute("src", SequentLogo))
        await expect(profileButton(canvasElement)).toBeVisible()
        expect(within(canvasElement).getByRole("button", {name: "English"})).toBeVisible()
        expect(tenantReads()).toEqual([
            {
                method: "getOne",
                args: ["sequent_backend_tenant", expect.objectContaining({id: TENANT_ID})],
            },
        ])
    },
}

export const TenantLogo: Story = {
    args: {logoUrl: TENANT_LOGO},
    play: async ({canvasElement}) => {
        await waitFor(() => expect(logo(canvasElement)).toHaveAttribute("src", TENANT_LOGO))
        expect(tenantReads()).toHaveLength(1)
    },
}

export const LoadingTenant: Story = {
    args: {reads: "loading"},
    play: async ({canvasElement}) => {
        // Until the tenant arrives the header keeps a blank logo, not Sequent's.
        await waitFor(() => expect(tenantReads()).toHaveLength(1))
        expect(logo(canvasElement)).toHaveAttribute("src", BlankLogoImg)
    },
}

export const ChangeTheLanguage: Story = {
    play: async ({canvasElement}) => {
        await userEvent.click(within(canvasElement).getByRole("button", {name: "English"}))
        const menu = await within(document.body).findByRole("menu")
        expect(
            within(menu)
                .getAllByRole("menuitem")
                .map((item) => item.textContent)
        ).toEqual(["English", "Español"])
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Español"}))
        await waitFor(() => expect(getValueFromCookie(USER_LANGUAGE_COOKIE_NAME)).toBe("es"))
        await expect(within(canvasElement).getByRole("button", {name: "Español"})).toBeVisible()
    },
}

export const OpenTheProfile: Story = {
    play: async ({canvasElement, args}) => {
        await userEvent.click(profileButton(canvasElement))
        const menu = await within(document.body).findByRole("menu")
        expect(within(menu).getByText("admin@example.invalid")).toBeVisible()
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Profile"}))
        expect(args.openProfileLink).toHaveBeenCalledTimes(1)
    },
}

export const LogOut: Story = {
    // The logout dialog hides the header from assistive technology.
    parameters: {expectedFailure: null},
    play: async ({canvasElement, args}) => {
        await userEvent.click(profileButton(canvasElement))
        const menu = await within(document.body).findByRole("menu")
        await userEvent.click(within(menu).getByRole("menuitem", {name: "Logout"}))
        const dialog = await within(document.body).findByRole("dialog")
        await waitFor(() =>
            expect(within(dialog).getByText(/Are you sure you want to logout/)).toBeVisible()
        )
        await userEvent.click(within(dialog).getByRole("button", {name: "OK"}))
        expect(args.logout).toHaveBeenCalledTimes(1)
    },
}

export const SignedOut: Story = {
    args: {signedIn: false},
    parameters: {
        // The open menu hides the header, and with it the nested banners.
        expectedFailure: {
            reason: "The profile menu's grey email lacks contrast on the highlighted item.",
            a11y: ["color-contrast"],
        },
    },
    play: async ({canvasElement}) => {
        await userEvent.click(profileButton(canvasElement))
        const menu = await within(document.body).findByRole("menu")
        expect(within(menu).queryByRole("menuitem", {name: "Logout"})).toBeNull()
    },
}

const eventHeader = {
    router: {
        path: "/sequent_backend_election_event/:id/*",
        initialEntries: [`/sequent_backend_election_event/${EVENT_ID}`],
    },
}

export const SignaturesInTheHeader: Story = {
    args: {roles: ["sign-close-voting"]},
    // Opening the drawer hides the header's existing nested banner landmarks.
    parameters: {...eventHeader, expectedFailure: null},
    play: async ({canvasElement}) => {
        const button = await within(canvasElement).findByRole("button", {
            name: i18n.t("signing.waiting.buttonCount", {count: 0}),
        })
        expect(button.closest(".header-actions")).not.toBeNull()
        expect(button.querySelector(".MuiBox-root")).toBeNull()
        await expect(button.querySelector(".MuiBadge-badge")).toHaveClass(/MuiBadge-invisible/)
        await userEvent.click(button)
        const panel = within(
            await within(document.body).findByRole("dialog", {
                name: i18n.t("signing.waiting.title"),
            })
        )
        await expect(await panel.findByText(i18n.t("signing.waiting.empty"))).toBeVisible()
    },
}

export const NoSignatureActionWithoutPermission: Story = {
    args: {roles: ["election-event-read"]},
    parameters: eventHeader,
    play: async ({canvasElement}) => {
        expect(
            within(canvasElement).queryByRole("button", {
                name: i18n.t("signing.waiting.buttonCount", {count: 0}),
            })
        ).toBeNull()
        expect(graphql.calls.some(({name}) => name === "GetWaitingSigningRequests")).toBe(false)
    },
}

export const PendingSignaturesInTheHeader: Story = {
    args: {roles: ["sign-close-voting", "sign-generate-election-returns"], requests: "waiting"},
    parameters: {...eventHeader, expectedFailure: null},
    play: async ({canvasElement}) => {
        const button = await within(canvasElement).findByRole("button", {
            name: i18n.t("signing.waiting.buttonCount", {count: 2}),
        })
        expect(button.closest(".header-actions")).not.toBeNull()
        expect(button.textContent?.trim()).toBe("2")
        await expect(within(button).getByText("2")).toBeVisible()
        await userEvent.click(button)
        const panel = within(
            await within(document.body).findByRole("dialog", {
                name: i18n.t("signing.waiting.title"),
            })
        )
        expect(await panel.findAllByRole("listitem")).toHaveLength(2)
    },
}

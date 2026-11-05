// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"

const {KcPageStory} = createKcPageStory({pageId: "login-username.ftl"})

const meta = {
    title: "Keycloak/Login username",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

const EMAIL = "trainee@example.test"
const EMAIL_REALM = {realm: {registrationEmailAsUsername: true, loginWithEmailAllowed: true}}
const GOOGLE = {
    social: {
        providers: [
            {
                alias: "google",
                displayName: "Google",
                providerId: "google",
                loginUrl: "/synthetic/broker/google/login",
                iconClasses: "",
            },
        ],
    },
}

export const Default: Story = {
    args: {kcContext: EMAIL_REALM},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const email = canvas.getByLabelText("Email")
        await expect(email).toHaveAttribute("autocomplete", "username")
        await expect(email).toHaveAttribute("type", "email")
        await expect(email).toHaveFocus()
        await expect(canvas.queryByLabelText("Password")).toBeNull()
        await expect(canvas.queryByRole("navigation")).toBeNull()
        await userEvent.type(email, EMAIL)
        const form = email.closest("form")!
        await expect(new FormData(form).get("username")).toBe(EMAIL)
        await expect(canvas.getByRole("button", {name: /continue/i})).toBeEnabled()
    },
}

export const WithGoogle: Story = {
    args: {kcContext: {...EMAIL_REALM, ...GOOGLE}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const providers = within(canvas.getByRole("navigation", {name: "Or sign in with"}))
        const google = providers.getByRole("link", {name: /continue with google/i})
        await expect(google).toHaveAttribute("href", "/synthetic/broker/google/login")
        // The address stays the first action; Google follows it in tab order.
        await userEvent.tab()
        await expect(canvas.getByRole("button", {name: /continue$/i})).toHaveFocus()
        await userEvent.tab()
        await expect(google).toHaveFocus()
    },
}

export const UnknownAddress: Story = {
    args: {
        kcContext: {
            ...EMAIL_REALM,
            login: {username: EMAIL},
            messagesPerField: {
                existsError: (...fieldNames: string[]) => fieldNames.includes("username"),
                getFirstError: () => "Invalid username.",
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent("Invalid username.")
        const email = canvas.getByLabelText("Email")
        await expect(email).toHaveValue(EMAIL)
        await expect(email).toHaveAttribute("aria-invalid", "true")
        await expect(email).toHaveAccessibleDescription("Invalid username.")
    },
}

export const ElectionArchitect: Story = {
    args: {kcContext: {...EMAIL_REALM, ...GOOGLE, themeName: "sequent-ui-architect"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Sign in to Election Architect"})
        ).toBeVisible()
        await expect(
            within(canvas.getByRole("banner")).getByText("Election Architect")
        ).toBeVisible()
    },
}

export const ElectionArchitectSpanish: Story = {
    args: {
        locale: "es",
        kcContext: {...EMAIL_REALM, ...GOOGLE, themeName: "sequent-ui-architect"},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Iniciar sesión en Election Architect"})
        await expect(canvas.getByRole("button", {name: /continuar$/i})).toBeVisible()
        await expect(canvas.getByRole("link", {name: /continuar con google/i})).toBeVisible()
        await expect(canvas.getByRole("navigation", {name: "O inicie sesión con"})).toBeVisible()
        // The product name keeps its English language tag inside Spanish copy.
        await expect(
            within(canvas.getByRole("banner")).getByText("Election Architect")
        ).toHaveAttribute("lang", "en")
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("lang", "es")
    },
}

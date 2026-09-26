// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, within} from "storybook/test"
import {KEYCLOAK_SYNTHETIC_USER} from "@sequentech/ui-test-kit/fixtures/keycloak"
import {createKcPageStory} from "../KcPageStory"
import {LoginHintUsernamePolicy, LoginValidationPolicy} from "../KcContext"

const {KcPageStory} = createKcPageStory({pageId: "login.ftl"})

const meta = {
    title: "Keycloak/Login",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

export const Default: Story = {
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        await expect(canvas.getByLabelText("Username or email")).toBeVisible()
        await expect(canvas.getByLabelText("Password")).toBeVisible()
        await expect(canvas.getByRole("button", {name: "LOGIN"})).toBeEnabled()
    },
}

export const InvalidCredentials: Story = {
    args: {
        kcContext: {
            login: {username: KEYCLOAK_SYNTHETIC_USER.username},
            messagesPerField: {
                existsError: (...fieldNames: string[]) =>
                    fieldNames.some((name) => name === "username" || name === "password"),
                getFirstError: () => "The details you entered are incorrect.",
            },
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        await expect(canvas.getByText("The details you entered are incorrect.")).toBeVisible()
        await expect(canvas.getByLabelText("Username or email")).toHaveAttribute(
            "aria-invalid",
            "true"
        )
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await expect(
            within(canvasElement).getByRole("button", {name: "INICIAR SESIÓN"})
        ).toBeVisible()
    },
}

export const ReadOnlyLoginHint: Story = {
    args: {
        kcContext: {
            themeName: "sequent-ui-voting",
            login: {username: KEYCLOAK_SYNTHETIC_USER.username},
            sequent: {loginHintUsernamePolicy: LoginHintUsernamePolicy.ReadOnly},
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        const username = canvas.getByLabelText("Username or email")
        await expect(username).toHaveAttribute("readonly")
        const form = username.closest("form")!
        await expect(new FormData(form).get("username")).toBe("synthetic-voter")
        await userEvent.type(username, "different-voter")
        await expect(username).toHaveValue("synthetic-voter")
    },
}

export const RememberedUsernameStaysEditable: Story = {
    args: {
        kcContext: {
            themeName: "sequent-ui-voting",
            realm: {rememberMe: true},
            login: {username: KEYCLOAK_SYNTHETIC_USER.username, rememberMe: "on"},
            sequent: {loginHintUsernamePolicy: LoginHintUsernamePolicy.ReadOnly},
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const canvas = within(canvasElement)
        const username = canvas.getByLabelText("Username or email")
        await expect(username).not.toHaveAttribute("readonly")
        await userEvent.clear(username)
        await userEvent.type(username, "another-synthetic-voter")
        await expect(username).toHaveValue("another-synthetic-voter")
    },
}

export const ServerValidation: Story = {
    args: {kcContext: {sequent: {loginValidationPolicy: LoginValidationPolicy.ServerOnly}}},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        const form = within(canvasElement).getByLabelText("Password").closest("form")!
        await expect(form.noValidate).toBe(true)
    },
}

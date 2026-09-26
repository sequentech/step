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
        await expect(canvas.getByLabelText("Username or email")).toHaveAttribute(
            "autocomplete",
            "username"
        )
        await expect(canvas.getByLabelText("Password")).toHaveAttribute(
            "autocomplete",
            "current-password"
        )
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
        await expect(canvas.getByRole("alert")).toHaveTextContent(
            "The details you entered are incorrect."
        )
        for (const label of ["Username or email", "Password"]) {
            await expect(canvas.getByLabelText(label)).toHaveAttribute("aria-invalid", "true")
            await expect(canvas.getByLabelText(label)).toHaveAccessibleDescription(
                "The details you entered are incorrect."
            )
        }
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await expect(
            within(canvasElement).getByRole("button", {name: "INICIAR SESIÓN"})
        ).toBeVisible()
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("lang", "es")
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("dir", "ltr")
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

export const PasswordManagerAndKeyboard: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const username = canvas.getByLabelText("Username or email")
        const password = canvas.getByLabelText("Password")
        await userEvent.click(username)
        await userEvent.clear(username)
        await userEvent.paste(KEYCLOAK_SYNTHETIC_USER.username)
        await userEvent.tab()
        await expect(password).toHaveFocus()
        await userEvent.paste("synthetic password manager value")
        const submitted = new FormData(password.closest("form")!)
        await expect(submitted.get("username")).toBe(KEYCLOAK_SYNTHETIC_USER.username)
        await expect(submitted.get("password")).toBe("synthetic password manager value")
    },
}

export const PasswordOnlyError: Story = {
    args: {
        kcContext: {
            usernameHidden: true,
            realm: {rememberMe: true},
            messagesPerField: {
                existsError: (...fields: string[]) => fields.includes("password"),
                getFirstError: () => "The details you entered are incorrect.",
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByLabelText("Username or email")).not.toBeInTheDocument()
        await expect(canvas.queryByRole("checkbox")).not.toBeInTheDocument()
        const password = canvas.getByLabelText("Password")
        await expect(password).toHaveFocus()
        await expect(password).toHaveAttribute("aria-invalid", "true")
        await expect(password).toHaveAccessibleDescription("The details you entered are incorrect.")
        await expect(canvas.getByRole("alert")).toBeVisible()
    },
}

export const RealmLoginOptions: Story = {
    args: {
        kcContext: {
            realm: {rememberMe: true, resetPasswordAllowed: true, registrationAllowed: true},
            registrationDisabled: false,
            login: {rememberMe: "on"},
            url: {
                loginResetCredentialsUrl: "/synthetic-reset-password",
                registrationUrl: "/synthetic-registration",
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const remember = canvas.getByRole("checkbox", {name: "Remember me"})
        await expect(remember).toBeChecked()
        const form = canvas.getByLabelText("Password").closest("form")!
        await expect(new FormData(form).get("rememberMe")).toBe("on")
        await userEvent.click(remember)
        await expect(new FormData(form).has("rememberMe")).toBe(false)
        await expect(canvas.getByRole("link", {name: "Forgot Password?"})).toHaveAttribute(
            "href",
            "/synthetic-reset-password"
        )
        await expect(canvas.getByRole("link", {name: "Register"})).toHaveAttribute(
            "href",
            "/synthetic-registration"
        )
    },
}

export const DisabledRealmOptions: Story = {
    args: {
        kcContext: {
            realm: {rememberMe: false, resetPasswordAllowed: false, registrationAllowed: true},
            registrationDisabled: true,
            social: {providers: []},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByRole("checkbox")).not.toBeInTheDocument()
        await expect(canvas.queryByRole("link", {name: "Forgot Password?"})).not.toBeInTheDocument()
        await expect(canvas.queryByRole("link", {name: "Register"})).not.toBeInTheDocument()
        await expect(
            canvas.queryByRole("link", {name: "Synthetic organisation"})
        ).not.toBeInTheDocument()
    },
}

export const PasswordVisibility: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const password = canvas.getByLabelText("Password")
        await userEvent.type(password, "synthetic visible password")
        await expect(password).toHaveAttribute("type", "password")
        const reveal = canvas.getByRole("button", {name: "Show password"})
        await userEvent.click(reveal)
        await expect(password).toHaveAttribute("type", "text")
        await expect(password).toHaveValue("synthetic visible password")
        const hide = canvas.getByRole("button", {name: "Hide password"})
        await expect(hide).toHaveFocus()
        await userEvent.keyboard("{Enter}")
        await expect(password).toHaveAttribute("type", "password")
        await expect(password).toHaveValue("synthetic visible password")
        await expect(canvas.getByRole("button", {name: "LOGIN"})).toBeEnabled()
    },
}

export const SubmittingPreventsDuplicateLogin: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const form = canvas.getByLabelText("Password").closest("form")!
        let submissions = 0
        const preventNavigation = (event: Event) => {
            event.preventDefault()
            submissions += 1
        }
        form.addEventListener("submit", preventNavigation)
        try {
            await userEvent.type(canvas.getByLabelText("Username or email"), "synthetic-voter")
            await userEvent.type(canvas.getByLabelText("Password"), "synthetic-password")
            const login = canvas.getByRole("button", {name: "LOGIN"})
            await userEvent.click(login)
            await expect(login).toBeDisabled()
            await userEvent.keyboard("{Enter}")
            await expect(submissions).toBe(1)
        } finally {
            form.removeEventListener("submit", preventNavigation)
        }
    },
}

export const RightToLeftLayout: Story = {
    // Exercise the server's direction flag without claiming an Arabic translation.
    args: {kcContext: {locale: {rtl: true}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("dir", "rtl")
        await expect(canvas.getByLabelText("Username or email")).toBeVisible()
        await expect(canvas.getByLabelText("Password")).toBeVisible()
    },
}

export const FrenchLayoutWithEnglishHeadingFallback: Story = {
    args: {kcContext: {locale: {currentLanguageTag: "fr"}}},
    // Keep the server locale; the global preview toolbar only offers en/es.
    render: ({kcContext}) => <KcPageStory kcContext={kcContext} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const heading = await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("lang", "fr")
        await expect(canvas.getByText("Votre espace électoral")).toHaveAttribute("lang", "fr")
        await expect(canvas.getByText("Portail d’administration")).toHaveAttribute("lang", "fr")
        await expect(heading).toHaveTextContent("Sign in to continue")
        await expect(heading).toHaveAttribute("lang", "en")
    },
}

export const UnsupportedLayoutLocaleUsesExplicitEnglish: Story = {
    args: {kcContext: {locale: {currentLanguageTag: "de"}}},
    render: ({kcContext}) => <KcPageStory kcContext={kcContext} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const heading = await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.ownerDocument.documentElement).toHaveAttribute("lang", "de")
        await expect(canvas.getByText("Your election workspace")).toHaveAttribute("lang", "en")
        await expect(canvas.getByText("Admin Portal")).toHaveAttribute("lang", "en")
        await expect(heading).toHaveTextContent("Sign in to continue")
        await expect(heading).toHaveAttribute("lang", "en")
    },
}

export const FrenchServerHeadingKeepsItsLanguage: Story = {
    args: {
        kcContext: {
            locale: {currentLanguageTag: "fr"},
            "x-keycloakify": {
                messages: {loginAccountTitle: "Bienvenue dans votre espace électoral"},
            },
        },
    },
    render: ({kcContext}) => <KcPageStory kcContext={kcContext} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const heading = await canvas.findByRole("heading", {level: 1})
        await expect(heading).toHaveTextContent("Bienvenue dans votre espace électoral")
        await expect(heading).toHaveAttribute("lang", "fr")
        await expect(canvas.getByText("Votre espace électoral")).toHaveAttribute("lang", "fr")
    },
}

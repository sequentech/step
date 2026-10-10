// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {KEYCLOAK_PROFILE_ATTRIBUTES} from "@sequentech/ui-test-kit/fixtures/keycloak"
import {createKcPageStory} from "../KcPageStory"
import {CredentialFieldPosition} from "../KcContext"
import {expectStickyActions} from "../scanovate/stickyActions"

const {KcPageStory} = createKcPageStory({pageId: "register.ftl"})

const meta = {
    title: "Keycloak/Register",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

const attribute = (name: string, displayName: string, extra: Record<string, unknown> = {}) => ({
    name,
    displayName,
    required: false,
    readOnly: false,
    validators: {},
    annotations: {},
    ...extra,
})

const [colour, phone] = KEYCLOAK_PROFILE_ATTRIBUTES

// The annotations sequent-theme's registration form understands.
const annotated = {
    username: attribute("username", "${username}", {
        required: true,
        annotations: {
            passwordStrengthBar: "true",
            passwordHelperTextBefore: "At least 8 characters.",
            "html-attribute:style": "text-transform: uppercase",
            "html-attribute:autocomplete": "off",
        },
    }),
    [colour.name]: attribute(colour.name, colour.displayName, {
        annotations: colour.annotations,
        validators: colour.validations,
    }),
    country: attribute("country", "Country", {
        annotations: {inputType: "select", filterSelectAttribute: "post"},
        validators: {options: {options: ["PH", "ES"]}},
    }),
    post: attribute("post", "Post", {
        annotations: {
            inputType: "select",
            inputOptionLabels: {"PH-manila": "Manila", "PH-cebu": "Cebu", "ES-madrid": "Madrid"},
        },
        validators: {options: {options: ["PH-manila", "PH-cebu", "ES-madrid"]}},
    }),
    contact: attribute("contact", "Contact me by", {
        required: true,
        values: ["email"],
        annotations: {
            inputType: "multiselect-checkboxes",
            disableAttribute: "true",
            inputOptionLabels: {email: "Email", [phone.name]: "Phone"},
        },
        validators: {options: {options: ["email", phone.name]}},
    }),
    email: attribute("email", "${email}", {annotations: {inputType: "html5-email"}}),
    [phone.name]: attribute(phone.name, phone.displayName, {
        value: "+34600000000",
        annotations: {...phone.annotations, inputTypePattern: "\\+[0-9]{8,15}"},
    }),
    gender: attribute("gender", "Gender", {
        annotations: {inputType: "select-radiobuttons", default: "X"},
        validators: {options: {options: ["F", "M", "X"]}},
    }),
    dateOfBirth: attribute("dateOfBirth", "Date of birth", {
        annotations: {inputType: "html5-date", inputHelperTextAfter: "As in your passport."},
    }),
    embassy: attribute("embassy", "Embassy", {value: "Madrid PE"}),
    internal: attribute("internal", "Internal", {annotations: {hidden: "true"}}),
    secret: attribute("secret", "Secret"),
}

export const Default: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByRole("heading", {level: 1, name: "Register"})).toBeVisible()
        // Only the voting portal's enrollment has steps.
        await expect(canvas.queryByRole("progressbar")).toBeNull()
        const form = canvasElement.querySelector<HTMLFormElement>("#kc-register-form")!
        await expect(form).toHaveAttribute("method", "post")
        const names = [...form.querySelectorAll<HTMLInputElement>("input")].map(({name}) => name)
        await expect(names).toEqual([
            "username",
            "password",
            "password-confirm",
            "email",
            "firstName",
            "lastName",
        ])
        await expect(canvas.getByLabelText(/^Username/)).toBeRequired()
        await expect(canvas.getByLabelText(/^Password/)).toHaveAttribute(
            "autocomplete",
            "new-password"
        )
        const submit = canvas.getByRole("button", {name: "Register"})
        await expect(submit).toHaveAttribute("type", "submit")
        await expect(submit.closest("#kc-form-buttons")?.parentElement).toBe(form)
        await expectStickyActions(submit)
        await expect(canvas.getByRole("link", {name: /Back to Login/})).toBeVisible()
    },
}

// As voters see it: the first step of the enrollment.
export const Voting: Story = {
    args: {kcContext: {themeName: "sequent-ui-voting"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText("Step 1 of 4 · Your details")).toBeVisible()
        await expect(canvas.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "1")
    },
}

// Keycloak's own registration form: one password field, no enrollment around it.
export const KeycloakRegistration: Story = {
    args: {
        kcContext: {
            themeName: "sequent-ui-voting",
            sequentRegistration: {formMode: undefined},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByRole("progressbar")).toBeNull()
        await expect(canvas.getByLabelText(/^Password/)).toBeVisible()
        await expect(canvasElement.querySelector("#password-confirm")).toBeNull()
        await expect(canvas.queryByRole("link", {name: /Back to Login/})).toBeNull()
    },
}

export const PasswordVisibility: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const password = await canvas.findByLabelText(/^Password/)
        await userEvent.type(password, "synthetic")
        await expect(password).toHaveAttribute("type", "password")
        const [show] = canvas.getAllByRole("button", {name: "Show password"})
        await userEvent.click(show)
        await expect(password).toHaveAttribute("type", "text")
        await userEvent.click(canvas.getByRole("button", {name: "Hide password"}))
        await expect(password).toHaveAttribute("type", "password")
    },
}

export const CredentialFirst: Story = {
    args: {
        kcContext: {
            sequentRegistration: {credentialFieldPosition: CredentialFieldPosition.First},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const [first] = canvasElement.querySelectorAll("#kc-register-form input")
        await expect(first).toHaveAttribute("name", "password")
        await expect(first).toHaveFocus()
    },
}

export const NoPassword: Story = {
    args: {kcContext: {passwordRequired: false}},
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("#password")).toBeNull()
    },
}

export const ServerErrors: Story = {
    args: {
        kcContext: {
            message: {type: "error", summary: "Fix the fields below."},
            messagesPerField: {
                exists: (field: string) => field === "global",
                existsError: (...fields: string[]) =>
                    fields.some((field) => ["email", "password-confirm"].includes(field)),
                get: (field: string) =>
                    field === "email" ? "Invalid email address." : "Passwords don't match.",
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("alert")).toHaveTextContent("Fix the fields below.")
        const email = canvas.getByLabelText(/^Email/)
        await expect(email).toHaveAttribute("aria-invalid", "true")
        await expect(email).toHaveAccessibleDescription("Invalid email address.")
        await expect(canvasElement.querySelector("#password-confirm")).toHaveAccessibleDescription(
            "Passwords don't match."
        )
        await expect(canvasElement.querySelector("#input-error-username")).toBeNull()
    },
}

type PhoneOptions = {hiddenInput: () => {phone: string; country: string}}

// Stand-ins for the scripts the theme's resources serve in Keycloak.
function installWidgets() {
    const destroyed: string[] = []
    window.zxcvbn = (password) => ({score: Math.min(4, Math.floor(password.length / 3))})
    window.intlTelInput = (input, options) => {
        const names = (options as PhoneOptions).hiddenInput()
        const wrapper = document.createElement("div")
        wrapper.className = "iti"
        input.before(wrapper)
        const number = document.createElement("input")
        number.type = "hidden"
        number.name = names.phone
        wrapper.append(input, number)
        const widget = {
            getNumber: () => input.value,
            destroy: () => {
                destroyed.push(names.phone)
                wrapper.before(input)
                wrapper.remove()
            },
        }
        input.form?.addEventListener("submit", () => (number.value = widget.getNumber()))
        return widget
    }
    return () => {
        delete window.zxcvbn
        delete window.intlTelInput
    }
}

export const ProfileAnnotations: Story = {
    args: {
        kcContext: {
            profile: {attributesByName: annotated},
            sequentRegistration: {hiddenAttributes: ["secret"], lockedAttributes: ["embassy"]},
        },
    },
    beforeEach: installWidgets,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})

        // Hidden by the authenticator or by its annotation.
        await expect(canvas.queryByLabelText("Secret")).toBeNull()
        await expect(canvas.queryByLabelText("Internal")).toBeNull()

        const username = canvas.getByLabelText(/^Username/)
        await expect(username).toHaveStyle({textTransform: "uppercase"})
        await expect(username).toHaveAttribute("autocomplete", "off")

        // The password follows the username, with what its annotations ask for.
        const password = canvas.getByLabelText(/^Password \*/)
        await expect(password).toHaveAccessibleDescription("At least 8 characters.")
        const strength = canvas.getByRole("progressbar", {name: "Password strength"})
        await expect(strength).toHaveAttribute("aria-valuenow", "0")
        await userEvent.type(password, "synthetic-password")
        await expect(strength).toHaveAttribute("aria-valuenow", "100")
        await expect(strength).toHaveAttribute("data-level", "STRONG")

        const select = canvas.getByLabelText("Synthetic colour")
        await expect(select).toHaveAccessibleDescription("Synthetic helper text")
        await expect([...(select as HTMLSelectElement).options].map(({value}) => value)).toEqual([
            "",
            "red",
            "green",
        ])

        // Choosing a country narrows the posts to its own and picks the first.
        const post = canvas.getByLabelText("Post") as HTMLSelectElement
        await expect(post.options).toHaveLength(4)
        await userEvent.selectOptions(canvas.getByLabelText("Country"), "PH")
        await expect([...post.options].map(({text}) => text)).toEqual(["", "Manila", "Cebu"])
        await expect(post).toHaveValue("PH-manila")

        // The contact options lock the fields they name.
        const contact = within(canvas.getByRole("group", {name: /^Contact me by/}))
        const email = canvasElement.querySelector<HTMLInputElement>("#email")!
        const number = canvasElement.querySelector<HTMLInputElement>("input[type='tel']")!
        await expect(email).toBeRequired()
        await expect(email).not.toHaveAttribute("readonly")
        await expect(number).toHaveAttribute("readonly")
        await expect(number).toHaveValue("")
        await userEvent.type(email, "voter@example.test")
        await userEvent.click(contact.getByLabelText("Email"))
        await expect(email).toHaveAttribute("readonly")
        await expect(email).toHaveValue("")
        // None is checked: the first one carries the group's requirement.
        const first = contact.getByLabelText<HTMLInputElement>("Email")
        await expect(first.required).toBe(true)
        await userEvent.click(contact.getByLabelText("Phone"))
        await expect(number).not.toHaveAttribute("readonly")
        await expect(number).toBeRequired()
        await expect(first.required).toBe(false)

        // The widget posts the number under the attribute's name.
        await expect(number).toHaveAttribute("name", "synthetic-phone-input")
        await expect(number.closest(".iti")).not.toBeNull()
        await expect(number).not.toHaveAttribute("pattern")
        await userEvent.type(number, "600")
        await expect(number.validity.customError).toBe(true)
        await userEvent.clear(number)
        await userEvent.type(number, "+34600000000")
        await expect(number.validity.valid).toBe(true)

        await expect(canvas.getByRole("radio", {name: "X"})).toBeChecked()
        await userEvent.click(canvas.getByRole("radio", {name: "F"}))
        await expect(canvas.getByRole("radio", {name: "X"})).not.toBeChecked()

        const birth = canvas.getByLabelText("Date of birth")
        await expect(birth).toHaveAttribute("type", "date")
        await expect(birth).toHaveAttribute("max", "9999-12-31")
        await expect(birth).toHaveAccessibleDescription("As in your passport.")

        // From a read-only login hint: posted as it came.
        const embassy = canvas.getByLabelText("Embassy")
        await expect(embassy).toHaveAttribute("readonly")
        await expect(embassy).toHaveValue("Madrid PE")
    },
}

// Without the theme's scripts the fields still work: a plain phone input
// under the attribute's name, with its pattern.
export const WithoutWidgets: Story = {
    args: {kcContext: {profile: {attributesByName: annotated}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const number = canvasElement.querySelector<HTMLInputElement>("input[type='tel']")!
        await waitFor(() => expect(number).toHaveAttribute("name", "synthetic-phone"))
        await expect(number).toHaveAttribute("pattern", "\\+[0-9]{8,15}")
        await expect(canvas.queryByRole("progressbar", {name: "Password strength"})).toBeNull()
    },
}

export const Spanish: Story = {
    args: {locale: "es", kcContext: {themeName: "sequent-ui-voting"}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText("Paso 1 de 4 · Sus datos")).toBeVisible()
    },
}

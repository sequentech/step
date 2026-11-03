// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {KEYCLOAK_PROFILE_ATTRIBUTES} from "@sequentech/ui-test-kit/fixtures/keycloak"
import {createKcPageStory} from "../KcPageStory"
import {CredentialFieldPosition, MessageChannel} from "../KcContext"
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
        await expect(canvasElement.querySelector("#login-custom-css")).toBeNull()
    },
}

// A realm that restyles its login pages through the loginCustomCss localization text.
export const RealmCustomCss: Story = {
    args: {
        kcContext: {
            themeName: "sequent-ui-voting",
            sequent: {loginCustomCss: ".sequent-auth .auth-title { color: rgb(1, 2, 3); }"},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const title = await canvas.findByRole("heading", {level: 1})
        await expect(getComputedStyle(title).color).toBe("rgb(1, 2, 3)")
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

// The enrollment's channel choice: two synthetic Posts, one offering every channel, one fewer.
const CHANNEL_CHOICE = {
    themeName: "sequent-ui-voting" as const,
    profile: {
        attributesByName: {
            post: attribute("post", "Post", {
                annotations: {
                    inputType: "select",
                    inputOptionLabels: {
                        "post-a": "Synthetic Embassy A",
                        "post-b": "Synthetic Consulate B",
                    },
                },
                validators: {options: {options: ["post-a", "post-b"]}},
            }),
        },
    },
    messagingPostAttribute: "post",
    // Before a Post is chosen: the channels every Post offers, in Keycloak's order.
    messagingChannels: [MessageChannel.Email, MessageChannel.Sms],
    messagingChannelsByPost: {
        "post-a": [
            MessageChannel.Email,
            MessageChannel.Sms,
            MessageChannel.Messenger,
            MessageChannel.Viber,
            MessageChannel.WhatsApp,
        ],
        "post-b": [MessageChannel.Sms, MessageChannel.Email, MessageChannel.WhatsApp],
    },
    messagingOrganization: "Synthetic Commission",
    messagingConsentVersion: "1",
    messagingPage: "Synthetic Page",
}

const channelValues = (canvasElement: HTMLElement) =>
    [
        ...within(canvasElement)
            .getByRole("radiogroup", {name: /Get codes and updates by/})
            .querySelectorAll<HTMLInputElement>("input[type='radio']"),
    ].map(({value}) => value)

const choosePost = async (canvasElement: HTMLElement, post: string) =>
    userEvent.selectOptions(within(canvasElement).getByLabelText(/^Post/), post)

/** The list follows the Post the voter picks, always in the ticket's channel order. */
export const ChannelChoiceFollowsThePost: Story = {
    args: {kcContext: CHANNEL_CHOICE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(channelValues(canvasElement)).toEqual(["SMS", "EMAIL"])
        await choosePost(canvasElement, "post-a")
        await expect(channelValues(canvasElement)).toEqual([
            "WHATSAPP",
            "VIBER",
            "MESSENGER",
            "SMS",
            "EMAIL",
        ])
        await expect(canvas.queryByText(/can reach you/)).toBeNull()
        await choosePost(canvasElement, "post-b")
        await expect(channelValues(canvasElement)).toEqual(["WHATSAPP", "SMS", "EMAIL"])
        await expect(
            canvas.getByText("The ways Synthetic Consulate B can reach you.")
        ).toBeVisible()
        // Every channel has the shared icon.
        await expect(
            canvasElement.querySelectorAll(".auth-channel-option [data-channel]")
        ).toHaveLength(3)
    },
}

/** Only the chosen channel's number and consent are on the form; there is no notice choice. */
export const ChannelChoiceNumberAndConsent: Story = {
    args: {kcContext: CHANNEL_CHOICE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await choosePost(canvasElement, "post-a")
        await userEvent.click(canvas.getByRole("radio", {name: "WhatsApp"}))
        const number = canvas.getByLabelText(/^WhatsApp number/)
        await expect(number).toBeRequired()
        await expect(number).toHaveAttribute("name", "sequent.whatsapp-number")
        await expect(number).toHaveAccessibleDescription(
            "Synthetic Commission will send codes and election notices to this number."
        )
        const consent = canvas.getByRole("checkbox", {
            name: "I agree that Synthetic Commission sends me codes and election notices by WhatsApp.",
        })
        await expect(consent).toBeRequired()
        await expect(consent).toHaveAttribute("value", "1:WHATSAPP")
        await userEvent.type(number, "+15550000999")
        await userEvent.click(consent)

        // Another channel replaces the number and the consent: nothing typed for WhatsApp is sent.
        await userEvent.click(canvas.getByRole("radio", {name: "Viber"}))
        await expect(canvas.queryByLabelText(/^WhatsApp number/)).toBeNull()
        await expect(canvas.getByLabelText(/^Viber number/)).toHaveValue("")
        const form = canvasElement.querySelector<HTMLFormElement>("#kc-register-form")!
        const data = new FormData(form)
        await expect(data.get("sequent.otp-channel")).toBe("VIBER")
        await expect(data.get("sequent.whatsapp-number")).toBeNull()
        await expect(data.getAll("sequent.message-consent")).toEqual([])
        await expect(data.get("sequent.notice-channel")).toBeNull()
        await expect(canvas.queryByText(/notices to my/)).toBeNull()
    },
}

/** A channel the new Post doesn't offer is no longer chosen, and its fields go. */
export const ChannelChoiceDropsAChannelThePostLacks: Story = {
    args: {kcContext: CHANNEL_CHOICE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await choosePost(canvasElement, "post-a")
        await userEvent.click(canvas.getByRole("radio", {name: "Viber"}))
        await expect(canvas.getByLabelText(/^Viber number/)).toBeVisible()
        await choosePost(canvasElement, "post-b")
        await expect(canvas.queryByRole("radio", {checked: true})).toBeNull()
        await expect(canvas.queryByLabelText(/^Viber number/)).toBeNull()
    },
}

/** Messenger needs no number: the voter connects the Page's chat on the next step. */
export const ChannelChoiceMessenger: Story = {
    args: {kcContext: CHANNEL_CHOICE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await choosePost(canvasElement, "post-a")
        await userEvent.click(canvas.getByRole("radio", {name: "Facebook Messenger"}))
        await expect(canvasElement.querySelector("#messenger-connect-help")).toHaveTextContent(
            "Connect Messenger Open Synthetic Page and tap Get Started to get your code."
        )
        await expect(canvasElement.querySelector("input[type='tel']")).toBeNull()
        await expect(
            canvas.getByRole("checkbox", {
                name: "I agree that Synthetic Commission sends me codes and election notices by Facebook Messenger.",
            })
        ).toHaveAttribute("value", "1:MESSENGER")
    },
}

/** A Post with one channel has it chosen already. */
export const ChannelChoiceSingleChannel: Story = {
    args: {
        kcContext: {
            ...CHANNEL_CHOICE,
            messagingChannels: [MessageChannel.Email],
            messagingChannelsByPost: {},
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("radio", {name: "Email"})).toBeChecked()
    },
}

export const ChannelChoiceErrors: Story = {
    args: {
        kcContext: {
            ...CHANNEL_CHOICE,
            messagesPerField: {
                exists: () => false,
                existsError: (...fields: string[]) => fields.includes("sequent.otp-channel"),
                get: () => "Choose how to get your verification code.",
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const group = canvas.getByRole("radiogroup", {name: /Get codes and updates by/})
        await expect(group).toHaveAttribute("aria-invalid", "true")
        await expect(group).toHaveAccessibleDescription("Choose how to get your verification code.")
    },
}

export const ChannelChoiceSpanish: Story = {
    args: {locale: "es", kcContext: CHANNEL_CHOICE},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByRole("radiogroup", {name: /Recibir códigos y avisos por/})
        ).toBeVisible()
        await choosePost(canvasElement, "post-b")
        await expect(
            canvas.getByText("Las formas en que Synthetic Consulate B puede contactarle.")
        ).toBeVisible()
    },
}

/** Shown again with errors, the form keeps the channel, number and consent the voter sent. */
export const ChannelChoiceKeepsWhatWasSubmitted: Story = {
    args: {
        kcContext: {
            ...CHANNEL_CHOICE,
            profile: {
                attributesByName: {
                    post: {...CHANNEL_CHOICE.profile.attributesByName.post, values: ["post-b"]},
                },
            },
            messagingFormData: {
                "sequent.otp-channel": "WHATSAPP",
                "sequent.whatsapp-number": "+15550000999",
                "sequent.message-consent": "1:WHATSAPP",
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("radio", {name: "WhatsApp"})).toBeChecked()
        await expect(canvas.getByLabelText(/^WhatsApp number/)).toHaveValue("+15550000999")
        await expect(canvas.getByRole("checkbox", {name: /by WhatsApp\./})).toBeChecked()
    },
}

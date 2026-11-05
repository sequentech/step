// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, spyOn, userEvent, waitFor, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"
import {MessageChannel} from "../KcContext"

const {KcPageStory} = createKcPageStory({pageId: "registration-manual-finish.ftl"})
const {KcPageStory: Enrolled} = createKcPageStory({pageId: "registration-finish.ftl"})
const {KcPageStory: Rejected} = createKcPageStory({pageId: "registration-rejected-finish.ftl"})
const {KcPageStory: Validated} = createKcPageStory({pageId: "message-finish.ftl"})

const meta = {
    title: "Keycloak/Enrollment finish",
    component: KcPageStory,
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

async function expectFinished(canvasElement: HTMLElement, outcome: string) {
    const canvas = within(canvasElement)
    // Every step of the enrollment is done.
    const progress = canvas.getByRole("progressbar", {name: "Enrollment progress"})
    await expect(progress).toHaveAttribute("aria-valuenow", "4")
    await expect(progress).toHaveAttribute("aria-valuetext", outcome)
    await expect(progress.querySelectorAll(".done")).toHaveLength(4)
    await expect(canvas.getByText(outcome)).toBeVisible()
    await expect(canvas.getByRole("link", {name: "Click here"})).toHaveAttribute(
        "id",
        "loginContinueLink"
    )
}

export const ManualVerification: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Manual Verification required"})
        ).toBeVisible()
        await expectFinished(canvasElement, "Enrollment under review")
        await expect(canvas.getByText(/some required information is missing/)).toBeVisible()
        const mismatches = within(
            canvas.getByRole("region", {
                name: "The following fields did not match in the registry:",
            })
        )
        await expect(mismatches.getByText("SANTOS")).toBeVisible()
        // A field the voter left empty.
        await expect(mismatches.getByText("Date of birth")).toBeVisible()
        await expect(mismatches.getByText("Empty")).toBeVisible()
        await expect(
            canvas.getByText(/Please expect to receive SMS or email for further instructions/)
        ).toBeVisible()
    },
}

// No voter in the registry matched: no field was compared, there's none to list.
export const NoMatchingVoter: Story = {
    args: {kcContext: {enrollmentOutcome: {reason: "NO_VOTER"}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByText(/does not match any existing user/)).toBeVisible()
        await expect(canvas.queryByRole("region")).toBeNull()
    },
}

export const Congratulations: Story = {
    render: (args) => <Enrolled locale={args.locale} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Congratulations!"})
        ).toBeVisible()
        await expectFinished(canvasElement, "Enrollment complete")
        await expect(canvas.getByText(/You are successfully validated/)).toBeVisible()
        await expect(canvas.queryByRole("region")).toBeNull()
    },
}

export const Disapproved: Story = {
    render: (args) => <Rejected locale={args.locale} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {level: 1, name: "Application Disapproved"})
        ).toBeVisible()
        await expectFinished(canvasElement, "Enrollment not approved")
        await expect(canvas.getByText(/already completed the enrollment process/)).toBeVisible()
    },
}

export const AlreadyValidated: Story = {
    render: (args) => <Validated locale={args.locale} />,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1, name: "Register"})
        await expectFinished(canvasElement, "Enrollment")
        await expect(canvas.getByText(/Your record has already been validated/)).toBeVisible()
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(
            await canvas.findByRole("heading", {
                level: 1,
                name: "Se requiere verificación manual",
            })
        ).toBeVisible()
        await expect(canvas.getByText("Inscripción en revisión")).toBeVisible()
        await expect(canvas.getByText("Vacío")).toBeVisible()
    },
}

// As voters see it, in the voting portal's theme, for walkthroughs.
export const Voting: Story = {
    args: {kcContext: {themeName: "sequent-ui-voting"}},
}

/** The review page names where the result goes: the voter's one channel and its masked contact. */
export const PendingResultByWhatsApp: Story = {
    args: {
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            noticeChannel: MessageChannel.WhatsApp,
            noticeAddress: "+155 ••• ••• 999",
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("We will send the result to your WhatsApp, +155 ••• ••• 999.")
        ).toBeVisible()
        // The channel line replaces the page's "SMS or email" text.
        await expect(canvas.queryByText(/SMS or email/)).toBeNull()
    },
}

export const PendingResultByMessenger: Story = {
    args: {
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            noticeChannel: MessageChannel.Messenger,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("We will send the result to your Facebook Messenger.")
        ).toBeVisible()
    },
}

export const PendingResultSpanish: Story = {
    args: {
        locale: "es",
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            noticeChannel: MessageChannel.Sms,
            noticeAddress: "+155 ••• ••• 999",
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(
            canvas.getByText("Le enviaremos el resultado a su SMS, +155 ••• ••• 999.")
        ).toBeVisible()
    },
}

/** An unknown channel from an older page is never shown. */
export const PendingResultUnknownChannel: Story = {
    args: {
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            noticeChannel: "PIGEON" as MessageChannel,
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("#notice-channel")).toBeNull()
        await expect(canvas.getByText(/SMS or email/)).toBeVisible()
    },
}

/** The review page gives a reference that support can look the application up by. */
export const PendingWithReference: Story = {
    args: {
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            noticeChannel: MessageChannel.WhatsApp,
            noticeAddress: "+155 ••• ••• 999",
            supportReference: "Q5KW8RFX",
        },
    },
    beforeEach: () => {
        const writeText = spyOn(navigator.clipboard, "writeText").mockResolvedValue(undefined)
        return () => writeText.mockRestore()
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const reference = canvasElement.querySelector("#support-reference")!
        await expect(reference).toHaveTextContent(
            "Need help? Contact support and give them this reference code: Q5KW-8RFX"
        )
        await userEvent.click(canvas.getByRole("button", {name: "Copy"}))
        await expect(navigator.clipboard.writeText).toHaveBeenCalledWith("Q5KW-8RFX")
        await waitFor(() => expect(canvas.getByRole("status")).toHaveTextContent("Copied"))
    },
}

export const PendingWithReferenceSpanish: Story = {
    args: {
        locale: "es",
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            supportReference: "Q5KW-8RFX",
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("#support-reference")).toHaveTextContent(
            "¿Necesita ayuda? Contacte con soporte y deles este código de referencia: Q5KW-8RFX"
        )
        await expect(canvas.getByRole("button", {name: "Copiar"})).toBeVisible()
    },
}

/** Without a clipboard the reference is still shown, to copy by hand; nothing breaks. */
export const PendingReferenceWithoutClipboard: Story = {
    args: {
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            supportReference: "Q5KW8RFX",
        },
    },
    beforeEach: () => {
        // The test runner may give navigator its own clipboard: hide whichever is there.
        const own = Object.getOwnPropertyDescriptor(navigator, "clipboard")
        Object.defineProperty(navigator, "clipboard", {configurable: true, value: undefined})
        return () => {
            if (own) {
                Object.defineProperty(navigator, "clipboard", own)
            } else {
                delete (navigator as {clipboard?: Clipboard}).clipboard
            }
        }
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("#support-reference")).toHaveTextContent(
            "Q5KW-8RFX"
        )
        await expect(canvas.queryByRole("button", {name: "Copy"})).not.toBeInTheDocument()
    },
}

/** A copy the browser refuses is said, so the voter copies the reference by hand. */
export const PendingReferenceCopyRefused: Story = {
    args: {
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            supportReference: "Q5KW8RFX",
        },
    },
    beforeEach: () => {
        const writeText = spyOn(navigator.clipboard, "writeText").mockRejectedValue(
            new DOMException("Denied", "NotAllowedError")
        )
        return () => writeText.mockRestore()
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await userEvent.click(canvas.getByRole("button", {name: "Copy"}))
        await waitFor(() => expect(canvas.getByRole("status")).toHaveTextContent(/Couldn’t copy/))
    },
}

export const PendingWithoutReference: Story = {
    args: {kcContext: {enrollmentOutcome: {mismatchedFields: []}}},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("#support-reference")).toBeNull()
        await expect(canvas.queryByRole("button", {name: "Copy"})).not.toBeInTheDocument()
    },
}

/** Only the reference's own shape is shown: anything else is ignored. */
export const PendingReferenceIgnoresOtherValues: Story = {
    args: {
        kcContext: {
            enrollmentOutcome: {mismatchedFields: []},
            supportReference: "<b>voter@example.test</b>",
        },
    },
    play: async ({canvasElement}) => {
        await within(canvasElement).findByRole("heading", {level: 1})
        await expect(canvasElement.querySelector("#support-reference")).toBeNull()
    },
}

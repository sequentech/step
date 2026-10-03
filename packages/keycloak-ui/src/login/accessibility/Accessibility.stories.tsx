// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, userEvent, waitFor, within} from "storybook/test"
import {createKcPageStory} from "../KcPageStory"
import {EAudioInstructionsPolicy, EVoterAccessibilitySettingsPolicy} from "../KcContext"
import {getAccessibilityCopy} from "./copy"

const {KcPageStory} = createKcPageStory({pageId: "login.ftl"})

const meta = {
    title: "Keycloak/Accessibility",
    component: KcPageStory,
    args: {
        kcContext: {
            sequent: {
                voterAccessibilitySettingsPolicy: EVoterAccessibilitySettingsPolicy.ENABLED,
                audioInstructionsPolicy: EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED,
            },
        },
    },
} satisfies Meta<typeof KcPageStory>

export default meta

type Story = StoryObj<typeof meta>

const root = document.documentElement

export const ChangeAndResetSettings: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const button = within(canvas.getByRole("banner")).getByRole("button", {
            name: "Accessibility",
        })
        await expect(button).toHaveAttribute("aria-haspopup", "dialog")
        await userEvent.click(button)

        const dialog = within(
            await within(document.body).findByRole("dialog", {name: "Accessibility settings"})
        )
        await userEvent.click(dialog.getByRole("radio", {name: "High contrast"}))
        await expect(root).toHaveAttribute("data-a11y-contrast", "high")
        await expect(dialog.getByRole("status")).toHaveTextContent("Contrast: High contrast")
        await userEvent.click(dialog.getByRole("radio", {name: "Larger"}))
        await expect(root).toHaveAttribute("data-a11y-text-size", "larger")
        await expect(document.cookie).toContain("USER_ACCESSIBILITY")

        await userEvent.click(dialog.getByRole("button", {name: "Reset settings"}))
        await expect(root).not.toHaveAttribute("data-a11y-contrast")
        await expect(root).not.toHaveAttribute("data-a11y-text-size")
        await expect(
            within(dialog.getByRole("radiogroup", {name: "Text size"})).getByRole("radio", {
                name: "Default",
            })
        ).toBeChecked()

        await userEvent.click(dialog.getByRole("button", {name: "Close"}))
        await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull())
        await expect(button).toHaveFocus()
    },
}

export const ReadTheInstructions: Story = {
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const instructions = within(canvas.getByRole("region", {name: "Audio instructions"}))
        const text = getAccessibilityCopy("en").instructions["login.ftl"]
        await expect(instructions.queryByText(text)).toBeNull()
        await userEvent.click(instructions.getByRole("button", {name: "Read the instructions"}))
        await expect(instructions.getByText(text)).toBeVisible()
        await expect(
            instructions.getByRole("button", {name: "Hide the instructions"})
        ).toHaveAttribute("aria-expanded", "true")
    },
}

/**
 * A browser whose voices finish loading while the page is still rendering: the list is empty on
 * the first read and the one "voiceschanged" event is gone before anything listens for it.
 */
const installVoicesLoadedEarly = () => {
    const original = Object.getOwnPropertyDescriptor(window, "speechSynthesis")
    let listening = false
    const speech = {
        getVoices: () => (listening ? [{lang: "en-US"}] : []),
        addEventListener: () => {
            listening = true
        },
        removeEventListener: () => undefined,
        speak: () => undefined,
        cancel: () => undefined,
        pause: () => undefined,
        resume: () => undefined,
    }
    Object.defineProperty(window, "speechSynthesis", {configurable: true, value: speech})
    return () => {
        if (original) {
            Object.defineProperty(window, "speechSynthesis", original)
        } else {
            Reflect.deleteProperty(window, "speechSynthesis")
        }
    }
}

export const ListenWhenVoicesLoadedEarly: Story = {
    beforeEach: installVoicesLoadedEarly,
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        const instructions = within(canvas.getByRole("region", {name: "Audio instructions"}))
        await expect(
            await instructions.findByRole("button", {name: "Listen to the instructions"})
        ).toBeVisible()
    },
}

export const Spanish: Story = {
    args: {locale: "es"},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.getByRole("button", {name: "Accesibilidad"})).toBeVisible()
        await expect(canvas.getByRole("region", {name: "Instrucciones en audio"})).toBeVisible()
    },
}

export const RecordingsOnlyHasNoLoginAudio: Story = {
    args: {
        kcContext: {sequent: {audioInstructionsPolicy: EAudioInstructionsPolicy.RECORDED}},
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByRole("region", {name: "Audio instructions"})).toBeNull()
    },
}

export const DisabledByDefault: Story = {
    args: {
        kcContext: {
            sequent: {
                voterAccessibilitySettingsPolicy: EVoterAccessibilitySettingsPolicy.DISABLED,
                audioInstructionsPolicy: EAudioInstructionsPolicy.DISABLED,
            },
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await canvas.findByRole("heading", {level: 1})
        await expect(canvas.queryByRole("button", {name: "Accessibility"})).toBeNull()
        await expect(canvas.queryByRole("region", {name: "Audio instructions"})).toBeNull()
    },
}

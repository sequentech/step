// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import type {Meta, StoryObj} from "@storybook/react-vite"
import {expect, fn, userEvent, within} from "storybook/test"
import {IvrEmulatorError} from "@sequentech/ui-essentials"
import {EmbedMessageType} from "../embed"
import {EmbeddedCall} from "../EmbeddedCall"
import {
    fakeCallConfig,
    fakeIvrEmulator,
    FakeLanguageMenuIvrDriver,
    FakePinIvrDriver,
} from "../fakeIvrEmulator"

/**
 * The telephone call `workbench/embed.html` places for a framing tool such as the Election
 * Architect's Call Emulator, over a stand-in for the IVR emulator. The real emulator is the
 * IVR Lambda compiled to WebAssembly and is served by the framing tool.
 */
const meta = {
    title: "Embedded voter preview/Telephone call",
    component: EmbeddedCall,
    args: {
        request: {
            config: fakeCallConfig(),
            emulatorUrl: "https://architect.example/wasm/ivr_emulator_wasm",
        },
        reply: fn(),
        load: async () => fakeIvrEmulator,
    },
} satisfies Meta<typeof EmbeddedCall>

export default meta
type Story = StoryObj<typeof meta>

/** The caller hears the greeting, presses a key and the call ends. */
export const Call: Story = {
    play: async ({canvasElement, args}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Press 1 to hear your ballot.")).toBeVisible()
        await userEvent.type(canvas.getByRole("textbox", {name: "Keys to press"}), "1")
        await userEvent.click(canvas.getByRole("button", {name: "Press these keys"}))
        await expect(await canvas.findByText("The call ended.")).toBeVisible()
        await expect(args.reply).toHaveBeenLastCalledWith({
            type: EmbedMessageType.CALLING,
            status: "Disconnected",
        })
    },
}

/** The framing tool's words, in its language. */
export const Translated: Story = {
    args: {
        request: {
            ...meta.args.request,
            labels: {
                input: "Teclas",
                placeholder: "Pulse {{keys}} en {{timeout}} s",
                or: "o",
                timeout: "Esperar",
                send: "Pulsar",
                disconnected: "La llamada terminó.",
            },
        },
    },
    play: async ({canvasElement}) => {
        const keypad = await within(canvasElement).findByRole("textbox", {name: "Teclas"})
        await expect(keypad).toHaveAttribute("placeholder", "Pulse 1 en 10 s")
    },
}

/**
 * A prompt that takes any digits, such as a PIN: the Lambda lists no keys for it, and the
 * hint says how many digits rather than "of" nothing.
 */
export const AnyDigits: Story = {
    args: {load: async () => ({IvrEmulatorDriver: FakePinIvrDriver})},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("Enter your 8-digit PIN.")).toBeVisible()
        const keypad = canvas.getByRole("textbox", {name: "Keys to press"})
        await expect(keypad).toHaveAttribute("placeholder", "Up to 8 digits, within 5s")
        await userEvent.type(keypad, "12345678")
        await userEvent.click(canvas.getByRole("button", {name: "Press these keys"}))
        await expect(await canvas.findByText("Press 1 to hear your ballot.")).toBeVisible()
        await expect(keypad).toHaveAttribute("placeholder", "Press 1, within 10s")
    },
}

/**
 * A language menu in two languages, which the Lambda speaks as SSML `<lang>` parts: the
 * transcript shows the words, with a badge for the part in another language, and none
 * of the markup.
 */
export const LanguageMenu: Story = {
    args: {load: async () => ({IvrEmulatorDriver: FakeLanguageMenuIvrDriver})},
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        const english = await canvas.findByText("For English, press 1")
        await expect(english).toHaveAttribute("lang", "en-US")
        await expect(canvas.getByText("Para español, pulse 2")).toHaveAttribute("lang", "es-ES")
        await expect(canvas.getByTitle("en-US")).toHaveTextContent("EN")
        await expect(canvasElement.textContent).not.toMatch(/<lang|xml:lang|<\/?speak/)
        // The Lambda lists the menu's keys as they come ("2,1"); the hint orders them.
        await expect(canvas.getByRole("textbox", {name: "Keys to press"})).toHaveAttribute(
            "placeholder",
            "Press 1 or 2, within 5s"
        )
    },
}

/** A caller the flow's blacklist refuses. */
export const BlockedNumber: Story = {
    args: {
        request: {
            ...meta.args.request,
            config: fakeCallConfig({blacklisted_numbers: ["+1234567890"]}),
        },
    },
    play: async ({canvasElement}) => {
        const canvas = within(canvasElement)
        await expect(await canvas.findByText("This number cannot vote by telephone.")).toBeVisible()
        await expect(canvas.getByText("The call ended.")).toBeVisible()
    },
}

/** No emulator is served where the framing tool said: not a failure. */
export const NoEmulator: Story = {
    args: {
        load: async () => {
            throw new IvrEmulatorError("fetch", "response was 404")
        },
    },
    play: async ({canvasElement, args}) => {
        await expect(
            await within(canvasElement).findByText(/No telephone emulator is served at/)
        ).toBeVisible()
        await expect(args.reply).toHaveBeenLastCalledWith({
            type: EmbedMessageType.CALLING,
            status: "absent",
        })
    },
}

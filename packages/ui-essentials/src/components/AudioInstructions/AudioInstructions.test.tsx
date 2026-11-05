/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {act, render, screen} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {createInstance} from "i18next"
import {I18nextProvider} from "react-i18next"
import {ThemeProvider} from "@mui/material/styles"
import {EAudioInstructionsPolicy} from "@sequentech/ui-core"
import english from "../../../../ui-core/src/translations/en"
import theme from "../../services/theme"
import AudioInstructions, {AudioInstructionsProps} from "./AudioInstructions"

class FakeUtterance {
    lang = ""
    voice: unknown = null
    onend: (() => void) | null = null
    onerror: (() => void) | null = null
    constructor(public text: string) {}
}

const installSpeech = (voices: {lang: string}[]) => {
    const speech = {
        getVoices: jest.fn(() => voices),
        speak: jest.fn(),
        cancel: jest.fn(),
        pause: jest.fn(),
        resume: jest.fn(),
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
    }
    Object.defineProperty(window, "speechSynthesis", {configurable: true, value: speech})
    Object.defineProperty(window, "SpeechSynthesisUtterance", {
        configurable: true,
        value: FakeUtterance,
    })
    return speech
}

const text = "Choose your candidates, then select Next."

const view = async (props: Partial<AudioInstructionsProps>) => {
    const i18n = createInstance()
    await i18n.init({lng: "en", resources: {en: {translation: english.translations}}})
    const element = (extra: Partial<AudioInstructionsProps> = {}) => (
        <I18nextProvider i18n={i18n}>
            <ThemeProvider theme={theme}>
                <AudioInstructions
                    policy={EAudioInstructionsPolicy.RECORDED_OR_SYNTHESIZED}
                    text={text}
                    language="en"
                    {...props}
                    {...extra}
                />
            </ThemeProvider>
        </I18nextProvider>
    )
    const rendered = render(element())
    return {
        ...rendered,
        update: (extra: Partial<AudioInstructionsProps>) => rendered.rerender(element(extra)),
    }
}

let play: jest.SpyInstance
let pause: jest.SpyInstance

beforeEach(() => {
    play = jest.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined)
    pause = jest.spyOn(HTMLMediaElement.prototype, "pause").mockImplementation(() => undefined)
})

afterEach(() => {
    jest.restoreAllMocks()
    Reflect.deleteProperty(window, "speechSynthesis")
    Reflect.deleteProperty(window, "SpeechSynthesisUtterance")
})

it("speaks the screen's text in the voter's language and never starts by itself", async () => {
    const speech = installSpeech([{lang: "es-ES"}, {lang: "en-GB"}])
    const user = userEvent.setup()
    await view({})

    expect(screen.getByRole("region", {name: "Audio instructions"})).toBeVisible()
    expect(speech.speak).not.toHaveBeenCalled()

    await user.click(screen.getByRole("button", {name: "Listen to the instructions"}))
    expect(speech.speak).toHaveBeenCalledTimes(1)
    const utterance = speech.speak.mock.calls[0][0] as FakeUtterance
    expect(utterance.text).toBe(text)
    expect(utterance.lang).toBe("en-GB")
    expect(screen.getByRole("status")).toHaveTextContent("Playing the instructions")

    await user.click(screen.getByRole("button", {name: "Pause the instructions"}))
    expect(speech.pause).toHaveBeenCalled()
    expect(screen.getByRole("status")).toHaveTextContent("Instructions paused")

    await user.click(screen.getByRole("button", {name: "Resume the instructions"}))
    expect(speech.resume).toHaveBeenCalled()

    await user.click(screen.getByRole("button", {name: "Stop the instructions"}))
    expect(speech.cancel).toHaveBeenCalled()
    expect(screen.getByRole("status")).toHaveTextContent("Instructions stopped")
    expect(screen.getByRole("button", {name: "Listen to the instructions"})).toBeVisible()
})

it("returns to the start when the speech ends", async () => {
    const speech = installSpeech([{lang: "en-US"}])
    const user = userEvent.setup()
    await view({})
    await user.click(screen.getByRole("button", {name: "Listen to the instructions"}))
    act(() => (speech.speak.mock.calls[0][0] as FakeUtterance).onend?.())
    expect(screen.getByRole("button", {name: "Listen to the instructions"})).toBeVisible()
    expect(screen.queryByRole("button", {name: "Stop the instructions"})).not.toBeInTheDocument()
})

it("finds the voices that finished loading before it started listening for them", async () => {
    const speech = installSpeech([{lang: "en-US"}])
    // The browser's list is empty on the first read and its one "voiceschanged" is already gone.
    speech.getVoices.mockReturnValueOnce([])
    await view({})

    expect(speech.addEventListener).toHaveBeenCalledWith("voiceschanged", expect.any(Function))
    expect(screen.getByRole("button", {name: "Listen to the instructions"})).toBeVisible()
})

it("plays the recording instead of synthesised speech when there is one", async () => {
    const speech = installSpeech([{lang: "en-US"}])
    const user = userEvent.setup()
    const {container} = await view({recordingUrl: "https://files.invalid/ballot-en.mp3"})

    await user.click(screen.getByRole("button", {name: "Listen to the instructions"}))
    expect(play).toHaveBeenCalledTimes(1)
    expect(speech.speak).not.toHaveBeenCalled()
    expect(container.querySelector("audio")).toHaveAttribute(
        "src",
        "https://files.invalid/ballot-en.mp3"
    )

    await user.click(screen.getByRole("button", {name: "Pause the instructions"}))
    expect(pause).toHaveBeenCalled()
})

it("stops when the voter leaves the screen or changes language", async () => {
    const speech = installSpeech([{lang: "en-US"}, {lang: "es-ES"}])
    const user = userEvent.setup()
    const {update, unmount} = await view({})

    await user.click(screen.getByRole("button", {name: "Listen to the instructions"}))
    speech.cancel.mockClear()
    update({language: "es", text: "Elija sus candidatos."})
    expect(speech.cancel).toHaveBeenCalled()
    expect(screen.getByRole("button", {name: "Listen to the instructions"})).toBeVisible()

    await user.click(screen.getByRole("button", {name: "Listen to the instructions"}))
    expect((speech.speak.mock.calls[1][0] as FakeUtterance).lang).toBe("es-ES")
    speech.cancel.mockClear()
    unmount()
    expect(speech.cancel).toHaveBeenCalled()
})

it("offers the text to read where there is neither a recording nor a voice", async () => {
    installSpeech([{lang: "es-ES"}])
    const user = userEvent.setup()
    await view({language: "tl"})

    expect(screen.queryByRole("button", {name: "Listen to the instructions"})).toBeNull()
    const toggle = screen.getByRole("button", {name: "Read the instructions"})
    expect(toggle).toHaveAttribute("aria-expanded", "false")
    expect(screen.queryByText(text)).not.toBeInTheDocument()

    await user.click(toggle)
    expect(screen.getByRole("button", {name: "Hide the instructions"})).toHaveAttribute(
        "aria-expanded",
        "true"
    )
    expect(screen.getByText(text)).toBeVisible()
})

it("works in a browser without speech synthesis", async () => {
    await view({})
    expect(screen.queryByRole("button", {name: "Listen to the instructions"})).toBeNull()
    expect(screen.getByRole("button", {name: "Read the instructions"})).toBeVisible()
})

it.each([
    ["the policy is disabled", {policy: EAudioInstructionsPolicy.DISABLED}],
    ["the event has no policy", {policy: undefined}],
    [
        "only recordings are allowed and the screen has none",
        {policy: EAudioInstructionsPolicy.RECORDED},
    ],
    ["the screen has no text", {text: ""}],
])("renders nothing when %s", async (_case, props) => {
    installSpeech([{lang: "en-US"}])
    const {container} = await view(props)
    expect(container).toBeEmptyDOMElement()
})

/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import "@testing-library/jest-dom"
import {AudioInstructionsFields, withAudioInstructions} from "./AudioInstructionsFields"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../../ui-core/src/services/audioInstructions")
)
jest.mock("react-i18next", () => ({
    useTranslation: () => ({t: (key: string) => key}),
}))

const SCREEN = "materials.audioInstructions.screenLabel"
const LANGUAGE = "materials.audioInstructions.languageLabel"

it("is offered only for audio files", () => {
    const {container, rerender} = render(
        <AudioInstructionsFields kind="application/pdf" languages={["en"]} onChange={jest.fn()} />
    )
    expect(container).toBeEmptyDOMElement()

    rerender(<AudioInstructionsFields kind={undefined} languages={["en"]} onChange={jest.fn()} />)
    expect(container).toBeEmptyDOMElement()

    rerender(<AudioInstructionsFields kind="audio/mpeg" languages={["en"]} onChange={jest.fn()} />)
    expect(screen.getByRole("combobox", {name: SCREEN})).toBeVisible()
    expect(screen.queryByRole("combobox", {name: LANGUAGE})).not.toBeInTheDocument()
})

it("assigns a screen in the event's first language, then lets the language change", async () => {
    const onChange = jest.fn()
    const {rerender} = render(
        <AudioInstructionsFields kind="audio/mpeg" languages={["tl", "en"]} onChange={onChange} />
    )

    userEvent.click(screen.getByRole("combobox", {name: SCREEN}))
    const screens = within(screen.getByRole("listbox")).getAllByRole("option")
    expect(screens.map((option) => option.getAttribute("data-value"))).toEqual([
        "",
        "election-chooser",
        "start",
        "ballot",
        "review",
        "confirmation",
        "audit",
        "ballot-locator",
        "support-materials",
    ])
    userEvent.click(screens[3])
    expect(onChange).toHaveBeenLastCalledWith({screen: "ballot", language: "tl"})

    rerender(
        <AudioInstructionsFields
            kind="audio/mpeg"
            languages={["tl", "en"]}
            value={{screen: "ballot", language: "tl"}}
            onChange={onChange}
        />
    )
    userEvent.click(screen.getByRole("combobox", {name: LANGUAGE}))
    userEvent.click(screen.getByRole("option", {name: "common.language.en"}))
    expect(onChange).toHaveBeenLastCalledWith({screen: "ballot", language: "en"})
})

it("removes the assignment when no screen is chosen", async () => {
    const onChange = jest.fn()
    render(
        <AudioInstructionsFields
            kind="audio/ogg"
            languages={["en"]}
            value={{screen: "review", language: "en"}}
            onChange={onChange}
        />
    )
    userEvent.click(screen.getByRole("combobox", {name: SCREEN}))
    userEvent.click(screen.getByRole("option", {name: "materials.audioInstructions.none"}))
    expect(onChange).toHaveBeenLastCalledWith(undefined)
})

it("keeps the rest of the material's data when the assignment changes", () => {
    const data = {title_i18n: {en: "Ballot audio"}, subtitle_i18n: {}}
    const assigned = withAudioInstructions(data, {screen: "ballot", language: "en"})
    expect(assigned).toEqual({...data, audio_instructions: {screen: "ballot", language: "en"}})
    expect(withAudioInstructions(assigned, undefined)).toEqual(data)
    expect(withAudioInstructions(null, undefined)).toEqual({})
})

/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The record viewer (design board RecordViewer): a record's text in a card,
// as a line of preview or laid out with line numbers; to copy and to save.

import React from "react"
import {act, render, screen, within} from "@testing-library/react"
import userEvent from "@testing-library/user-event"
import {COPIED_MS} from "../../hooks/useCopyFeedback"
import {saveTextFile} from "../../services/vitrine/file"
import {RecordViewer} from "./RecordViewer"

const WORDS: Record<string, string> = {
    "vitrine.record.json": "JSON",
    "vitrine.record.text": "Text",
    "vitrine.record.fields_one": "{{count}} field",
    "vitrine.record.fields_other": "{{count}} fields",
    "vitrine.record.items_one": "{{count}} item",
    "vitrine.record.items_other": "{{count}} items",
    "vitrine.record.size.bytes": "{{value}} B",
    "vitrine.record.size.kilobytes": "{{value}} KB",
    "vitrine.record.copy": "Copy {{file}}",
    "vitrine.record.copied": "Copied",
    "vitrine.record.download": "Download {{file}}",
}

jest.mock("react-i18next", () => ({
    useTranslation: () => ({
        t: (key: string, values: Record<string, unknown> = {}) => {
            const plural =
                typeof values.count === "number"
                    ? `${key}_${values.count === 1 ? "one" : "other"}`
                    : key
            return (WORDS[plural] ?? key).replace(/{{(\w+)}}/g, (_, name) => String(values[name]))
        },
        i18n: {language: "en", resolvedLanguage: "en"},
    }),
}))
jest.mock("../../services/vitrine/file", () => ({saveTextFile: jest.fn()}))

const JSON_CONTENT = '{"example":true,"items":[1,2,3],"note":"Pass any JSON as data"}'
const PRETTY = JSON.stringify(JSON.parse(JSON_CONTENT), null, 2)
const FILE = "ballot-FTBEMHRX.json"

const renderViewer = (
    content = JSON_CONTENT,
    props: Partial<React.ComponentProps<typeof RecordViewer>> = {}
) =>
    render(
        <RecordViewer
            title="Your ballot content"
            subtitle="Encrypted, as in the public record"
            filename={FILE}
            content={content}
            {...props}
        />
    )

const toggle = () => screen.getByRole("button", {expanded: false}) as HTMLButtonElement

describe("the card of a JSON record", () => {
    it("names itself, with what the record is, its format, how much it holds and its size", () => {
        renderViewer()
        const card = screen.getByRole("region", {name: "Your ballot content"})
        expect(within(card).getByText("Your ballot content")).toBeVisible()
        expect(
            within(card).getByText("Encrypted, as in the public record · JSON · 3 fields · 63 B")
        ).toBeVisible()
    })

    it("counts the items of a list, and leaves out what the record is when it has no subtitle", () => {
        renderViewer("[1,2,3,4]", {subtitle: undefined})
        expect(screen.getByText("JSON · 4 items · 9 B")).toBeVisible()
    })

    it("counts one field, and none for a JSON value that has no fields", () => {
        renderViewer('{"a":1}')
        expect(screen.getByText(/JSON · 1 field · 7 B$/)).toBeVisible()
    })

    it("gives the size in kilobytes from a kilobyte", () => {
        renderViewer(JSON.stringify({text: "x".repeat(1500)}))
        expect(screen.getByText(/JSON · 1 field · 1\.5 KB$/)).toBeVisible()
    })

    it("shows a line of preview of the compact record, and none of its layout", () => {
        const {container} = renderViewer('{\n  "example": true,\n  "items": [1, 2]\n}')
        expect(toggle()).toHaveAttribute("aria-expanded", "false")
        expect(container.querySelector(".record-viewer-preview")).toHaveTextContent(
            '{"example":true,"items":[1,2]}'
        )
        expect(container.querySelector(".record-viewer-body")).toBeNull()
    })

    it("hides the preview from assistive technology, which has the card's meta line", () => {
        const {container} = renderViewer()
        expect(container.querySelector(".record-viewer-preview")).toHaveAttribute(
            "aria-hidden",
            "true"
        )
    })
})

describe("opening the card", () => {
    it("lays the JSON out with a number for each line, and takes the preview away", async () => {
        const {container} = renderViewer()
        await userEvent.click(toggle())
        expect(screen.getByRole("button", {expanded: true})).toBeVisible()
        expect(container.querySelector(".record-viewer-preview")).toBeNull()
        const lines = Array.from(container.querySelectorAll(".record-viewer-line"))
        expect(lines.map((line) => line.querySelector(".record-viewer-code")?.textContent)).toEqual(
            PRETTY.split("\n")
        )
        expect(
            lines.map((line) => line.querySelector(".record-viewer-line-number")?.textContent)
        ).toEqual(PRETTY.split("\n").map((_, n) => String(n + 1)))
    })

    it("keeps the line numbers away from what the voter selects and assistive technology reads", async () => {
        const {container} = renderViewer()
        await userEvent.click(toggle())
        container.querySelectorAll(".record-viewer-line-number").forEach((number) => {
            expect(number).toHaveAttribute("aria-hidden", "true")
        })
    })

    it("colours the keys, strings, numbers and literals of the JSON apart", async () => {
        const {container} = renderViewer()
        await userEvent.click(toggle())
        const of = (kind: string) =>
            Array.from(container.querySelectorAll(`.record-token-${kind}`)).map(
                (token) => token.textContent
            )
        expect(of("key")).toEqual(['"example"', '"items"', '"note"'])
        expect(of("string")).toEqual(['"Pass any JSON as data"'])
        expect(of("number")).toEqual(["1", "2", "3"])
        expect(of("literal")).toEqual(["true"])
    })

    it("closes again, back to the preview", async () => {
        const {container} = renderViewer()
        await userEvent.click(toggle())
        await userEvent.click(screen.getByRole("button", {expanded: true}))
        expect(toggle()).toBeVisible()
        expect(container.querySelector(".record-viewer-body")).toBeNull()
        expect(container.querySelector(".record-viewer-preview")).not.toBeNull()
    })

    it("can be scrolled with the keyboard, and is named by the card's title", async () => {
        const {container} = renderViewer()
        await userEvent.click(toggle())
        const body = container.querySelector(".record-viewer-body")
        expect(body).toHaveAttribute("tabindex", "0")
        expect(screen.getByRole("group", {name: "Your ballot content"})).toBe(body)
    })
})

describe("the card of text that is not JSON", () => {
    const TEXT = "A ballot,\n  as plain text"

    it("says it is text, and counts nothing", () => {
        renderViewer(TEXT)
        expect(screen.getByText("Encrypted, as in the public record · Text · 25 B")).toBeVisible()
    })

    it("shows it as it is, on a line of preview and laid out, without colours", async () => {
        const {container} = renderViewer(TEXT)
        expect(container.querySelector(".record-viewer-preview")).toHaveTextContent(
            "A ballot, as plain text"
        )
        await userEvent.click(toggle())
        const code = Array.from(container.querySelectorAll(".record-viewer-code"))
        expect(code.map((line) => line.textContent)).toEqual(["A ballot,", "  as plain text"])
        expect(container.querySelectorAll(".record-token-text")).toHaveLength(2)
        expect(container.querySelector(".record-token-key")).toBeNull()
    })

    it("shows an empty record without lines to speak of", async () => {
        const {container} = renderViewer("")
        expect(screen.getByText("Encrypted, as in the public record · Text · 0 B")).toBeVisible()
        await userEvent.click(toggle())
        expect(container.querySelectorAll(".record-viewer-line")).toHaveLength(1)
    })
})

describe("copying and saving", () => {
    afterEach(() => {
        jest.useRealTimers()
        jest.mocked(saveTextFile).mockReset()
    })

    const setClipboard = (writeText: jest.Mock) =>
        Object.defineProperty(navigator, "clipboard", {value: {writeText}, configurable: true})

    it("has a button to copy the record and another to download it, each named by the file", () => {
        renderViewer()
        expect(screen.getByRole("button", {name: `Copy ${FILE}`})).toBeVisible()
        expect(screen.getByRole("button", {name: `Download ${FILE}`})).toBeVisible()
    })

    it("copies the record as it reads laid out, however it was given", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        setClipboard(writeText)
        renderViewer()
        await userEvent.click(screen.getByRole("button", {name: `Copy ${FILE}`}))
        expect(writeText).toHaveBeenCalledWith(PRETTY)
    })

    it("copies text that is not JSON as it is", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        setClipboard(writeText)
        renderViewer("plain  text")
        await userEvent.click(screen.getByRole("button", {name: `Copy ${FILE}`}))
        expect(writeText).toHaveBeenCalledWith("plain  text")
    })

    it("swaps the copy icon for a check, and the name for Copied, for a moment", async () => {
        jest.useFakeTimers()
        setClipboard(jest.fn().mockResolvedValue(undefined))
        const user = userEvent.setup({advanceTimers: jest.advanceTimersByTime})
        const {container} = renderViewer()
        const button = screen.getByRole("button", {name: `Copy ${FILE}`})
        expect(button.querySelector(".record-viewer-copy-icon")).not.toBeNull()
        await user.click(button)
        expect(screen.getByRole("button", {name: "Copied"})).toBe(button)
        expect(button.querySelector(".record-viewer-copied-icon")).not.toBeNull()
        expect(button.querySelector(".record-viewer-copy-icon")).toBeNull()
        act(() => jest.advanceTimersByTime(COPIED_MS))
        expect(screen.getByRole("button", {name: `Copy ${FILE}`})).toBe(button)
        expect(container.querySelector(".record-viewer-copied-icon")).toBeNull()
    })

    it("saves the record laid out, as JSON, under the file's name", async () => {
        renderViewer()
        await userEvent.click(screen.getByRole("button", {name: `Download ${FILE}`}))
        expect(saveTextFile).toHaveBeenCalledWith(FILE, PRETTY, "application/json")
    })

    it("saves text that is not JSON as plain text", async () => {
        renderViewer("plain text", {filename: "note.txt"})
        await userEvent.click(screen.getByRole("button", {name: "Download note.txt"}))
        expect(saveTextFile).toHaveBeenCalledWith("note.txt", "plain text", "text/plain")
    })
})

/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {fireEvent, render, screen, waitFor} from "@testing-library/react"
import {CopyableFingerprint} from "./CopyableFingerprint"

const mockNotify = jest.fn()
jest.mock("react-admin", () => ({useNotify: () => mockNotify}))
jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))

const FULL = "f7080f24".padEnd(60, "a") + "fe78"
const COPY = "copyableFingerprint.copy"

function withClipboard(clipboard: unknown) {
    Object.defineProperty(navigator, "clipboard", {configurable: true, value: clipboard})
}

describe("CopyableFingerprint", () => {
    beforeEach(() => mockNotify.mockClear())

    it("shows the given short text, with the complete value in its tooltip", () => {
        render(<CopyableFingerprint value={FULL} short="F7:08:…:FE:78" />)
        expect(screen.getByText("F7:08:…:FE:78")).toHaveProperty("title", FULL)
        expect(screen.queryByText(FULL)).toBeNull()
    })

    it("shortens the value itself when no short text is given", () => {
        render(<CopyableFingerprint value={FULL} />)
        expect(screen.getByText("f7080f24…aafe78")).toHaveProperty("title", FULL)
    })

    it("copies the complete value and confirms it", async () => {
        const writeText = jest.fn(async () => undefined)
        withClipboard({writeText})
        render(<CopyableFingerprint value={FULL} short="F7:08:…:FE:78" />)
        fireEvent.click(screen.getByRole("button", {name: COPY}))
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("copyableFingerprint.copied", {
                type: "success",
            })
        )
        expect(writeText).toHaveBeenCalledWith(FULL)
    })

    it.each([
        [
            "refuses the copy",
            {
                writeText: async () => {
                    throw new Error("Denied")
                },
            },
        ],
        ["has no clipboard", undefined],
    ])("reports an error and keeps the tooltip when the browser %s", async (_case, clipboard) => {
        withClipboard(clipboard)
        render(<CopyableFingerprint value={FULL} short="F7:08:…:FE:78" />)
        fireEvent.click(screen.getByRole("button", {name: COPY}))
        await waitFor(() =>
            expect(mockNotify).toHaveBeenCalledWith("copyableFingerprint.copyError", {
                type: "error",
            })
        )
        expect(screen.getByText("F7:08:…:FE:78")).toHaveProperty("title", FULL)
    })
})

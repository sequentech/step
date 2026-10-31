/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The full Tracking ID block (design board FullTrackingId).

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {FullTrackingId} from "./FullTrackingId"

jest.mock("react-i18next", () => ({useTranslation: () => ({t: (key: string) => key})}))
jest.mock("@sequentech/ui-essentials", () => ({
    copyBallotHash: jest.requireActual(
        "../../../../ui-essentials/src/components/BallotHash/BallotHash"
    ).copyBallotHash,
    CopyBallotHashStatus: jest.requireActual(
        "../../../../ui-essentials/src/components/BallotHash/BallotHash"
    ).CopyBallotHashStatus,
}))

const GROUPED = "FTBE-MHRX-0123-4567-89AB-CDEF-GHJK-MNPQ-RSTV-WXYZ-0123-4567-89AB"
const setClipboard = (clipboard: unknown) =>
    Object.defineProperty(navigator, "clipboard", {value: clipboard, configurable: true})

afterEach(() => {
    setClipboard(undefined)
    Reflect.deleteProperty(document, "execCommand")
    window.getSelection()?.removeAllRanges()
})

describe("the full Tracking ID block", () => {
    it("labels the ID, and shows it in groups of four that never break inside", () => {
        const {container} = render(<FullTrackingId ballotId={GROUPED.replace(/-/g, "")} />)
        expect(container.querySelector(".full-tracking-id-label")?.textContent).toBe(
            "vitrine.ballotId.full"
        )
        expect(container.querySelector(".full-tracking-id-value")?.textContent).toBe(GROUPED)
        const groups = Array.from(container.querySelectorAll(".full-tracking-id-group"))
        expect(groups).toHaveLength(13)
        for (const group of groups) {
            expect(getComputedStyle(group).whiteSpace).toBe("nowrap")
        }
    })

    it("sets the short ID, the first two groups, in bold", () => {
        const {container} = render(<FullTrackingId ballotId={GROUPED} />)
        const weights = Array.from(container.querySelectorAll(".full-tracking-id-group")).map(
            (group) => getComputedStyle(group).fontWeight
        )
        expect(weights.slice(0, 2)).toEqual(["700", "700"])
        expect(weights.slice(2).every((weight) => weight === "400")).toBe(true)
    })

    it("has a copy button that is always visible, and copies the ID in groups", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        setClipboard({writeText})
        render(<FullTrackingId ballotId={GROUPED.replace(/-/g, "")} />)
        const copy = screen.getByRole("button", {name: "vitrine.ballotId.copy"})
        expect(getComputedStyle(copy).opacity).not.toBe("0")
        await act(async () => {
            fireEvent.click(copy)
        })
        expect(writeText).toHaveBeenCalledWith(GROUPED)
        expect(screen.getByRole("button", {name: "vitrine.ballotId.copied"})).toBe(copy)
        expect(screen.getByRole("status").textContent).toBe("vitrine.ballotId.copied")
    })

    it("tries the fallback when the Clipboard API refuses", async () => {
        setClipboard({writeText: jest.fn().mockRejectedValue(new Error("denied"))})
        const execCommand = jest.fn().mockReturnValue(true)
        Object.defineProperty(document, "execCommand", {configurable: true, value: execCommand})
        render(<FullTrackingId ballotId={GROUPED} />)
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(execCommand).toHaveBeenCalledWith("copy")
        expect(screen.getByRole("status").textContent).toBe("vitrine.ballotId.copied")
    })

    it("selects the ID and says how to copy it by hand when nothing can copy, with no warning", async () => {
        const {container} = render(<FullTrackingId ballotId={GROUPED} />)
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(window.getSelection()?.toString()).toBe(GROUPED)
        expect(screen.getByRole("status").textContent).toBe("vitrine.ballotId.selected")
        expect(container.querySelector("[data-icon='triangle-exclamation']")).toBeNull()
        expect(screen.queryByRole("alert")).toBeNull()
        expect(screen.getByRole("button", {name: "vitrine.ballotId.copy"})).toBeTruthy()
    })
})

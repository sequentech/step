/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The full Tracking ID block (design boards FullTrackingId and ReviewFixes).

import React from "react"
import {act, fireEvent, render, screen} from "@testing-library/react"
import {EFullTrackingIdSurface, FullTrackingId} from "./FullTrackingId"

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

/** The CSS the block's styles produced, as the document holds it. */
const css = (): string =>
    Array.from(document.styleSheets)
        .flatMap((sheet) => Array.from(sheet.cssRules).map((rule) => rule.cssText))
        .join("\n")

const chips = (container: HTMLElement) =>
    Array.from(container.querySelectorAll<HTMLElement>(".full-tracking-id-group"))

afterEach(() => {
    setClipboard(undefined)
    Reflect.deleteProperty(document, "execCommand")
    window.getSelection()?.removeAllRanges()
})

describe("the full Tracking ID block", () => {
    it("labels the ID, and shows its 13 groups as chips that never break inside", () => {
        const {container} = render(<FullTrackingId ballotId={GROUPED.replace(/-/g, "")} />)
        expect(container.querySelector(".full-tracking-id-label")?.textContent).toBe(
            "vitrine.ballotId.full"
        )
        expect(container.querySelector(".full-tracking-id-value")?.textContent).toBe(GROUPED)
        expect(chips(container)).toHaveLength(13)
        for (const chip of chips(container)) {
            expect(getComputedStyle(chip).whiteSpace).toBe("nowrap")
        }
        const group = container.querySelector("[role=group]")
        expect(group?.getAttribute("aria-labelledby")).toBe(
            container.querySelector(".full-tracking-id-label")?.id
        )
    })

    it("fills the two groups of the short ID with the key colour, and leaves the rest outlined", () => {
        const {container} = render(<FullTrackingId ballotId={GROUPED} />)
        const all = chips(container)
        for (const chip of all.slice(0, 2)) {
            expect(chip.classList.contains("full-tracking-id-short")).toBe(true)
            expect(getComputedStyle(chip).backgroundColor).toBe("rgb(15, 5, 76)")
            expect(getComputedStyle(chip).color).toBe("rgb(255, 255, 255)")
            expect(getComputedStyle(chip).fontWeight).toBe("700")
        }
        for (const chip of all.slice(2)) {
            expect(chip.classList.contains("full-tracking-id-short")).toBe(false)
            expect(getComputedStyle(chip).backgroundColor).toBe("rgb(255, 255, 255)")
            expect(getComputedStyle(chip).color).toBe("rgb(15, 5, 76)")
            expect(getComputedStyle(chip).fontWeight).toBe("500")
        }
    })

    it.each([
        [EFullTrackingIdSurface.WHITE, "rgb(15, 5, 76)"],
        [EFullTrackingIdSurface.TINT, "rgb(15, 5, 76)"],
        [EFullTrackingIdSurface.GREEN, "rgb(4, 120, 87)"],
        [EFullTrackingIdSurface.HUB, "rgb(14, 79, 85)"],
    ])("takes the key colour of the %s surface", (surface, key) => {
        const {container} = render(<FullTrackingId ballotId={GROUPED} surface={surface} />)
        expect(container.querySelector(".full-tracking-id")?.getAttribute("data-surface")).toBe(
            surface
        )
        expect(getComputedStyle(chips(container)[0]).backgroundColor).toBe(key)
    })

    it("takes the colour of the green card behind a thin rule, with no box of its own", () => {
        const {container} = render(
            <FullTrackingId ballotId={GROUPED} surface={EFullTrackingIdSurface.GREEN} />
        )
        const block = getComputedStyle(container.querySelector(".full-tracking-id") as Element)
        expect(block.borderTopWidth).toBe("1px")
        expect(block.borderLeftWidth).toBe("0px")
        expect(block.backgroundColor).toMatch(/^(transparent|rgba\(0, 0, 0, 0\))$/)
        expect(block.borderRadius).toMatch(/^0(px)?$/)
    })

    it.each([
        EFullTrackingIdSurface.WHITE,
        EFullTrackingIdSurface.TINT,
        EFullTrackingIdSurface.HUB,
    ])("is a soft panel on the %s surface, never a plain white box", (surface) => {
        const {container} = render(<FullTrackingId ballotId={GROUPED} surface={surface} />)
        const block = getComputedStyle(container.querySelector(".full-tracking-id") as Element)
        expect(block.borderTopWidth).toBe("1px")
        expect(block.borderRadius).toBe("10px")
        expect(block.backgroundColor).not.toBe("rgb(255, 255, 255)")
    })

    it("lays the 13 chips out in one row, 7 and 6, or 5, 5 and 3, by the block's own width", () => {
        const {container} = render(<FullTrackingId ballotId={GROUPED} />)
        const wrap = container.querySelector(".full-tracking-id-chips-wrap") as Element
        expect(getComputedStyle(wrap).getPropertyValue("container-type")).toBe("inline-size")
        const styles = css()
        expect(styles).toContain("grid-template-columns: repeat(7, max-content)")
        expect(styles).toMatch(/@container \(min-width: 720px\) \{[^}]*repeat\(13, max-content\)/)
        expect(styles).toMatch(/@container \(max-width: 384px\) \{[^}]*repeat\(5, max-content\)/)
        expect(styles).toMatch(/@container \(max-width: 268px\) \{[^}]*gap: 4px/)
    })

    it("has a labelled Copy button that is always visible, and copies the ID in groups", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        setClipboard({writeText})
        render(<FullTrackingId ballotId={GROUPED.replace(/-/g, "")} />)
        const copy = screen.getByRole("button", {name: "vitrine.ballotId.copy"})
        expect(copy.textContent).toBe("vitrine.ballotId.copyText")
        expect(getComputedStyle(copy).opacity).not.toBe("0")
        expect(getComputedStyle(copy).backgroundColor).toBe("rgb(255, 255, 255)")
        await act(async () => {
            fireEvent.click(copy)
        })
        expect(writeText).toHaveBeenCalledWith(GROUPED)
        const done = screen.getByRole("button", {name: "vitrine.ballotId.copied"})
        expect(done).toBe(copy)
        expect(done.textContent).toBe("vitrine.ballotId.copiedText")
        expect(getComputedStyle(done).backgroundColor).toBe("rgb(4, 120, 87)")
        expect(getComputedStyle(done).color).toBe("rgb(255, 255, 255)")
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

    it("selects the chips and says how to copy by hand when nothing can copy, with no warning", async () => {
        const {container} = render(<FullTrackingId ballotId={GROUPED} />)
        await act(async () => {
            fireEvent.click(screen.getByRole("button", {name: "vitrine.ballotId.copy"}))
        })
        expect(window.getSelection()?.toString()).toBe(GROUPED)
        expect(screen.getByRole("status").textContent).toBe("vitrine.ballotId.selected")
        for (const chip of chips(container)) {
            expect(getComputedStyle(chip).backgroundColor).toBe("rgb(199, 215, 254)")
        }
        expect(container.querySelector("[data-icon='triangle-exclamation']")).toBeNull()
        expect(screen.queryByRole("alert")).toBeNull()
        expect(screen.getByRole("button", {name: "vitrine.ballotId.copy"})).toBeTruthy()
    })
})

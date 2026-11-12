// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {expect, userEvent} from "storybook/test"

const box = (element: HTMLElement) => element.getBoundingClientRect()

/** The lines a button's label takes, from its text alone. */
const lines = (button: HTMLElement) => {
    const tops = new Set<number>()
    const walker = document.createTreeWalker(button, NodeFilter.SHOW_TEXT)
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        if (!node.textContent?.trim()) continue
        const range = document.createRange()
        range.selectNodeContents(node)
        for (const line of Array.from(range.getClientRects())) tops.add(Math.round(line.top))
    }
    return tops.size
}

/** The width a button would take with its label on one line. */
const oneLineWidth = (button: HTMLElement) => {
    const {whiteSpace, width, maxWidth} = button.style
    Object.assign(button.style, {whiteSpace: "nowrap", width: "max-content", maxWidth: "none"})
    const wide = button.getBoundingClientRect().width
    Object.assign(button.style, {whiteSpace, width, maxWidth})
    return wide
}

/** The simulation editor's content box. */
export const editorContent = (editor: HTMLElement) => {
    const style = getComputedStyle(editor)
    const bounds = box(editor)
    const left = bounds.left + parseFloat(style.paddingLeft) + parseFloat(style.borderLeftWidth)
    const right = bounds.right - parseFloat(style.paddingRight) - parseFloat(style.borderRightWidth)
    return {left, right, width: right - left}
}

/**
 * The editor's buttons, with its draft edited: the fill-in button on its own row at the left, then
 * Cancel at the left and Apply at the right of one row, inside the editor, no word broken, nothing
 * cut, with nothing focused and with Cancel and then Apply focused from the keyboard.
 */
export const expectEditorButtons = async (editor: HTMLElement, fontSize: string) => {
    const {left, right, width} = editorContent(editor)
    const [fill, cancel, apply] = Array.from(editor.querySelectorAll("button")).slice(-3)
    await expect(fill).toBeVisible()
    expect(box(fill).bottom).toBeLessThanOrEqual(box(cancel).top)
    expect(Math.abs(box(fill).left - left)).toBeLessThanOrEqual(2)
    // A label that fits on one line leaves the rest of the row free; a longer one wraps inside it.
    if (oneLineWidth(fill) <= width) expect(box(fill).width).toBeLessThan(width)
    const footer = (focused: string) => {
        expect(Math.abs(box(cancel).left - left), focused).toBeLessThanOrEqual(2)
        expect(box(apply).right, focused).toBeLessThanOrEqual(right + 0.5)
        expect(right - box(apply).right, focused).toBeLessThanOrEqual(2)
        expect(Math.abs(box(cancel).top - box(apply).top), focused).toBeLessThanOrEqual(1)
        expect(box(cancel).right, focused).toBeLessThan(box(apply).left)
        expect(box(fill).right, focused).toBeLessThanOrEqual(right + 0.5)
        expect(fill.scrollWidth, focused).toBeLessThanOrEqual(fill.clientWidth + 1)
        for (const button of [cancel, apply]) {
            if (!/\s/.test((button.textContent ?? "").trim()))
                expect(lines(button), focused).toBe(1)
            expect(button.scrollWidth, focused).toBeLessThanOrEqual(button.clientWidth + 1)
            expect(getComputedStyle(button).fontSize, focused).toBe(fontSize)
        }
        expect(document.documentElement.scrollWidth, focused).toBeLessThanOrEqual(window.innerWidth)
    }
    footer("nothing focused")
    // Keyboard order follows: fill in, Cancel, Apply; a focused button's border still fits.
    fill.focus()
    await userEvent.tab()
    await expect(cancel).toHaveFocus()
    footer("Cancel focused")
    await userEvent.tab()
    await expect(apply).toHaveFocus()
    footer("Apply focused")
}

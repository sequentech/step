/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {contentAfter, scrollFrame} from "./matrixLayout"

/** An element drawn from `top` to `top + height`; jsdom draws nothing on its own. */
function box(
    tag: string,
    {top, height, style = {}}: {top: number; height: number; style?: Partial<CSSStyleDeclaration>}
): HTMLElement {
    const element = document.createElement(tag)
    Object.assign(element.style, style)
    element.getBoundingClientRect = () =>
        ({top, bottom: top + height, height, left: 0, right: 100, width: 100}) as DOMRect
    return element
}

/** A page whose scrolled frame holds a page with the columns and what follows them. */
function page(after: HTMLElement[]) {
    const frame = box("div", {top: 0, height: 800, style: {overflowY: "auto"}})
    const content = box("div", {top: 0, height: 1500, style: {paddingBottom: "24px"}})
    const columns = box("div", {top: 100, height: 1200, style: {marginBottom: "8px"}})
    content.append(columns, ...after)
    frame.append(content)
    document.body.replaceChildren(frame)
    return {frame, columns}
}

describe("contentAfter", () => {
    it("counts what comes below the columns, with margins and the paddings around them", () => {
        const {frame, columns} = page([
            box("div", {top: 1308, height: 60, style: {marginTop: "24px", marginBottom: "4px"}}),
        ])
        // The columns' margin, the next element with its margins, the page's bottom padding.
        expect(contentAfter(columns, frame)).toBe(8 + 60 + 24 + 4 + 24)
    })

    it("counts a sticky save bar drawn stuck above its place", () => {
        const {frame, columns} = page([
            box("div", {top: 700, height: 80, style: {position: "sticky", marginTop: "24px"}}),
        ])
        expect(contentAfter(columns, frame)).toBe(8 + 80 + 24 + 24)
    })

    it("ignores what is beside the columns, hidden or positioned out of the flow", () => {
        const {frame, columns} = page([
            box("div", {top: 100, height: 300}),
            box("div", {top: 1308, height: 50, style: {display: "none"}}),
            box("div", {top: 1308, height: 50, style: {position: "absolute"}}),
            box("div", {top: 1308, height: 50, style: {position: "fixed"}}),
        ])
        expect(contentAfter(columns, frame)).toBe(8 + 24)
    })

    it("adds what follows each ancestor up to the frame, and nothing outside it", () => {
        const frame = box("div", {top: 0, height: 800, style: {overflowY: "auto"}})
        const page = box("div", {top: 0, height: 1600, style: {paddingBottom: "16px"}})
        const tab = box("div", {top: 0, height: 1500, style: {paddingBottom: "8px"}})
        const columns = box("div", {top: 100, height: 1200})
        tab.append(columns, box("div", {top: 1300, height: 70}))
        page.append(tab, box("footer", {top: 1508, height: 40, style: {marginTop: "4px"}}))
        frame.append(page)
        // Outside the frame: never counted.
        document.body.replaceChildren(frame, box("div", {top: 900, height: 500}))
        // Below the columns in the tab, the tab's padding, the footer below the tab, the page's padding.
        expect(contentAfter(columns, frame)).toBe(70 + 8 + 40 + 4 + 16)
    })
})

describe("scrollFrame", () => {
    it("is the nearest ancestor that scrolls", () => {
        const {frame, columns} = page([])
        expect(scrollFrame(columns)).toBe(frame)
    })

    it("is the document when nothing scrolls", () => {
        const element = document.createElement("div")
        document.body.replaceChildren(element)
        expect(scrollFrame(element)).toBe(document.documentElement)
    })
})

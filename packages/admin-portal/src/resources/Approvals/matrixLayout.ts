// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Measurements the approval matrix's sticky example card needs from the page around it.

const px = (value: string) => parseFloat(value) || 0

/**
 * The height of what follows `element` in the scrolled content up to `scroller`'s end: the
 * elements below it and below each of its ancestors, their margins and the ancestors' bottom
 * padding and border.
 */
export function contentAfter(element: HTMLElement, scroller: HTMLElement): number {
    let room = 0
    for (
        let node: HTMLElement | null = element;
        node && node !== scroller;
        node = node.parentElement
    ) {
        room += px(getComputedStyle(node).marginBottom)
        const bottom = node.getBoundingClientRect().bottom
        for (let next = node.nextElementSibling; next; next = next.nextElementSibling) {
            const style = getComputedStyle(next)
            // Only what comes below: a layout's row puts siblings beside each other. A sticky
            // sibling (the save bar) may be drawn stuck above its place, which is below.
            if (
                style.display === "none" ||
                ["absolute", "fixed"].includes(style.position) ||
                (style.position !== "sticky" && next.getBoundingClientRect().top < bottom - 1)
            ) {
                continue
            }
            room +=
                next.getBoundingClientRect().height + px(style.marginTop) + px(style.marginBottom)
        }
        const parent = node.parentElement
        if (parent) {
            const style = getComputedStyle(parent)
            room += px(style.paddingBottom) + px(style.borderBottomWidth)
        }
    }
    return room
}

/** The element `element` scrolls in: its nearest ancestor that scrolls, or the document. */
export function scrollFrame(element: HTMLElement): HTMLElement {
    for (let parent = element.parentElement; parent; parent = parent.parentElement) {
        if (/(auto|scroll)/.test(getComputedStyle(parent).overflowY)) {
            return parent
        }
    }
    return document.documentElement
}

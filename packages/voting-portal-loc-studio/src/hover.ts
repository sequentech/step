// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {firstKeyFromText, keysFromText} from "./markers"

const SKIP_TAGS = new Set(["SCRIPT", "STYLE", "SVG", "PATH", "TEXTAREA", "INPUT", "SELECT"])

export interface HoverTarget {
    key: string
    element: HTMLElement
}

export const isStudioChrome = (target: EventTarget | null): boolean => {
    if (!(target instanceof Element)) {
        return false
    }
    return Boolean(
        target.closest(
            ".loc-studio-nav, .loc-studio-keys, .loc-studio-toolbar, .loc-studio-hover-chip, .loc-studio-highlight, .loc-studio-import-overlay"
        )
    )
}

const isSkippable = (element: HTMLElement): boolean =>
    SKIP_TAGS.has(element.tagName) || Boolean(element.closest("svg"))

const keyFromNode = (node: Node | null): HoverTarget | null => {
    let current: Node | null = node
    for (let depth = 0; depth < 12 && current; depth += 1) {
        if (current.nodeType === Node.TEXT_NODE) {
            const key = firstKeyFromText(current.textContent || "")
            if (key && current.parentElement && !isSkippable(current.parentElement)) {
                return {key, element: current.parentElement}
            }
        } else if (current.nodeType === Node.ELEMENT_NODE) {
            const element = current as HTMLElement
            if (!isSkippable(element)) {
                for (const child of Array.from(element.childNodes)) {
                    if (child.nodeType === Node.TEXT_NODE) {
                        const key = firstKeyFromText(child.textContent || "")
                        if (key) {
                            return {key, element}
                        }
                    }
                }
            }
        }
        current = current.parentNode
    }
    return null
}

const caretNodeFromPoint = (x: number, y: number): Node | null => {
    const documentWithCaret = document as Document & {
        caretRangeFromPoint?: (x: number, y: number) => Range | null
        caretPositionFromPoint?: (x: number, y: number) => {offsetNode: Node; offset: number} | null
    }
    if (typeof documentWithCaret.caretRangeFromPoint === "function") {
        return documentWithCaret.caretRangeFromPoint(x, y)?.startContainer ?? null
    }
    if (typeof documentWithCaret.caretPositionFromPoint === "function") {
        return documentWithCaret.caretPositionFromPoint(x, y)?.offsetNode ?? null
    }
    return null
}

export const hoverTargetFromPoint = (x: number, y: number): HoverTarget | null => {
    const element = document.elementFromPoint(x, y)
    if (!(element instanceof HTMLElement) || isStudioChrome(element)) {
        return null
    }
    return keyFromNode(caretNodeFromPoint(x, y) ?? element)
}

export const collectKeysFromRoots = (roots: ParentNode[]): string[] => {
    const keys = new Set<string>()
    roots.forEach((root) => {
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
        let node = walker.nextNode()
        while (node) {
            keysFromText(node.textContent || "").forEach((key) => keys.add(key))
            node = walker.nextNode()
        }
    })
    return Array.from(keys)
}

export const elementForKey = (roots: ParentNode[], key: string): HTMLElement | null => {
    for (const root of roots) {
        const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT)
        let node = walker.nextNode()
        while (node) {
            if (keysFromText(node.textContent || "").includes(key) && node.parentElement) {
                return node.parentElement
            }
            node = walker.nextNode()
        }
    }
    return null
}

export const previewRoots = (previewRoot: ParentNode | null): ParentNode[] => {
    const roots: ParentNode[] = []
    if (previewRoot) {
        roots.push(previewRoot)
    }
    document.querySelectorAll('[role="dialog"], .MuiDialog-paper, .MuiMenu-paper').forEach((node) => {
        roots.push(node)
    })
    return roots
}

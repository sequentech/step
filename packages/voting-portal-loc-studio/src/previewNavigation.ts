// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {isStudioChrome} from "./hover"

const ACTIVATABLE_SELECTOR =
    "button:not([disabled]), [role='button']:not([aria-disabled='true']), a[href], .election-item, .start-voting-button, .next-button, .cast-ballot-button, .finish-button, .MuiButtonBase-root"

export const findActivatableElement = (target: EventTarget | null): HTMLElement | null => {
    if (!(target instanceof Element)) {
        return null
    }
    const match = target.closest(ACTIVATABLE_SELECTOR)
    if (!(match instanceof HTMLElement)) {
        return null
    }
    if (match.getAttribute("aria-disabled") === "true" || match.hasAttribute("disabled")) {
        return null
    }
    return match
}

export const activatePreviewControl = (element: HTMLElement): void => {
    element.dispatchEvent(new MouseEvent("mousedown", {bubbles: true, cancelable: true}))
    element.dispatchEvent(new MouseEvent("mouseup", {bubbles: true, cancelable: true}))
    element.click()
}

export interface SceneFromPath {
    sceneId: string
    variantId: string
}

export const sceneFromPath = (pathname: string): SceneFromPath | null => {
    if (pathname.includes("/election-chooser")) {
        return {sceneId: "election-list", variantId: "default"}
    }
    if (pathname.includes("/start")) {
        return {sceneId: "start", variantId: "default"}
    }
    if (pathname.includes("/review")) {
        return {sceneId: "review", variantId: "default"}
    }
    if (pathname.includes("/confirmation")) {
        return {sceneId: "confirmation", variantId: "default"}
    }
    if (pathname.includes("/audit")) {
        return {sceneId: "audit", variantId: "default"}
    }
    if (pathname.includes("/ballot-locator")) {
        return {sceneId: "ballot-locator", variantId: "lookup"}
    }
    if (pathname.includes("/materials")) {
        return {sceneId: "materials", variantId: "default"}
    }
    if (pathname.includes("/vote")) {
        return null
    }
    return null
}

export const handlePreviewDoubleClick = (
    event: MouseEvent,
    onNavigate: (scene: SceneFromPath) => void
): boolean => {
    const target = event.target
    if (!target || isStudioChrome(target)) {
        return false
    }

    const activatable = findActivatableElement(target)
    if (!activatable) {
        return false
    }

    event.preventDefault()
    event.stopPropagation()
    activatePreviewControl(activatable)

    window.setTimeout(() => {
        const nextScene = sceneFromPath(window.location.pathname)
        if (nextScene) {
            onNavigate(nextScene)
        }
    }, 120)

    return true
}

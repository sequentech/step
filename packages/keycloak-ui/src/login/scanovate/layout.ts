// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The layout of Scanovate's document page after the device is turned.
//
// That page fits its camera and the frame for the document to the window
// when the camera starts, and again 50 ms after each resize of its window. A
// tablet that is turned changes the window first; the camera's image takes
// its new shape a moment later, and the window may still be settling. The
// page is then left with a layout made of the old sizes: the camera as a
// small column in the middle of a wide window, or wider than a narrow one.
//
// So the page is asked to lay itself out again, the way the browser asks it
// on a resize, once a turn has settled and whenever the camera's image
// changes shape. Nothing of the page is changed: it gets one more of the
// events it already answers. This needs a page of this page's origin.

/** After a turn or a resize of the window: when to lay the page out again. */
export const TURN_SETTLE_MS = [300, 1000] as const
/** After the camera's image changed shape, once the page has drawn it. */
export const IMAGE_SETTLE_MS = 100

type Framed = Pick<HTMLIFrameElement, "contentWindow">

/**
 * Has the page in the frame lay itself out, as on a resize of its window.
 * False when there is no page within reach.
 */
export function layOutAgain(frame: Framed | null | undefined): boolean {
    try {
        const target = frame?.contentWindow
        if (target === null || target === undefined) return false
        target.dispatchEvent(new Event("resize"))
        return true
    } catch {
        return false
    }
}

function documentOf(frame: HTMLIFrameElement): Document | null {
    try {
        return frame.contentDocument
    } catch {
        return null
    }
}

/**
 * Keeps the page in the frame laid out for the window it has, through turns
 * of the device, in every page the frame loads. Returns how to stop.
 */
export function keepLaidOut(frame: HTMLIFrameElement, host: Window = window): () => void {
    const timers = new Set<number>()
    const later = (delay: number) => {
        const timer = host.setTimeout(() => {
            timers.delete(timer)
            layOutAgain(frame)
        }, delay)
        timers.add(timer)
    }
    const turned = () => TURN_SETTLE_MS.forEach(later)
    // A video tells its new size with a resize of its own, which does not
    // bubble: it is heard on its way down from the document.
    const imageChanged = () => later(IMAGE_SETTLE_MS)

    let watched: Document | null = null
    const unwatch = () => {
        watched?.removeEventListener("resize", imageChanged, true)
        watched = null
    }
    const watch = () => {
        unwatch()
        watched = documentOf(frame)
        watched?.addEventListener("resize", imageChanged, true)
    }

    watch()
    frame.addEventListener("load", watch)
    host.addEventListener("resize", turned)
    host.addEventListener("orientationchange", turned)
    return () => {
        frame.removeEventListener("load", watch)
        host.removeEventListener("resize", turned)
        host.removeEventListener("orientationchange", turned)
        unwatch()
        timers.forEach((timer) => host.clearTimeout(timer))
        timers.clear()
    }
}

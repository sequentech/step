// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {afterEach, describe, expect, it, vi} from "vitest"
import {IMAGE_SETTLE_MS, TURN_SETTLE_MS, keepLaidOut, layOutAgain} from "./layout"

// A page like Scanovate's document scan: it lays its camera out on a resize
// of its window, and shows the camera in a video.
async function scanPage() {
    const frame = document.createElement("iframe")
    frame.srcdoc = "<!doctype html><video></video>"
    const loaded = new Promise((resolve) => frame.addEventListener("load", resolve, {once: true}))
    document.body.append(frame)
    await loaded
    const layouts = vi.fn()
    frame.contentWindow!.addEventListener("resize", layouts)
    const video = () => frame.contentDocument!.querySelector("video")!
    return {frame, layouts, video}
}

afterEach(() => {
    vi.useRealTimers()
    document.querySelectorAll("iframe").forEach((frame) => frame.remove())
})

describe("layOutAgain", () => {
    it("has the page in the frame lay itself out, as on a resize of its window", async () => {
        const {frame, layouts} = await scanPage()
        expect(layOutAgain(frame)).toBe(true)
        expect(layouts).toHaveBeenCalledTimes(1)
    })

    it("says so when the frame is gone or out of reach", () => {
        expect(layOutAgain(null)).toBe(false)
        expect(layOutAgain({contentWindow: null})).toBe(false)
        const closed = {
            dispatchEvent: () => {
                throw new DOMException("cross-origin", "SecurityError")
            },
        } as unknown as Window
        expect(layOutAgain({contentWindow: closed})).toBe(false)
    })
})

describe("keepLaidOut", () => {
    it("lays the page out again once a turn of the device has settled", async () => {
        const {frame, layouts} = await scanPage()
        vi.useFakeTimers()
        const stop = keepLaidOut(frame)
        window.dispatchEvent(new Event("orientationchange"))
        expect(layouts).not.toHaveBeenCalled()
        vi.advanceTimersByTime(Math.max(...TURN_SETTLE_MS))
        expect(layouts).toHaveBeenCalledTimes(TURN_SETTLE_MS.length)
        stop()
    })

    it("lays the page out again when the window changes size", async () => {
        const {frame, layouts} = await scanPage()
        vi.useFakeTimers()
        const stop = keepLaidOut(frame)
        window.dispatchEvent(new Event("resize"))
        vi.advanceTimersByTime(Math.max(...TURN_SETTLE_MS))
        expect(layouts).toHaveBeenCalledTimes(TURN_SETTLE_MS.length)
        stop()
    })

    it("lays the page out again when the camera's image changes shape", async () => {
        // A turned tablet's camera gives its image the other way round a
        // moment after the window has its new size.
        const {frame, layouts, video} = await scanPage()
        vi.useFakeTimers()
        const stop = keepLaidOut(frame)
        video().dispatchEvent(new Event("resize"))
        expect(layouts).not.toHaveBeenCalled()
        vi.advanceTimersByTime(IMAGE_SETTLE_MS)
        expect(layouts).toHaveBeenCalledTimes(1)
        stop()
    })

    it("follows the frame to the next page it loads", async () => {
        const {frame} = await scanPage()
        const stop = keepLaidOut(frame)
        const loaded = new Promise((resolve) =>
            frame.addEventListener("load", resolve, {once: true})
        )
        frame.srcdoc = "<!doctype html><video></video><p>again</p>"
        await loaded
        const layouts = vi.fn()
        frame.contentWindow!.addEventListener("resize", layouts)
        vi.useFakeTimers()
        frame.contentDocument!.querySelector("video")!.dispatchEvent(new Event("resize"))
        vi.advanceTimersByTime(IMAGE_SETTLE_MS)
        expect(layouts).toHaveBeenCalledTimes(1)
        stop()
    })

    it("does nothing more once stopped", async () => {
        const {frame, layouts, video} = await scanPage()
        vi.useFakeTimers()
        const stop = keepLaidOut(frame)
        window.dispatchEvent(new Event("resize"))
        stop()
        video().dispatchEvent(new Event("resize"))
        window.dispatchEvent(new Event("orientationchange"))
        vi.advanceTimersByTime(10 * Math.max(...TURN_SETTLE_MS))
        expect(layouts).not.toHaveBeenCalled()
    })
})

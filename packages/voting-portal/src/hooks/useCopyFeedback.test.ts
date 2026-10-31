// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {act, renderHook} from "@testing-library/react"
import {COPIED_MS, MANUAL_MS, useCopyFeedback} from "./useCopyFeedback"

const setClipboard = (clipboard: unknown) =>
    Object.defineProperty(navigator, "clipboard", {value: clipboard, configurable: true})

beforeEach(() => {
    jest.useFakeTimers()
})

afterEach(() => {
    jest.useRealTimers()
    setClipboard(undefined)
    Reflect.deleteProperty(document, "execCommand")
})

describe("copying with feedback", () => {
    it("writes the text to the clipboard, and says so until the moment is over", async () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        setClipboard({writeText})
        const {result} = renderHook(() => useCopyFeedback())
        expect(result.current.copied).toBe(false)

        await act(async () => void result.current.copy("FTBE-MHRX"))
        expect(writeText).toHaveBeenCalledWith("FTBE-MHRX")
        expect(result.current.copied).toBe(true)

        act(() => jest.advanceTimersByTime(COPIED_MS - 1))
        expect(result.current.copied).toBe(true)
        act(() => jest.advanceTimersByTime(1))
        expect(result.current.copied).toBe(false)
    })

    it("starts the moment again when the voter copies again", async () => {
        setClipboard({writeText: jest.fn().mockResolvedValue(undefined)})
        const {result} = renderHook(() => useCopyFeedback())
        await act(async () => void result.current.copy("one"))
        act(() => jest.advanceTimersByTime(COPIED_MS - 100))
        await act(async () => void result.current.copy("two"))
        act(() => jest.advanceTimersByTime(COPIED_MS - 100))
        expect(result.current.copied).toBe(true)
        act(() => jest.advanceTimersByTime(100))
        expect(result.current.copied).toBe(false)
    })

    it("copies through the fallback where the Clipboard API refuses", async () => {
        setClipboard({writeText: jest.fn().mockRejectedValue(new Error("denied"))})
        Object.defineProperty(document, "execCommand", {configurable: true, value: () => true})
        const {result} = renderHook(() => useCopyFeedback())
        await act(async () => void result.current.copy("text"))
        expect(result.current.copied).toBe(true)
        expect(result.current.manual).toBe(false)
    })

    it("asks for a manual copy, with no error, where nothing can copy", async () => {
        const {result} = renderHook(() => useCopyFeedback())
        await act(async () => void result.current.copy("text"))
        expect(result.current.copied).toBe(false)
        expect(result.current.manual).toBe(true)

        act(() => jest.advanceTimersByTime(MANUAL_MS))
        expect(result.current.manual).toBe(false)
    })

    it("leaves no timer behind when it goes away", async () => {
        setClipboard({writeText: jest.fn().mockResolvedValue(undefined)})
        const {result, unmount} = renderHook(() => useCopyFeedback())
        await act(async () => void result.current.copy("text"))
        const running = jest.getTimerCount()
        unmount()
        // The moment's timer is the one that goes.
        expect(jest.getTimerCount()).toBe(running - 1)
    })
})

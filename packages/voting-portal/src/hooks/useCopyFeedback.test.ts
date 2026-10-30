// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {act, renderHook} from "@testing-library/react"
import {COPIED_MS, useCopyFeedback} from "./useCopyFeedback"

const setClipboard = (clipboard: unknown) =>
    Object.defineProperty(navigator, "clipboard", {value: clipboard, configurable: true})

beforeEach(() => {
    jest.useFakeTimers()
})

afterEach(() => {
    jest.useRealTimers()
    setClipboard(undefined)
})

describe("copying with feedback", () => {
    it("writes the text to the clipboard, and says so until the moment is over", () => {
        const writeText = jest.fn().mockResolvedValue(undefined)
        setClipboard({writeText})
        const {result} = renderHook(() => useCopyFeedback())
        expect(result.current.copied).toBe(false)

        act(() => result.current.copy("30ff d314"))
        expect(writeText).toHaveBeenCalledWith("30ff d314")
        expect(result.current.copied).toBe(true)

        act(() => jest.advanceTimersByTime(COPIED_MS - 1))
        expect(result.current.copied).toBe(true)
        act(() => jest.advanceTimersByTime(1))
        expect(result.current.copied).toBe(false)
    })

    it("starts the moment again when the voter copies again", () => {
        setClipboard({writeText: jest.fn().mockResolvedValue(undefined)})
        const {result} = renderHook(() => useCopyFeedback())
        act(() => result.current.copy("one"))
        act(() => jest.advanceTimersByTime(COPIED_MS - 100))
        act(() => result.current.copy("two"))
        act(() => jest.advanceTimersByTime(COPIED_MS - 100))
        expect(result.current.copied).toBe(true)
        act(() => jest.advanceTimersByTime(100))
        expect(result.current.copied).toBe(false)
    })

    it("does not fail where the clipboard is missing or refuses", async () => {
        const {result} = renderHook(() => useCopyFeedback())
        expect(() => act(() => result.current.copy("text"))).not.toThrow()

        setClipboard({writeText: jest.fn().mockRejectedValue(new Error("denied"))})
        await act(async () => result.current.copy("text"))
        expect(result.current.copied).toBe(true)
    })

    it("leaves no timer behind when it goes away", () => {
        setClipboard({writeText: jest.fn().mockResolvedValue(undefined)})
        const {result, unmount} = renderHook(() => useCopyFeedback())
        act(() => result.current.copy("text"))
        unmount()
        expect(jest.getTimerCount()).toBe(0)
    })
})

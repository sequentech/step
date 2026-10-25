/** @jest-environment jsdom */
// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {
    startSessionActivity,
    sessionActivityStorageKey,
    SESSION_IDLE_TIMEOUT_MS,
} from "./sessionActivity"

const key = "test-session-activity"
let input: (event: Event) => void
let onExpire: jest.Mock
let refresh: jest.Mock
let onRefreshed: jest.Mock
let cleanup: (() => void) | undefined

beforeEach(() => {
    jest.useFakeTimers()
    jest.setSystemTime(new Date("2026-10-25T12:00:00Z"))
    localStorage.clear()
    onExpire = jest.fn()
    refresh = jest.fn().mockResolvedValue(true)
    onRefreshed = jest.fn()
    const original = window.addEventListener.bind(window)
    jest.spyOn(window, "addEventListener").mockImplementation((type, listener, options) => {
        if (type === "pointerdown") input = listener as (event: Event) => void
        original(type, listener, options)
    })
})
afterEach(() => {
    cleanup?.()
    cleanup = undefined
    jest.restoreAllMocks()
    jest.useRealTimers()
})
const start = () => {
    const session = startSessionActivity({storageKey: key, refresh, onRefreshed, onExpire})
    cleanup = session.dispose
    return session
}
const interact = () => {
    const event = {isTrusted: true, preventDefault: jest.fn(), stopImmediatePropagation: jest.fn()}
    input(event as unknown as Event)
    return event
}

it("expires after fifteen minutes, without background polling refreshing an idle session", async () => {
    start()
    await jest.advanceTimersByTimeAsync(SESSION_IDLE_TIMEOUT_MS - 1)
    expect(refresh).not.toHaveBeenCalled()
    expect(onExpire).not.toHaveBeenCalled()
    await jest.advanceTimersByTimeAsync(1)
    expect(onExpire).toHaveBeenCalledTimes(1)
    expect(jest.getTimerCount()).toBe(0)
})

it("keeps an interacting user signed in but does not refresh twice for the same input", async () => {
    start()
    for (let minute = 0; minute < 20; minute++) {
        await jest.advanceTimersByTimeAsync(60_000)
        interact()
        await Promise.resolve()
    }
    expect(onExpire).not.toHaveBeenCalled()
    expect(refresh).toHaveBeenCalledTimes(20)
    await jest.advanceTimersByTimeAsync(60_000)
    expect(refresh).toHaveBeenCalledTimes(20)
})

it("does not count synthetic events or visibility/focus as user interaction", async () => {
    start()
    await jest.advanceTimersByTimeAsync(14 * 60_000)
    window.dispatchEvent(new Event("pointerdown"))
    window.dispatchEvent(new Event("focus"))
    document.dispatchEvent(new Event("visibilitychange"))
    await jest.advanceTimersByTimeAsync(60_000)
    expect(refresh).not.toHaveBeenCalled()
    expect(onExpire).toHaveBeenCalledTimes(1)
})

it("expires on resume after timers were suspended, before accepting the first input", () => {
    start()
    jest.setSystemTime(Date.now() + SESSION_IDLE_TIMEOUT_MS)
    const event = interact()
    expect(event.preventDefault).toHaveBeenCalledTimes(1)
    expect(event.stopImmediatePropagation).toHaveBeenCalledTimes(1)
    expect(onExpire).toHaveBeenCalledTimes(1)
    expect(refresh).not.toHaveBeenCalled()
})

it("checks the deadline synchronously before token access or a requested refresh", async () => {
    const session = start()
    jest.setSystemTime(Date.now() + SESSION_IDLE_TIMEOUT_MS)
    expect(session.isActive()).toBe(false)
    await session.refresh()
    expect(refresh).not.toHaveBeenCalled()
    expect(onExpire).toHaveBeenCalledTimes(1)
})

it("uses activity from another tab without treating a reload as fresh activity", async () => {
    start()
    await jest.advanceTimersByTimeAsync(10 * 60_000)
    localStorage.setItem(key, String(Date.now()))
    window.dispatchEvent(new StorageEvent("storage", {key, newValue: String(Date.now())}))
    cleanup?.()
    start()
    await jest.advanceTimersByTimeAsync(14 * 60_000)
    expect(onExpire).not.toHaveBeenCalled()
    await jest.advanceTimersByTimeAsync(60_000)
    expect(onExpire).toHaveBeenCalledTimes(1)
})

it("reads cross-tab activity before checking a suspended tab's deadline", () => {
    start()
    const initial = Date.now()
    jest.setSystemTime(initial + 20 * 60_000)
    localStorage.setItem(key, String(initial + 19 * 60_000))
    window.dispatchEvent(new Event("focus"))
    expect(onExpire).not.toHaveBeenCalled()
})

it("expires a reloaded idle session immediately", () => {
    localStorage.setItem(key, String(Date.now() - SESSION_IDLE_TIMEOUT_MS))
    start()
    expect(onExpire).toHaveBeenCalledTimes(1)
    expect(refresh).not.toHaveBeenCalled()
})

it("removes listeners and timers, and ignores an in-flight refresh after disposal", async () => {
    const removeWindow = jest.spyOn(window, "removeEventListener")
    const removeDocument = jest.spyOn(document, "removeEventListener")
    let finish: (value: boolean) => void = () => {}
    refresh.mockImplementation(
        () =>
            new Promise<boolean>((resolve) => {
                finish = resolve
            })
    )
    const session = start()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    session.dispose()
    finish(true)
    await Promise.resolve()
    expect(onRefreshed).not.toHaveBeenCalled()
    expect(jest.getTimerCount()).toBe(0)
    expect(removeWindow).toHaveBeenCalledWith("pointerdown", expect.any(Function), true)
    expect(removeWindow).toHaveBeenCalledWith("storage", expect.any(Function))
    expect(removeDocument).toHaveBeenCalledWith("visibilitychange", expect.any(Function))
})

it("fails closed on refresh rejection without exposing the error", async () => {
    refresh.mockRejectedValue(new Error("private transport detail"))
    start()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    await Promise.resolve()
    await Promise.resolve()
    expect(onExpire).toHaveBeenCalledTimes(1)
    expect(onRefreshed).not.toHaveBeenCalled()
})

it("retains activity until the token actually needs refreshing", async () => {
    refresh.mockResolvedValueOnce(false).mockResolvedValueOnce(true)
    start()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    await Promise.resolve()
    await jest.advanceTimersByTimeAsync(50_000)
    expect(refresh).toHaveBeenCalledTimes(2)
    await jest.advanceTimersByTimeAsync(50_000)
    expect(refresh).toHaveBeenCalledTimes(2)
})

it("does not consume input that arrives while a refresh is pending", async () => {
    let finish: (value: boolean) => void = () => {}
    refresh.mockImplementationOnce(
        () =>
            new Promise<boolean>((resolve) => {
                finish = resolve
            })
    )
    start()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    expect(refresh).toHaveBeenCalledTimes(1)
    finish(true)
    await Promise.resolve()
    await jest.advanceTimersByTimeAsync(50_000)
    expect(refresh).toHaveBeenCalledTimes(2)
})

it.each(["invalid", "", "0", "Infinity", "99999999999999"])(
    "fails closed when the stored deadline is corrupt or in the future: %s",
    (value) => {
        localStorage.setItem(key, value)
        start()
        expect(onExpire).toHaveBeenCalledTimes(1)
        localStorage.setItem(key, String(Date.now()))
        interact()
        expect(refresh).not.toHaveBeenCalled()
    }
)

it("namespaces activity by realm, person and authenticated session", () => {
    const current = sessionActivityStorageKey("realm-a", "person-a", "session-a")
    expect(current).not.toEqual(sessionActivityStorageKey("realm-b", "person-a", "session-a"))
    expect(current).not.toEqual(sessionActivityStorageKey("realm-a", "person-b", "session-a"))
    expect(current).not.toEqual(sessionActivityStorageKey("realm-a", "person-a", "session-b"))
    expect(sessionActivityStorageKey("realm-a", "person-a", undefined)).toBeUndefined()
})

it("does not publish a token returned after the inactivity deadline", async () => {
    let finish: (value: boolean) => void = () => {}
    refresh.mockImplementation(
        () =>
            new Promise<boolean>((resolve) => {
                finish = resolve
            })
    )
    start()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    jest.setSystemTime(Date.now() + SESSION_IDLE_TIMEOUT_MS)
    finish(true)
    await Promise.resolve()
    expect(onRefreshed).not.toHaveBeenCalled()
    expect(onExpire).toHaveBeenCalledTimes(1)
})

it("authorizes one explicit post-action refresh only after real input", async () => {
    const session = start()
    await session.captureActionRefresh()()
    expect(refresh).not.toHaveBeenCalled()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    const completeAction = session.captureActionRefresh()
    await completeAction()
    expect(refresh).toHaveBeenLastCalledWith(true)
    const count = refresh.mock.calls.length
    await completeAction()
    await session.captureActionRefresh()()
    expect(refresh).toHaveBeenCalledTimes(count)
})

it("rejects a captured action refresh after the session expires", async () => {
    const session = start()
    jest.setSystemTime(Date.now() + 1000)
    interact()
    const completeAction = session.captureActionRefresh()
    await Promise.resolve()
    const count = refresh.mock.calls.length
    jest.setSystemTime(Date.now() + SESSION_IDLE_TIMEOUT_MS)
    await completeAction()
    expect(refresh).toHaveBeenCalledTimes(count)
    expect(onExpire).toHaveBeenCalledTimes(1)
})

it("still blocks stale input when the logout adapter throws synchronously", () => {
    onExpire.mockImplementation(() => {
        throw new Error("adapter failure")
    })
    start()
    jest.setSystemTime(Date.now() + SESSION_IDLE_TIMEOUT_MS)
    const event = interact()
    expect(event.stopImmediatePropagation).toHaveBeenCalledTimes(1)
    expect(refresh).not.toHaveBeenCalled()
})

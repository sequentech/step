// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect} from "react"

// Keycloak rejects a login form submitted after the realm's login timeout and makes the voter
// start again. Ask Keycloak how long this login has left, and when the page is used after that,
// start the login over before the voter types, keeping the username they had already typed.
// Mirrors sequent-theme's login-timeout.js for the FreeMarker pages.

// Covers the time the form takes to reach Keycloak.
const SUBMIT_MARGIN_MS = 5 * 1000
export const RESTART_KEY = "sequent.loginTimeoutRestart"
// A restart older than this belongs to some other page and is ignored.
const RESTART_MAX_AGE_MS = 60 * 1000
// When Keycloak can't be reached, ask again after this long.
const RETRY_MS = 30 * 1000
const AUTH_ENDPOINT_PATH = /\/protocol\/openid-connect\/auth$/
const LOGIN_ACTIONS_PATH = "/login-actions/"
const STATUS_PATH = "/login-timeout"
// Only fields that identify the voter come back after a restart: never secrets, codes or
// personal data.
export const KEPT_FIELD_NAMES = ["username", "email"]
const LOGIN_PARAMS = ["client_id", "tab_id"]

export enum LoginRestart {
    // First page of the flow: reloading the authorization request opens a fresh login without
    // any notice.
    Reload = "RELOAD",
    // A form result or a later step: reloading would resubmit the form, so use Keycloak's restart
    // endpoint, whose "timed out" notice the next page hides.
    RestartFlow = "RESTART_FLOW",
}

type SavedRestart = {
    savedAt: number
    hideNotice: boolean
    values: Record<string, unknown>
}

export type LoginTimeoutUrls = {
    restart: URL
    status: URL
}

/**
 * The URLs to restart and to ask about the login that `loginActionUrl` submits to. Both name
 * this page's client and tab, so Keycloak doesn't fall back to whichever login the browser
 * started last. Null when `restartUrl` isn't one of Keycloak's login actions, as in previews,
 * since there is no realm to ask.
 */
export function loginTimeoutUrls(
    restartUrl: string,
    loginActionUrl: string,
    base: string
): LoginTimeoutUrls | null {
    const loginAction = new URL(loginActionUrl || base, base)
    const restart = new URL(restartUrl, base)
    const loginActionsAt = restart.pathname.indexOf(LOGIN_ACTIONS_PATH)
    if (loginActionsAt < 0) {
        return null
    }
    LOGIN_PARAMS.forEach((param) => {
        const value = loginAction.searchParams.get(param)
        if (value) {
            restart.searchParams.set(param, value)
        }
    })
    const realmPath = restart.pathname.slice(0, loginActionsAt)
    const status = new URL(`${realmPath}${STATUS_PATH}`, restart)
    const tabId = loginAction.searchParams.get("tab_id")
    if (tabId) {
        status.searchParams.set("tab_id", tabId)
    }
    return {restart, status}
}

export function loginRestartOf(pathname: string): LoginRestart {
    return AUTH_ENDPOINT_PATH.test(pathname) ? LoginRestart.Reload : LoginRestart.RestartFlow
}

export function sameMessage(a: string, b: string): boolean {
    // Keycloakify's bundled English differs from the server's in whitespace.
    const normalize = (text: string) => text.replace(/\s+/g, " ").trim()
    return normalize(a) === normalize(b)
}

function keptFields(): HTMLInputElement[] {
    return KEPT_FIELD_NAMES.map((name) =>
        document.querySelector<HTMLInputElement>(`form input[name="${name}"]`)
    ).filter(
        (field): field is HTMLInputElement =>
            field !== null && !field.disabled && field.type !== "password"
    )
}

function takeSavedRestart(): SavedRestart | null {
    try {
        const saved = JSON.parse(
            sessionStorage.getItem(RESTART_KEY) ?? "null"
        ) as SavedRestart | null
        sessionStorage.removeItem(RESTART_KEY)
        return saved && Date.now() - saved.savedAt < RESTART_MAX_AGE_MS ? saved : null
    } catch {
        return null
    }
}

function setValue(field: HTMLInputElement, value: string) {
    // Go through the native setter so React notices the change.
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(field, value)
    field.dispatchEvent(new Event("input", {bubbles: true}))
    field.dispatchEvent(new Event("change", {bubbles: true}))
}

// Read once per page load, before any component mounts or remounts.
const savedRestart = typeof window === "undefined" ? null : takeSavedRestart()

/**
 * Restarts the login once Keycloak would reject its next form. Returns whether this page comes
 * from such a restart, so the caller can hide Keycloak's "timed out" notice.
 */
export function useLoginTimeout(restartUrl: string, loginActionUrl: string): boolean {
    useEffect(() => {
        const values = savedRestart?.values ?? {}
        keptFields().forEach((field) => {
            const value = values[field.name]
            if (!field.value && typeof value === "string") {
                setValue(field, value)
            }
        })
    }, [])

    useEffect(() => {
        const urls = loginTimeoutUrls(restartUrl, loginActionUrl, location.href)
        if (!urls) {
            return
        }
        let deadline: number | null = null
        let deadlineTimer: ReturnType<typeof setTimeout> | undefined
        let checking = false
        let restarting = false
        // Cleared when a page that a restart just opened is already expired: restarting again
        // would loop, so that is left to Keycloak.
        let mayRestart = true
        let firstCheck = true

        const expired = () => deadline !== null && Date.now() >= deadline

        const restart = () => {
            if (restarting || !mayRestart) {
                return
            }
            restarting = true
            const kind = loginRestartOf(location.pathname)
            const values: Record<string, string> = {}
            keptFields().forEach((field) => {
                if (field.value) {
                    values[field.name] = field.value
                }
            })
            try {
                const saved: SavedRestart = {
                    savedAt: Date.now(),
                    hideNotice: kind === LoginRestart.RestartFlow,
                    values,
                }
                sessionStorage.setItem(RESTART_KEY, JSON.stringify(saved))
            } catch {
                // Storage blocked: the voter types again, as without this hook.
            }
            if (kind === LoginRestart.Reload) {
                location.reload()
            } else {
                // Keeps any session the voter is already signed in with.
                location.replace(urls.restart)
            }
        }

        // Asks Keycloak again each time, since another tab of the same login also extends it.
        // When Keycloak has no login for this page, it is left alone. Until Keycloak answers, the
        // last deadline it gave still applies.
        const check = async () => {
            if (restarting || checking || document.visibilityState !== "visible") {
                return
            }
            checking = true
            try {
                const response = await fetch(urls.status, {
                    credentials: "same-origin",
                    cache: "no-store",
                })
                if (response.status !== 204 && !response.ok) {
                    throw new Error(`login-timeout answered ${response.status}`)
                }
                const status =
                    response.status === 204
                        ? null
                        : ((await response.json()) as {secondsLeft?: unknown})
                clearTimeout(deadlineTimer)
                if (
                    typeof status?.secondsLeft !== "number" ||
                    !Number.isFinite(status.secondsLeft)
                ) {
                    deadline = null
                    return
                }
                deadline = Date.now() + status.secondsLeft * 1000 - SUBMIT_MARGIN_MS
                if (!expired()) {
                    deadlineTimer = setTimeout(() => void check(), deadline - Date.now())
                } else if (firstCheck && savedRestart) {
                    mayRestart = false
                } else {
                    restart()
                }
            } catch {
                // A failed request must not switch the check off: the timer that replaces it also
                // works with no deadline yet.
                clearTimeout(deadlineTimer)
                deadlineTimer = setTimeout(() => void check(), RETRY_MS)
            } finally {
                checking = false
                firstCheck = false
            }
        }
        const onCheck = () => void check()
        const onPageShow = (event: PageTransitionEvent) => {
            if (event.persisted) {
                onCheck()
            }
        }
        // Last resort: a form Keycloak would reject must never be sent.
        const onSubmit = (event: SubmitEvent) => {
            if (expired() && mayRestart) {
                event.preventDefault()
                event.stopImmediatePropagation()
                restart()
            }
        }

        onCheck()
        document.addEventListener("visibilitychange", onCheck)
        window.addEventListener("focus", onCheck)
        window.addEventListener("pageshow", onPageShow)
        document.addEventListener("submit", onSubmit, true)
        return () => {
            clearTimeout(deadlineTimer)
            document.removeEventListener("visibilitychange", onCheck)
            window.removeEventListener("focus", onCheck)
            window.removeEventListener("pageshow", onPageShow)
            document.removeEventListener("submit", onSubmit, true)
        }
    }, [restartUrl, loginActionUrl])

    return savedRestart?.hideNotice ?? false
}

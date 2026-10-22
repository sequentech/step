// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect} from "react"

// Keycloak rejects a login form submitted after the realm's login timeout and makes the voter
// start again. When a page that has been open longer than that becomes active again, start the
// login over before they type. Mirrors sequent-theme's login-timeout.js for the FreeMarker pages.

const SAFETY_MARGIN_MS = 60 * 1000
const CHECK_INTERVAL_MS = 30 * 1000
const DEFAULT_LOGIN_TIMEOUT_SECONDS = 1800
const DEFAULT_ACTION_TIMEOUT_SECONDS = 300
export const RESTART_FLAG = "sequent.loginTimeoutRestart"
// Keycloak checks these pages against the shorter "login action timeout".
const USER_ACTION_PATH = /\/login-actions\/(required-action|reset-credentials)$/
const AUTH_ENDPOINT_PATH = /\/protocol\/openid-connect\/auth$/

export enum LoginRestart {
    // First page of the flow: reloading the authorization request opens a fresh login session
    // without any notice.
    Reload = "RELOAD",
    // A form result or a later step: reloading would resubmit the form, so use Keycloak's restart
    // endpoint, which shows a "timed out" notice that the next page hides.
    RestartFlow = "RESTART_FLOW",
}

export type LoginTimeoutSettings = {
    loginTimeoutSeconds?: string
    loginActionTimeoutSeconds?: string
}

function seconds(value: string | undefined, fallback: number): number {
    const parsed = Number(value)
    return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

export function loginTimeoutLimitMs(pathname: string, settings: LoginTimeoutSettings): number {
    const timeout = USER_ACTION_PATH.test(pathname)
        ? seconds(settings.loginActionTimeoutSeconds, DEFAULT_ACTION_TIMEOUT_SECONDS)
        : seconds(settings.loginTimeoutSeconds, DEFAULT_LOGIN_TIMEOUT_SECONDS)
    return Math.max(timeout * 1000 - SAFETY_MARGIN_MS, SAFETY_MARGIN_MS)
}

export function loginRestartOf(pathname: string): LoginRestart {
    return AUTH_ENDPOINT_PATH.test(pathname) ? LoginRestart.Reload : LoginRestart.RestartFlow
}

export function sameMessage(a: string, b: string): boolean {
    // Keycloakify's bundled English differs from the server's in whitespace.
    const normalize = (text: string) => text.replace(/\s+/g, " ").trim()
    return normalize(a) === normalize(b)
}

function takeRestartFlag(): boolean {
    try {
        const restarted = sessionStorage.getItem(RESTART_FLAG) !== null
        sessionStorage.removeItem(RESTART_FLAG)
        return restarted
    } catch {
        return false
    }
}

/**
 * Restarts the login once the page outlives the login timeout. Returns whether this page comes
 * from such a restart, so the caller can hide Keycloak's "timed out" notice.
 */
// Read once per page load, before any component mounts or remounts.
const restartedByTimeout = takeRestartFlag()

export function useLoginTimeout(restartUrl: string, settings: LoginTimeoutSettings): boolean {
    const {loginTimeoutSeconds, loginActionTimeoutSeconds} = settings

    useEffect(() => {
        const renderedAt = Date.now()
        const limitMs = loginTimeoutLimitMs(location.pathname, {
            loginTimeoutSeconds,
            loginActionTimeoutSeconds,
        })
        let restarting = false

        const restartIfExpired = (): boolean => {
            if (restarting || Date.now() - renderedAt < limitMs) {
                return false
            }
            restarting = true
            if (loginRestartOf(location.pathname) === LoginRestart.Reload) {
                location.reload()
            } else {
                try {
                    sessionStorage.setItem(RESTART_FLAG, "1")
                } catch {
                    // Storage blocked: the notice stays visible, which is harmless.
                }
                location.replace(restartUrl)
            }
            return true
        }
        const onVisibilityChange = () => {
            if (document.visibilityState === "visible") {
                restartIfExpired()
            }
        }
        const onPageShow = (event: PageTransitionEvent) => {
            if (event.persisted) {
                restartIfExpired()
            }
        }
        // Timers are throttled in background tabs and stop while the computer sleeps, so this
        // only covers a page left in view.
        const interval = setInterval(onVisibilityChange, CHECK_INTERVAL_MS)
        // Last resort: an expired page must never submit.
        const onSubmit = (event: SubmitEvent) => {
            if (restartIfExpired()) {
                event.preventDefault()
                event.stopImmediatePropagation()
            }
        }

        document.addEventListener("visibilitychange", onVisibilityChange)
        window.addEventListener("focus", restartIfExpired)
        window.addEventListener("pageshow", onPageShow)
        document.addEventListener("submit", onSubmit, true)
        return () => {
            clearInterval(interval)
            document.removeEventListener("visibilitychange", onVisibilityChange)
            window.removeEventListener("focus", restartIfExpired)
            window.removeEventListener("pageshow", onPageShow)
            document.removeEventListener("submit", onSubmit, true)
        }
    }, [restartUrl, loginTimeoutSeconds, loginActionTimeoutSeconds])

    return restartedByTimeout
}

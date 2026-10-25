// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

export const SESSION_IDLE_TIMEOUT_MS = 15 * 60 * 1000
const REFRESH_INTERVAL_MS = 50_000
const INPUT_EVENTS = ["pointerdown", "pointermove", "keydown", "wheel", "touchstart", "input"]

export interface SessionActivityOptions {
    /** A non-secret namespace unique to the authenticated realm, user and session. */
    storageKey?: string
    refresh: (force: boolean) => Promise<boolean>
    onRefreshed: () => void
    onExpire: () => void | Promise<void>
}

export interface SessionActivity {
    isActive: () => boolean
    refresh: () => Promise<void>
    dispose: () => void
    captureActionRefresh: () => () => Promise<void>
}

/** Missing session identity disables cross-tab persistence instead of sharing another login. */
export function sessionActivityStorageKey(realm?: string, subject?: string, session?: unknown) {
    const validSession =
        typeof session === "string"
            ? session.length > 0
            : typeof session === "number" && Number.isFinite(session) && session > 0
    if (!realm || !subject || !validSession) {
        return undefined
    }
    return `sequent-session-activity:${JSON.stringify([realm, subject, session])}`
}

/**
 * Polling, navigation and token refresh are not user interaction. Persist only
 * the activity time, never credentials. An absolute deadline also covers tabs
 * whose timers were suspended by the browser.
 */
export function startSessionActivity(options: SessionActivityOptions): SessionActivity {
    let stopped = false
    let pending: Promise<void> | undefined
    let timer: ReturnType<typeof setTimeout> | undefined
    let lastActivity = Date.now()
    let lastRefreshedActivity = lastActivity
    let lastAuthorizedActivity = lastActivity

    const storedActivity = (): number | undefined => {
        if (!options.storageKey) return undefined
        try {
            const value = localStorage.getItem(options.storageKey)
            if (value === null) return undefined
            const parsed = Number(value)
            // Corrupt or future values fail closed; they never grant a new deadline.
            return value.trim() && Number.isFinite(parsed) && parsed > 0 && parsed <= Date.now()
                ? parsed
                : 0
        } catch {
            // Disabled storage still permits a tab-local inactivity deadline.
            return undefined
        }
    }
    const persist = () => {
        if (!options.storageKey) return
        try {
            localStorage.setItem(options.storageKey, String(lastActivity))
        } catch {
            // Keep enforcing the local deadline when storage is unavailable.
        }
    }
    const existing = storedActivity()
    if (existing !== undefined) lastActivity = existing
    else persist()
    // A remount must not earn a refresh or extend an existing deadline.
    lastRefreshedActivity = lastActivity

    const dispose = () => {
        stopped = true
        clearTimeout(timer)
        INPUT_EVENTS.forEach((event) => window.removeEventListener(event, activity, true))
        window.removeEventListener("storage", resume)
        window.removeEventListener("focus", resume)
        window.removeEventListener("pageshow", resume)
        document.removeEventListener("visibilitychange", resume)
    }
    const expire = () => {
        if (stopped) return
        dispose()
        // The provider clears local authentication before initiating server logout.
        // A failed redirect must not resume refresh or log transport/token details.
        try {
            Promise.resolve(options.onExpire()).catch(() => {})
        } catch {
            // Local expiry remains final even if the logout adapter throws.
        }
    }
    const isActive = () => {
        if (stopped) return false
        const stored = storedActivity()
        if (stored === 0) {
            expire()
            return false
        }
        if (stored !== undefined) lastActivity = Math.max(lastActivity, stored)
        const elapsed = Date.now() - lastActivity
        if (elapsed < 0 || elapsed >= SESSION_IDLE_TIMEOUT_MS) {
            expire()
            return false
        }
        return true
    }
    const refreshForActivity = async (force = false, authorizedActivity?: number) => {
        if (!isActive()) return
        if (pending) {
            await pending
            if (!force) return
            if (!isActive()) return
        }
        if (!force && lastActivity <= lastRefreshedActivity) return
        const activityAtStart = authorizedActivity ?? lastActivity
        const request = (async () => {
            try {
                const refreshed = await options.refresh(force)
                if (!isActive()) return
                if (refreshed) {
                    // Do not consume newer input received while the request was pending.
                    lastRefreshedActivity = Math.max(lastRefreshedActivity, activityAtStart)
                    options.onRefreshed()
                }
            } catch {
                expire()
            }
        })()
        pending = request
        await request
        if (pending === request) pending = undefined
    }
    const refresh = () => refreshForActivity()
    // A completed user mutation can require fresh permission claims even when
    // the token is still valid. Capture a one-use authorization at the start of
    // that action; background work cannot mint one, and expiry still wins.
    const captureActionRefresh = () => {
        if (!isActive() || lastActivity <= lastAuthorizedActivity) return async () => {}
        lastAuthorizedActivity = lastActivity
        const authorizedActivity = lastActivity
        let used = false
        return async () => {
            if (used) return
            used = true
            await refreshForActivity(true, authorizedActivity)
        }
    }
    const schedule = () => {
        clearTimeout(timer)
        if (!isActive()) return
        timer = setTimeout(
            () => {
                void refresh()
                schedule()
            },
            Math.min(REFRESH_INTERVAL_MS, lastActivity + SESSION_IDLE_TIMEOUT_MS - Date.now())
        )
    }
    function activity(event: Event) {
        if (!event.isTrusted) return
        if (!isActive()) {
            // A resumed tab must not dispatch its stale, token-bearing action.
            event.preventDefault()
            event.stopImmediatePropagation()
            return
        }
        lastActivity = Date.now()
        persist()
        void refresh()
        schedule()
    }
    function resume() {
        if (!isActive()) return
        void refresh()
        schedule()
    }
    INPUT_EVENTS.forEach((event) => window.addEventListener(event, activity, true))
    window.addEventListener("storage", resume)
    window.addEventListener("focus", resume)
    window.addEventListener("pageshow", resume)
    document.addEventListener("visibilitychange", resume)
    schedule()
    return {isActive, refresh, dispose, captureActionRefresh}
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useRef} from "react"
import {useBlocker, useCanBlock} from "react-admin"

/** Called when navigation is held: `proceed` completes it, `reset` cancels it. */
export type OnBlocked = (proceed: () => void, reset: () => void) => void

interface Props {
    active: boolean
    onBlocked: OnBlocked
}

const RouterBlocker: React.FC<Props> = ({active, onBlocked}) => {
    // One blocker for the component's life, reading the latest render: a
    // navigation right after the first edit is held even before effects run.
    const activeNow = useRef(active)
    activeNow.current = active
    const blocker = useBlocker(useCallback(() => activeNow.current, []))
    const latest = useRef(onBlocked)
    latest.current = onBlocked
    useEffect(() => {
        if (blocker.state === "blocked") {
            latest.current(
                () => blocker.proceed?.(),
                () => blocker.reset?.()
            )
        }
        // Only a change of blocker state is a new navigation attempt.
    }, [blocker.state])
    return null
}

/**
 * While `active`, holds in-app navigation (menu links, browser Back and
 * Forward) for the caller to confirm, and asks the browser to confirm closing
 * or reloading the tab. In-app blocking needs the portal's data router; where
 * it is absent (isolated tests and stories) only the unload prompt applies.
 */
export const NavigationGuard: React.FC<Props> = ({active, onBlocked}) => {
    const canBlock = useCanBlock()
    useEffect(() => {
        if (!active) return
        const warn = (event: BeforeUnloadEvent) => {
            event.preventDefault()
            event.returnValue = ""
        }
        window.addEventListener("beforeunload", warn)
        return () => window.removeEventListener("beforeunload", warn)
    }, [active])
    return canBlock ? <RouterBlocker active={active} onBlocked={onBlocked} /> : null
}

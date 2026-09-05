// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useRef} from "react"

/** Invalidates in-flight plaintext responses on context change or unmount. */
export function useSecretRevealGuard(context: string, allowed: boolean) {
    const scope = useRef({context, allowed})
    if (scope.current.context !== context || scope.current.allowed !== allowed) {
        scope.current = {context, allowed}
    }
    useEffect(
        () => () => {
            scope.current = {...scope.current}
        },
        []
    )
    return () => {
        const requestScope = scope.current
        return () => requestScope.allowed && scope.current === requestScope
    }
}

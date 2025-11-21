// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"

export const useForwardedRef = function <T>(ref: React.ForwardedRef<T>) {
    const innerRef = React.useRef<T>(null)

    React.useEffect(() => {
        if (!ref) return
        if (typeof ref === "function") {
            ref(innerRef.current)
        } else {
            ref.current = innerRef.current
        }
    }, [ref])

    return innerRef
}

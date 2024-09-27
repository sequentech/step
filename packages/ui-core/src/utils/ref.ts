// SPDX-FileCopyrightText: 2022-2023 Félix Robles <felix@sequentech.io>
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

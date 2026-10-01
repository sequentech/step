// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {useEffect, useState} from "react"

/** Whether the browser tab is shown; polling stops while it is hidden. */
export function usePageVisible(): boolean {
    const [visible, setVisible] = useState(() => document.visibilityState !== "hidden")
    useEffect(() => {
        const update = () => setVisible(document.visibilityState !== "hidden")
        document.addEventListener("visibilitychange", update)
        return () => document.removeEventListener("visibilitychange", update)
    }, [])
    return visible
}

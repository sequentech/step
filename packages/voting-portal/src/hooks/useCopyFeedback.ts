// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useEffect, useRef, useState} from "react"

/** How long a copy button shows that it copied. */
export const COPIED_MS = 1600

const writeToClipboard = async (text: string): Promise<void> => {
    try {
        await navigator.clipboard?.writeText(text)
    } catch {
        // The voter can still select the text.
    }
}

export interface ICopyFeedback {
    /** Whether text was copied a moment ago. */
    copied: boolean
    copy: (text: string) => void
}

/** Copies text to the clipboard and says so for a moment: the icon of a copy
 * button swaps for a check. */
export const useCopyFeedback = (): ICopyFeedback => {
    const [copied, setCopied] = useState(false)
    const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    useEffect(() => () => clearTimeout(timer.current), [])

    const copy = useCallback((text: string) => {
        void writeToClipboard(text)
        clearTimeout(timer.current)
        setCopied(true)
        timer.current = setTimeout(() => setCopied(false), COPIED_MS)
    }, [])

    return {copied, copy}
}

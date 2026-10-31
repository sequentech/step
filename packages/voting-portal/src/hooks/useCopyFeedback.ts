// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback, useEffect, useRef, useState} from "react"
import {copyBallotHash, CopyBallotHashStatus} from "@sequentech/ui-essentials"

/** How long a copy button shows that it copied. */
export const COPIED_MS = 1600
/** How long the voter is told to copy by hand: long enough to press the keys. */
export const MANUAL_MS = 8000

export interface ICopyFeedback {
    /** Whether text was copied a moment ago. */
    copied: boolean
    /** Whether the browser refused to copy a moment ago: the voter copies by
     * hand, which is no error. */
    manual: boolean
    copy: (text: string) => Promise<CopyBallotHashStatus>
}

/** Copies text to the clipboard and says so for a moment: the icon of a copy
 * button swaps for a check. Where the browser refuses the Clipboard API and
 * the fallback, `manual` says that the voter has to copy the text. */
export const useCopyFeedback = (): ICopyFeedback => {
    const [status, setStatus] = useState<CopyBallotHashStatus>(CopyBallotHashStatus.Idle)
    const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined)
    const mounted = useRef(true)
    useEffect(() => {
        mounted.current = true
        return () => {
            mounted.current = false
            clearTimeout(timer.current)
        }
    }, [])

    const copy = useCallback(async (text: string) => {
        const result = await copyBallotHash(text, navigator.clipboard)
        if (!mounted.current) return result
        clearTimeout(timer.current)
        setStatus(result)
        timer.current = setTimeout(
            () => setStatus(CopyBallotHashStatus.Idle),
            result === CopyBallotHashStatus.Manual ? MANUAL_MS : COPIED_MS
        )
        return result
    }, [])

    return {
        copied: status === CopyBallotHashStatus.Copied,
        manual: status === CopyBallotHashStatus.Manual,
        copy,
    }
}

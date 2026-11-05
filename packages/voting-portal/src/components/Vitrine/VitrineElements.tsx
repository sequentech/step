// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// React wrapper for the Vitrine UI kit's status element. The kit takes its
// message as a property, so the wrapper sets it through a ref. It renders the
// element only once the kit has defined it: a property set on an element before
// it is upgraded would hide the element's own setter.

import React, {useEffect, useRef, useState} from "react"
import type {OverallStatus, VitrineStatus} from "@vitrine/ui"
import {loadVitrineElements} from "../../services/vitrine/sdk"

type TElementProps<T extends HTMLElement> = React.DetailedHTMLProps<React.HTMLAttributes<T>, T>

declare module "react" {
    namespace JSX {
        interface IntrinsicElements {
            "vitrine-status": TElementProps<VitrineStatus> & {state?: OverallStatus}
        }
    }
}

/** True once the kit's elements are defined; false while loading or when
 * the kit couldn't be loaded. */
export const useVitrineElements = (): boolean => {
    const [ready, setReady] = useState(false)
    useEffect(() => {
        let active = true
        loadVitrineElements().then(
            () => {
                if (active) setReady(true)
            },
            () => {
                if (active) setReady(false)
            }
        )
        return () => {
            active = false
        }
    }, [])
    return ready
}

export interface IVitrineStatusViewProps {
    state: OverallStatus
    /** The page's own words, shown after the kit's status word. */
    message?: string
    lang: string
    className?: string
    /** Shown instead while the kit loads, or if it can't be loaded. */
    fallback?: React.ReactNode
}

export const VitrineStatusView: React.FC<IVitrineStatusViewProps> = ({
    state,
    message,
    lang,
    className,
    fallback,
}) => {
    const ready = useVitrineElements()
    const ref = useRef<VitrineStatus | null>(null)
    useEffect(() => {
        if (ref.current) ref.current.message = message ?? ""
    }, [ready, message])
    if (!ready) return <>{fallback ?? null}</>
    return <vitrine-status ref={ref} state={state} lang={lang} className={className} />
}

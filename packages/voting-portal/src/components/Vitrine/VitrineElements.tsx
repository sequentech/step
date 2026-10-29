// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// React wrappers for the Vitrine UI kit's elements. The kit takes objects
// (chains, policies, expectations, results) as properties, so the wrappers set
// them through refs, and listen to the kit's events with addEventListener.
// They render the elements only once the kit has defined them: a property set
// on an element before it is upgraded would hide the element's own setter.

import React, {useEffect, useRef, useState} from "react"
import type {
    CheckIndependentlyDetail,
    ChainResult,
    Expect,
    OverallStatus,
    ProfileWords,
    ResultDetail,
    VitrineProofSteps,
    VitrineReceipt,
    VitrineStatus,
    Where,
} from "@vitrine/ui"
import {loadVitrineElements} from "../../services/vitrine/sdk"

type TElementProps<T extends HTMLElement> = React.DetailedHTMLProps<React.HTMLAttributes<T>, T>

declare module "react" {
    namespace JSX {
        interface IntrinsicElements {
            "vitrine-receipt": TElementProps<VitrineReceipt> & {where?: Where; bare?: boolean}
            "vitrine-status": TElementProps<VitrineStatus> & {state?: OverallStatus}
            "vitrine-proof-steps": TElementProps<VitrineProofSteps>
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

export interface IVitrineReceiptViewProps {
    /** The receipt chain document. */
    chain?: string
    /** The pinned policy: its text, or the verifier's parsed policy. */
    policy?: unknown
    expect?: Expect
    /** The page's own result for the chain; without one the element checks
     * the chain itself. */
    result?: ChainResult
    where: Where
    compact?: boolean
    /** The card alone: the page lays out its own actions. */
    bare?: boolean
    /** The profile's words, which name an entry ("ballot"). */
    profileWords?: ProfileWords
    lang: string
    className?: string
    onCheckIndependently?: (detail: CheckIndependentlyDetail) => void
    onResult?: (result: ChainResult) => void
}

export const VitrineReceiptView: React.FC<IVitrineReceiptViewProps> = ({
    chain,
    policy,
    expect,
    result,
    where,
    compact,
    bare,
    profileWords,
    lang,
    className,
    onCheckIndependently,
    onResult,
}) => {
    const ready = useVitrineElements()
    const ref = useRef<VitrineReceipt | null>(null)
    const handlers = useRef({onCheckIndependently, onResult})
    handlers.current = {onCheckIndependently, onResult}

    useEffect(() => {
        const element = ref.current
        if (!element) return
        element.policy = policy ?? null
        element.expect = expect ?? null
        element.chain = chain ?? null
        if (chain && result) element.result = result
    }, [ready, chain, policy, expect, result])

    useEffect(() => {
        const element = ref.current
        if (!element) return
        element.compact = Boolean(compact)
        element.bare = Boolean(bare)
        element.profileWords = profileWords ?? null
    }, [ready, compact, bare, profileWords])

    useEffect(() => {
        const element = ref.current
        if (!element) return
        const onCheck = (event: Event) => {
            handlers.current.onCheckIndependently?.(
                (event as CustomEvent<CheckIndependentlyDetail>).detail
            )
        }
        const onComputed = (event: Event) => {
            handlers.current.onResult?.((event as CustomEvent<ResultDetail>).detail.result)
        }
        element.addEventListener("vitrine-check-independently", onCheck)
        element.addEventListener("vitrine-result", onComputed)
        return () => {
            element.removeEventListener("vitrine-check-independently", onCheck)
            element.removeEventListener("vitrine-result", onComputed)
        }
    }, [ready])

    if (!ready) return null
    return <vitrine-receipt ref={ref} where={where} lang={lang} className={className} />
}

export interface IVitrineProofStepsViewProps {
    result?: ChainResult | null
    lang: string
    className?: string
}

export const VitrineProofStepsView: React.FC<IVitrineProofStepsViewProps> = ({
    result,
    lang,
    className,
}) => {
    const ready = useVitrineElements()
    const ref = useRef<VitrineProofSteps | null>(null)
    useEffect(() => {
        if (ref.current) ref.current.result = result ?? null
    }, [ready, result])
    if (!ready) return null
    return <vitrine-proof-steps ref={ref} lang={lang} className={className} />
}

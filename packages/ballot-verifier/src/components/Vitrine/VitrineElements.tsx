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
    ChainResult,
    EvidenceDetail,
    Expect,
    VitrineProofSteps,
    VitrineReceipt,
    Where,
} from "@vitrine/ui"
import {loadVitrineUi, TVitrineUi} from "../../services/vitrine/sdk"

type TElementProps<T extends HTMLElement> = React.DetailedHTMLProps<React.HTMLAttributes<T>, T>

declare module "react" {
    namespace JSX {
        interface IntrinsicElements {
            "vitrine-receipt": TElementProps<VitrineReceipt> & {where?: Where}
            "vitrine-proof-steps": TElementProps<VitrineProofSteps>
        }
    }
}

export interface IVitrineUiState {
    /** The kit, once its elements are defined. */
    ui?: TVitrineUi
    /** True when the kit couldn't be loaded. */
    failed: boolean
}

/** Loads the kit and defines its elements. */
export const useVitrineUi = (): IVitrineUiState => {
    const [state, setState] = useState<IVitrineUiState>({failed: false})
    useEffect(() => {
        let active = true
        loadVitrineUi().then(
            (ui) => {
                if (active) setState({ui, failed: false})
            },
            () => {
                if (active) setState({failed: true})
            }
        )
        return () => {
            active = false
        }
    }, [])
    return state
}

export interface IVitrineReceiptViewProps {
    /** The receipt chain document. */
    chain: string
    /** The pinned policy, as the verifier parsed it. */
    policy: unknown
    expect: Expect
    /** The page's own result for the chain. */
    result: ChainResult
    where: Where
    lang: string
    className?: string
    /** The data looked damaged and the user asked to check again. */
    onRetry?: () => void
}

export const VitrineReceiptView: React.FC<IVitrineReceiptViewProps> = ({
    chain,
    policy,
    expect,
    result,
    where,
    lang,
    className,
    onRetry,
}) => {
    const {ui} = useVitrineUi()
    const ref = useRef<VitrineReceipt | null>(null)
    const handlers = useRef({onRetry})
    handlers.current = {onRetry}

    useEffect(() => {
        const element = ref.current
        if (!element) return
        element.policy = policy
        element.expect = expect
        element.chain = chain
        element.result = result
    }, [ui, chain, policy, expect, result])

    useEffect(() => {
        const element = ref.current
        if (!element) return
        const retry = (event: Event) => {
            if (!handlers.current.onRetry) return
            event.preventDefault()
            handlers.current.onRetry()
        }
        element.addEventListener("vitrine-retry", retry)
        return () => element.removeEventListener("vitrine-retry", retry)
    }, [ui])

    if (!ui) return null
    return <vitrine-receipt ref={ref} where={where} lang={lang} className={className} />
}

export interface IVitrineProofStepsViewProps {
    result: ChainResult
    lang: string
    className?: string
    /** The user asked to save the evidence for a failed check. */
    onEvidence?: (detail: EvidenceDetail) => void
}

export const VitrineProofStepsView: React.FC<IVitrineProofStepsViewProps> = ({
    result,
    lang,
    className,
    onEvidence,
}) => {
    const {ui} = useVitrineUi()
    const ref = useRef<VitrineProofSteps | null>(null)
    const handlers = useRef({onEvidence})
    handlers.current = {onEvidence}

    useEffect(() => {
        if (ref.current) ref.current.result = result
    }, [ui, result])

    useEffect(() => {
        const element = ref.current
        if (!element) return
        const evidence = (event: Event) =>
            handlers.current.onEvidence?.((event as CustomEvent<EvidenceDetail>).detail)
        element.addEventListener("vitrine-evidence", evidence)
        return () => element.removeEventListener("vitrine-evidence", evidence)
    }, [ui])

    if (!ui) return null
    return <vitrine-proof-steps ref={ref} lang={lang} className={className} />
}

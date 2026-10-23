// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {createContext, useContext, useMemo} from "react"
import {ENumberFormatPolicy} from "../types/ElectionEventPresentation"
import {
    DEFAULT_NUMBER_FORMAT_POLICY,
    formatNumber,
    formatPercentage,
    NumberInput,
    resolveNumberFormatPolicy,
} from "./numberFormat"

export interface INumberFormat {
    policy: ENumberFormatPolicy
    formatNumber: (value: NumberInput, decimals?: number) => string
    formatPercentage: (percentage: NumberInput, decimals?: number) => string
}

const NumberFormatContext = createContext<ENumberFormatPolicy>(DEFAULT_NUMBER_FORMAT_POLICY)

/** Makes `policy`, usually an election event's, the number format below it. */
export const NumberFormatProvider: React.FC<{
    policy?: string | null
    children: React.ReactNode
}> = ({policy, children}) => (
    <NumberFormatContext.Provider value={resolveNumberFormatPolicy(policy)}>
        {children}
    </NumberFormatContext.Provider>
)

/** Formatters for the nearest provider's policy, or for the default without one. */
export const useNumberFormat = (): INumberFormat => {
    const policy = useContext(NumberFormatContext)
    return useMemo(
        () => ({
            policy,
            formatNumber: (value, decimals) => formatNumber(value, policy, decimals),
            formatPercentage: (percentage, decimals) =>
                formatPercentage(percentage, policy, decimals),
        }),
        [policy]
    )
}

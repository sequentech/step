// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {MonitoringCondition} from "../types"
import {ESelectorState, type SelectorState} from "./selectors"

/**
 * Whether a selector with this `when` condition is shown: the selector it names
 * is declared earlier and holds one of the listed values. A hidden selector
 * feeds nothing, as on the server.
 */
export function conditionHolds(
    condition: MonitoringCondition | undefined,
    earlier: Record<string, SelectorState>
): boolean {
    if (!condition) return true
    const state = earlier[condition.selector]
    return state?.kind === ESelectorState.VALUE && condition.in.includes(state.value)
}

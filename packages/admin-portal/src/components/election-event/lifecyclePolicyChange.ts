// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// How two lifecycle configurations compare (design §5b), for the
// configuration approval's diff: each policy or rule change is a tightening,
// a loosening, or both. How a save applies comes from the server.
import {
    EInitializationScope,
    EUnsignedScheduledClosePolicy,
    type ILifecyclePolicies,
} from "@sequentech/ui-core"

export enum EPolicyChange {
    NONE = "none",
    /** Requires more: applies now. */
    TIGHTENS = "tightens",
    /** Requires less: applies after the next approved publication. */
    LOOSENS = "loosens",
    /** Requires something new and drops something else: both rules apply until then. */
    MIXED = "mixed",
}

/** What each initialization scope requires before a Post opens. */
const SCOPE_REQUIRES: Record<EInitializationScope, ReadonlyArray<string>> = {
    [EInitializationScope.POST]: ["post"],
    [EInitializationScope.EVENT]: ["post", "every-post"],
    [EInitializationScope.POST_AND_COUNTRY]: ["post", "every-country"],
}

/** What each close policy requires of a scheduled close outside the signed configuration. */
const CLOSE_REQUIRES: Record<EUnsignedScheduledClosePolicy, ReadonlyArray<string>> = {
    [EUnsignedScheduledClosePolicy.REFUSE]: ["signatures"],
    [EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM]: [],
}

/** Compares two sets of requirements. */
export const requirementChange = (
    before: ReadonlyArray<string>,
    after: ReadonlyArray<string>
): EPolicyChange => {
    const added = after.some((requirement) => !before.includes(requirement))
    const dropped = before.some((requirement) => !after.includes(requirement))
    if (added && dropped) return EPolicyChange.MIXED
    if (added) return EPolicyChange.TIGHTENS
    if (dropped) return EPolicyChange.LOOSENS
    return EPolicyChange.NONE
}

/** The policies with their defaults (design §2: `None` = defaults). */
export const policiesOf = (
    policies: ILifecyclePolicies | null | undefined
): Required<ILifecyclePolicies> => ({
    initialization_scope: policies?.initialization_scope ?? EInitializationScope.POST,
    unsigned_scheduled_close:
        policies?.unsigned_scheduled_close ?? EUnsignedScheduledClosePolicy.REFUSE,
})

export const scopeChange = (before: EInitializationScope, after: EInitializationScope) =>
    requirementChange(SCOPE_REQUIRES[before], SCOPE_REQUIRES[after])

export const closePolicyChange = (
    before: EUnsignedScheduledClosePolicy,
    after: EUnsignedScheduledClosePolicy
) => requirementChange(CLOSE_REQUIRES[before], CLOSE_REQUIRES[after])

/** A signing rule as it decides whether a scheduled opening or closing needs signatures. */
export interface IRuleStrictness {
    required: boolean
    signatures?: number | null
}

/** How a signing rule edit applies: requiring signatures, or more of them, tightens. */
export const ruleChange = (before: IRuleStrictness, after: IRuleStrictness): EPolicyChange => {
    if (before.required !== after.required) {
        return after.required ? EPolicyChange.TIGHTENS : EPolicyChange.LOOSENS
    }
    if (!after.required) return EPolicyChange.NONE
    const from = before.signatures ?? 0
    const to = after.signatures ?? 0
    if (to > from) return EPolicyChange.TIGHTENS
    if (to < from) return EPolicyChange.LOOSENS
    return EPolicyChange.NONE
}

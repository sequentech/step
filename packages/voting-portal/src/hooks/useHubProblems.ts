// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useState} from "react"
import type {Policy} from "@vitrine/verify"
import {hubProblems, IHubProblems} from "../services/vitrine/hubProblems"

/** The problems the hubs of the policy list, asked once while `enabled`.
 * Undefined until they answer, and when there are no hubs to ask. */
export const useHubProblems = (
    policy: Policy | undefined,
    enabled: boolean
): IHubProblems | undefined => {
    const [found, setFound] = useState<{policyId: string; problems: IHubProblems}>()
    const policyId = policy?.id

    useEffect(() => {
        if (!enabled || !policy || policy.hubs.size === 0) return
        const controller = new AbortController()
        void hubProblems(policy, {signal: controller.signal}).then((problems) => {
            if (!controller.signal.aborted) setFound({policyId: policy.id, problems})
        })
        return () => controller.abort()
        // The policy object is rebuilt with each receipt view; its ID is what matters.
    }, [enabled, policyId])

    return found?.policyId === policyId ? found?.problems : undefined
}

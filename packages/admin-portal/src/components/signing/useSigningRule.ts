// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useContext} from "react"
import {useQuery} from "@apollo/client"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import {GET_SIGNING_RULES} from "@/queries/SigningSettings"
import {SigningRequirement, type ISigningRuleRow, type SigningAction} from "@/lib/signing/types"

/**
 * Whether an action of the event needs signatures: `null` while unknown,
 * when the viewer can't read the signing rules, or while not `enabled`.
 */
export function useActionNeedsSignatures(
    electionEventId: string | null | undefined,
    action: SigningAction,
    enabled = true
): boolean | null {
    const auth = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    const canRead = auth.isAuthorized(true, tenantId, IPermissions.SIGNING_RULES_READ)
    const {data} = useQuery<{sequent_backend_signing_rule: ISigningRuleRow[]}>(GET_SIGNING_RULES, {
        variables: {electionEventId},
        context: {headers: {"x-hasura-role": IPermissions.SIGNING_RULES_READ}},
        skip: !enabled || !canRead || !electionEventId,
    })
    const rules = data?.sequent_backend_signing_rule
    if (!rules) return null
    return rules.some(
        (rule) => rule.action === action && rule.requirement === SigningRequirement.Required
    )
}

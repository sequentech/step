// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {AuthContextValues} from "@/providers/AuthContextProvider"
import {EStoryPermissions} from "../../../ui-essentials/.storybook/globals"
// Every new tenant realm is created from this template, which defines the groups.
import {groups} from "../../../../.devcontainer/keycloak/import/tenant-90505c8a-23a9-4cdf-a26b-4e19f6a097d5.json"

/** Realm user of the trustee group whose `trustee` attribute names it in ceremonies. */
export const STORY_TRUSTEE = "trustee1"

/** Realm roles a member of the group holds. */
export function groupRoles(group: EStoryPermissions): string[] {
    if (group === EStoryPermissions.NONE) return []
    const template = groups.find(({name}) => name === group)
    if (!template) throw new Error(`The tenant realm template has no ${group} group`)
    return template.realmRoles
}

/** A signed-in member of the group, checking permissions as AuthContextProvider does. */
export function storyAuth(
    group: EStoryPermissions,
    tenantId: string,
    base: AuthContextValues
): AuthContextValues {
    const roles = new Set(groupRoles(group))
    const trustee = group === EStoryPermissions.TRUSTEE ? STORY_TRUSTEE : ""
    const username = trustee || "admin"
    return {
        ...base,
        isAuthenticated: true,
        username,
        firstName: username,
        tenantId,
        trustee,
        hasRole: (role) => roles.has(role),
        // A list of permissions requires any one of them.
        isAuthorized: (_checkSuperAdmin, someTenantId, permission) =>
            someTenantId === tenantId && [permission].flat().some((role) => roles.has(role)),
    }
}

// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import isEqual from "lodash/isEqual"
import {IUser} from "@sequentech/ui-core"
import {ReviewChangesRow} from "@sequentech/ui-essentials"
import {UserProfileAttribute} from "@/gql/graphql"
import {getTranslationLabel, userBasicInfo} from "@/services/UserService"

export interface UserBaseline {
    user: IUser
    phoneInputs: {[key: string]: string[]}
}

export interface UserDraft {
    user: IUser | undefined
    phoneInputs: {[key: string]: string[]}
    selectedActedTrustee: string
}

export interface RoleDraft {
    activeRoleIds: string[]
}

export interface RoleDefinition {
    id?: string
    name?: string
}

type UserAreaValue = {
    id?: string
    name?: string
} | null | undefined

export const formatFieldValue = (value: unknown, t: (key: string) => string): string => {
    if (value === null || value === undefined || value === "") {
        return "-"
    }
    if (typeof value === "boolean") {
        return value ? t("common.label.yes") : t("common.label.no")
    }
    if (Array.isArray(value)) {
        return value.length > 0 ? value.join(", ") : "-"
    }
    return String(value)
}

const formatAreaValue = (area: UserAreaValue, t: (key: string) => string): string => {
    return formatFieldValue(area?.name ?? area?.id, t)
}

const valuesEqual = (a: unknown, b: unknown): boolean => {
    if (Array.isArray(a) || Array.isArray(b)) {
        const arrayA = Array.isArray(a) ? [...a].sort() : []
        const arrayB = Array.isArray(b) ? [...b].sort() : []
        return isEqual(arrayA, arrayB)
    }
    return (a ?? "") === (b ?? "")
}

/**
 * Diffs the in-progress edit-voter draft against the baseline captured when
 * the edit drawer loaded. Mirrors the field set/sourcing that
 * EditUserForm's renderFormField actually displays (userBasicInfo fields
 * live on `user` directly, everything else lives in `user.attributes`),
 * so the review table never reports a field that isn't actually editable.
 */
export const computeUserDiff = (
    baseline: UserBaseline,
    current: UserDraft,
    userAttributes: UserProfileAttribute[],
    t: (key: string) => string
): ReviewChangesRow[] => {
    const rows: ReviewChangesRow[] = []
    const areaAttribute = userAttributes.find((attr) => attr.name?.toLowerCase().includes("area"))

    const pushIfChanged = (field: string, label: string, oldValue: unknown, newValue: unknown) => {
        if (!valuesEqual(oldValue, newValue)) {
            rows.push({
                field,
                label,
                currentValue: formatFieldValue(oldValue, t),
                newValue: formatFieldValue(newValue, t),
            })
        }
    }

    // "enabled" is rendered as a standalone checkbox outside the userAttributes loop.
    pushIfChanged(
        "enabled",
        t("usersAndRolesScreen.users.fields.enabled"),
        baseline.user.enabled,
        current.user?.enabled
    )

    if (!valuesEqual(baseline.user.area?.id, current.user?.area?.id)) {
        rows.push({
            field: areaAttribute?.name ?? "area",
            label: areaAttribute
                ? getTranslationLabel(areaAttribute.name, areaAttribute.display_name, t)
                : t("usersAndRolesScreen.users.fields.area"),
            currentValue: formatAreaValue(baseline.user.area, t),
            newValue: formatAreaValue(current.user?.area, t),
        })
    }

    userAttributes.forEach((attr) => {
        const name = attr.name
        if (!name) {
            return
        }
        const lowerName = name.toLowerCase()
        const label = getTranslationLabel(name, attr.display_name, t)

        // These substring checks intentionally mirror renderFormField's own attr.name
        // matching (EditUserForm.tsx), so a field is categorized here exactly the way
        // the edit form treats it. Area is rendered via a dedicated selector outside
        // the loop, so it's diffed once above against `user.area.id`.
        if (lowerName.includes("area")) {
            return
        }

        // SelectActedTrustee auto-populates selectedActedTrustee from the baseline trustee
        // on mount; falling back to the baseline value here neutralizes that so it never
        // reports a spurious change until the admin actually picks a different trustee.
        if (lowerName.includes("trustee")) {
            const baselineTrustee = baseline.user.attributes?.[name]?.[0]
            const currentTrustee = current.selectedActedTrustee || baselineTrustee
            pushIfChanged(name, label, baselineTrustee, currentTrustee)
            return
        }

        if (lowerName.includes("mobile-number")) {
            const baselineValue = baseline.user.attributes?.[name]?.[0]
            const currentValue =
                current.phoneInputs[name]?.[0] ?? current.user?.attributes?.[name]?.[0]
            pushIfChanged(name, label, baselineValue, currentValue)
            return
        }

        const isCustomAttribute = !userBasicInfo.includes(name)
        if (isCustomAttribute) {
            pushIfChanged(
                name,
                label,
                baseline.user.attributes?.[name],
                current.user?.attributes?.[name]
            )
        } else if (name !== "username") {
            // username is always disabled in edit mode, so it's never diffed.
            pushIfChanged(
                name,
                label,
                baseline.user[name as keyof IUser],
                current.user?.[name as keyof IUser]
            )
        }
    })

    return rows
}

export const computeRoleDiff = (
    baseline: RoleDraft,
    current: RoleDraft,
    roles: RoleDefinition[],
    t: (key: string) => string
): ReviewChangesRow[] => {
    const baselineIds = new Set(baseline.activeRoleIds)
    const currentIds = new Set(current.activeRoleIds)

    return roles
        .filter((role) => role.id)
        .filter((role) => baselineIds.has(role.id as string) !== currentIds.has(role.id as string))
        .sort((left, right) => (left.name ?? left.id ?? "").localeCompare(right.name ?? right.id ?? ""))
        .map((role) => {
            const roleId = role.id as string
            return {
                field: `role:${roleId}`,
                label: role.name ?? roleId,
                currentValue: baselineIds.has(roleId)
                    ? t("common.label.yes")
                    : t("common.label.no"),
                newValue: currentIds.has(roleId) ? t("common.label.yes") : t("common.label.no"),
            }
        })
}

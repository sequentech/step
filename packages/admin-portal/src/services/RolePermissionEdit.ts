// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"
import {SIGNING_ACTIONS} from "@/lib/signing/types"

/**
 * Which of a role's permissions the user may turn on or off in Users and Roles. Harvest's
 * `RolePermissionEdit` enforces the same: `role-write` alone changes only who can sign.
 */
export enum RolePermissionEdit {
    AnyPermission = "any-permission",
    SignPermissions = "sign-permissions",
    ReadOnly = "read-only",
}

const SIGN_PERMISSIONS: ReadonlySet<string> = new Set(
    Object.values(SIGNING_ACTIONS).map((action) => action.signPermission)
)

export const rolePermissionEdit = (
    holds: (permission: IPermissions) => boolean
): RolePermissionEdit => {
    if (!holds(IPermissions.ROLE_WRITE)) {
        return RolePermissionEdit.ReadOnly
    }
    return holds(IPermissions.USER_PERMISSION_WRITE)
        ? RolePermissionEdit.AnyPermission
        : RolePermissionEdit.SignPermissions
}

export const canEditRolePermission = (edit: RolePermissionEdit, permissionName: string): boolean =>
    edit === RolePermissionEdit.AnyPermission ||
    (edit === RolePermissionEdit.SignPermissions && SIGN_PERMISSIONS.has(permissionName))

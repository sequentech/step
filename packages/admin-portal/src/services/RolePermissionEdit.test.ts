// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {canEditRolePermission, rolePermissionEdit, RolePermissionEdit} from "./RolePermissionEdit"
import {IPermissions} from "@/types/keycloak"

const holding =
    (...held: IPermissions[]) =>
    (permission: IPermissions) =>
        held.includes(permission)

it.each([
    [
        [IPermissions.ROLE_WRITE, IPermissions.USER_PERMISSION_WRITE],
        RolePermissionEdit.AnyPermission,
    ],
    [[IPermissions.ROLE_WRITE], RolePermissionEdit.SignPermissions],
    [[IPermissions.USER_PERMISSION_WRITE], RolePermissionEdit.ReadOnly],
    [[IPermissions.ROLE_READ, IPermissions.USER_PERMISSION_READ], RolePermissionEdit.ReadOnly],
])("holding %p edits %s", (held, edit) => {
    expect(rolePermissionEdit(holding(...held))).toBe(edit)
})

it.each([
    [IPermissions.SIGN_CLOSE_VOTING, true, true],
    [IPermissions.SIGN_TALLY_KEY, true, true],
    [IPermissions.SIGNING_RULES_WRITE, true, false],
    [IPermissions.ROLE_WRITE, true, false],
    [IPermissions.USER_PERMISSION_WRITE, true, false],
    ["sign-", true, false],
])("%s is editable by anyone with both: %p; by role-write alone: %p", (name, any, sign) => {
    expect(canEditRolePermission(RolePermissionEdit.AnyPermission, name)).toBe(any)
    expect(canEditRolePermission(RolePermissionEdit.SignPermissions, name)).toBe(sign)
    expect(canEditRolePermission(RolePermissionEdit.ReadOnly, name)).toBe(false)
})

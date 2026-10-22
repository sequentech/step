// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Synthetic Keycloak roles and permissions of the tenant realm.
import type {IPermission, IRole} from "@sequentech/ui-core"
import {storyId} from "@/__stories__/fixtures"
import {IPermissions} from "@/types/keycloak"

export type PermissionRecord = IPermission & {id: string}
export type RoleRecord = IRole & {id: string}

export const MANAGER_ROLE_ID = storyId(2, 1)
export const AUDITOR_ROLE_ID = storyId(2, 2)

const permission = (name: string, index: number): PermissionRecord => ({
    id: storyId(1, index),
    name,
    attributes: {},
    container_id: "tenant-realm",
    description: "",
})

/**
 * The realm's permissions; `offline_access` is a Keycloak default role that
 * the role editors leave out because it is not an admin permission.
 */
export const permissionRecords = (): PermissionRecord[] => [
    permission(IPermissions.ROLE_READ, 1),
    permission(IPermissions.ROLE_WRITE, 2),
    permission(IPermissions.USER_READ, 3),
    permission(IPermissions.USER_WRITE, 4),
    permission("offline_access", 5),
]

/** Some signing permissions: the tab's own and the sign permissions of two actions. */
export const signingPermissionRecords = (): PermissionRecord[] => [
    permission(IPermissions.ELECTION_EVENT_SIGNATURES_TAB, 6),
    permission(IPermissions.SIGNING_REQUESTS_READ, 7),
    permission(IPermissions.SIGN_OPEN_VOTING, 8),
    permission(IPermissions.SIGN_CLOSE_VOTING, 9),
]

const access = {manage: true, manageMembers: true, manageMembership: true, view: true}

export const roleRecords = (): RoleRecord[] => [
    {
        id: MANAGER_ROLE_ID,
        name: "voter-manager",
        permissions: [IPermissions.USER_READ, IPermissions.USER_WRITE],
        access,
        attributes: {},
        client_roles: {},
    },
    {
        id: AUDITOR_ROLE_ID,
        name: "auditor",
        permissions: [IPermissions.ROLE_READ],
        access,
        attributes: {},
        client_roles: {},
    },
]

/** A sign permission: role-write alone changes it, as it decides who can sign. */
export const signPermissionRecord = (): PermissionRecord =>
    permission(IPermissions.SIGN_CLOSE_VOTING, 6)

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The election event's permission label as the form submits it. Only an
 * editor of the label sends it back, so saving the event as anyone else
 * leaves it alone, and an empty input is stored as no label.
 */
export const withSubmittedPermissionLabel = <T extends {permission_label?: string | null}>(
    values: T,
    canEditPermissionLabel: boolean
): T => {
    const {permission_label: permissionLabel, ...rest} = values
    if (!canEditPermissionLabel) {
        return rest as T
    }
    return {...rest, permission_label: permissionLabel?.trim() || null} as T
}

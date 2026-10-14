// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"

/** What the Settings screen shows someone. */
export enum ESettingsAccess {
    NONE = "none",
    /** Every tab but Logs. */
    CONFIGURATION = "configuration",
    /** Only the Logs tab. */
    LOGS = "logs",
    CONFIGURATION_AND_LOGS = "configuration-and-logs",
}

/**
 * Settings opens with settings-menu; its configuration tabs need tenant-write
 * and its Logs tab tenant-logs-read. Both are decided before anything shows.
 */
export const settingsAccess = (has: (permission: IPermissions) => boolean): ESettingsAccess => {
    if (!has(IPermissions.SETTINGS_MENU)) return ESettingsAccess.NONE
    const configuration = has(IPermissions.TENANT_WRITE)
    const logs = has(IPermissions.TENANT_LOGS_READ)
    if (configuration && logs) return ESettingsAccess.CONFIGURATION_AND_LOGS
    if (configuration) return ESettingsAccess.CONFIGURATION
    return logs ? ESettingsAccess.LOGS : ESettingsAccess.NONE
}

export const showsConfiguration = (access: ESettingsAccess): boolean =>
    access === ESettingsAccess.CONFIGURATION || access === ESettingsAccess.CONFIGURATION_AND_LOGS

export const showsLogs = (access: ESettingsAccess): boolean =>
    access === ESettingsAccess.LOGS || access === ESettingsAccess.CONFIGURATION_AND_LOGS

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"
import {ESettingsAccess, settingsAccess} from "./settingsAccess"

const access = (...roles: IPermissions[]) => settingsAccess((role) => roles.includes(role))

describe("who sees what in Settings", () => {
    it("shows an auditor only the Logs tab", () => {
        expect(access(IPermissions.SETTINGS_MENU, IPermissions.TENANT_LOGS_READ)).toBe(
            ESettingsAccess.LOGS
        )
    })

    it("shows every tab with both permissions", () => {
        expect(
            access(
                IPermissions.SETTINGS_MENU,
                IPermissions.TENANT_WRITE,
                IPermissions.TENANT_LOGS_READ
            )
        ).toBe(ESettingsAccess.CONFIGURATION_AND_LOGS)
    })

    it("shows every tab but Logs with tenant-write alone", () => {
        expect(access(IPermissions.SETTINGS_MENU, IPermissions.TENANT_WRITE)).toBe(
            ESettingsAccess.CONFIGURATION
        )
    })

    it.each([
        ["no permission", []],
        ["the menu alone", [IPermissions.SETTINGS_MENU]],
        ["tenant-write without the menu", [IPermissions.TENANT_WRITE]],
        ["tenant-logs-read without the menu", [IPermissions.TENANT_LOGS_READ]],
        [
            "both without the menu",
            [IPermissions.TENANT_WRITE, IPermissions.TENANT_LOGS_READ, IPermissions.LOGS_READ],
        ],
        ["an event's logs-read", [IPermissions.SETTINGS_MENU, IPermissions.LOGS_READ]],
    ])("shows nothing with %s", (_, roles) => {
        expect(access(...roles)).toBe(ESettingsAccess.NONE)
    })
})

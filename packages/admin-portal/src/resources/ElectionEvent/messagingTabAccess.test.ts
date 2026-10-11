// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IPermissions} from "@/types/keycloak"
import {showMessagingTab} from "./messagingTabAccess"

const holding =
    (...held: IPermissions[]) =>
    (wanted: IPermissions | IPermissions[]) =>
        (Array.isArray(wanted) ? wanted : [wanted]).some((permission) => held.includes(permission))

describe("the event's Messaging tab (R4-05)", () => {
    it("shows for messaging-config-read", () => {
        expect(showMessagingTab(holding(IPermissions.MESSAGING_CONFIG_READ), false)).toBe(true)
    })

    it("shows for messaging-config-write alone, where the tab says what is missing", () => {
        expect(showMessagingTab(holding(IPermissions.MESSAGING_CONFIG_WRITE), false)).toBe(true)
    })

    it("is hidden without either, and on a locked event", () => {
        expect(showMessagingTab(holding(IPermissions.MESSAGING_ACCOUNT_READ), false)).toBe(false)
        expect(showMessagingTab(holding(IPermissions.MESSAGING_CONFIG_READ), true)).toBe(false)
    })
})

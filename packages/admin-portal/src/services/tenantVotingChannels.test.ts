// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {toTenantVotingChannels} from "./tenantVotingChannels"

describe("toTenantVotingChannels", () => {
    it("keeps a disabled online channel disabled", () => {
        expect(toTenantVotingChannels({online: false, kiosk: true, telephone: false})).toEqual({
            online: false,
            kiosk: true,
            telephone: false,
        })
    })

    it("defaults to online only when the tenant has no channels configured", () => {
        const expected = {online: true, kiosk: false, telephone: false}
        expect(toTenantVotingChannels(undefined)).toEqual(expected)
        expect(toTenantVotingChannels(null)).toEqual(expected)
        expect(toTenantVotingChannels({})).toEqual(expected)
    })
})

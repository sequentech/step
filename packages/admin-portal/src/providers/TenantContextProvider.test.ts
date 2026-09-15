// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {triggerOverrideTranslations} from "@/services/i18n"
import {applyTenantTranslationOverrides} from "./TenantContextProvider"

jest.mock("@/services/i18n", () => ({
    triggerOverrideTranslations: jest.fn(),
}))

const triggerOverrideTranslationsMock = jest.mocked(triggerOverrideTranslations)

describe("TenantContextProvider", () => {
    beforeEach(() => {
        triggerOverrideTranslationsMock.mockClear()
    })

    it("clears the previous tenant translation layer when the next tenant has no overrides", () => {
        const overrides = {en: {"adminPortal:common.label.save": "Store"}}

        applyTenantTranslationOverrides({i18n: overrides})
        expect(triggerOverrideTranslationsMock).toHaveBeenLastCalledWith(overrides)

        applyTenantTranslationOverrides({})
        expect(triggerOverrideTranslationsMock).toHaveBeenLastCalledWith(undefined)
    })
})

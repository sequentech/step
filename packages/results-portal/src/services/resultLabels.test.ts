// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {describe, expect, it, jest} from "@jest/globals"
import type {ResultsRow} from "@/types/results"
import {translatedLabel} from "./resultLabels"

jest.mock("@sequentech/ui-core", () =>
    jest.requireActual("../../../ui-core/src/services/translate")
)

describe("translatedLabel", () => {
    it("uses translations from a serialized presentation before the legacy name", () => {
        const row = {
            name: "Legacy election",
            presentation: JSON.stringify({
                i18n: {
                    en: {name: "Translated election"},
                },
            }),
        } as ResultsRow

        expect(translatedLabel(row, "en-GB")).toBe("Translated election")
    })

    it("falls back to the legacy name when the serialized translation is empty", () => {
        const row = {
            name: "Legacy election",
            presentation: JSON.stringify({
                i18n: {
                    en: {name: ""},
                },
            }),
        } as ResultsRow

        expect(translatedLabel(row, "en")).toBe("Legacy election")
    })
})

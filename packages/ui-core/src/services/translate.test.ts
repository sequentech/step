// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {translateFromPresentation} from "./translate"

const presentation = {
    i18n: {
        en: {name: "Election"},
        fr: {name: "French election"},
    },
    language_conf: {
        default_language_code: "en",
    },
}

describe("translateFromPresentation", () => {
    it("uses the primary language code for regional user locales", () => {
        expect(translateFromPresentation({presentation}, "name", "en-GB")).toBe("Election")
        expect(translateFromPresentation({presentation}, "name", "fr-CA")).toBe("French election")
    })

    it("falls back to the configured default language", () => {
        expect(translateFromPresentation({presentation}, "name", "de-DE")).toBe("Election")
    })

    it("supports receiving the presentation object directly", () => {
        expect(translateFromPresentation(presentation, "name", "de-DE")).toBe("Election")
    })

    it("returns undefined when neither user nor default translation exists", () => {
        expect(
            translateFromPresentation(
                {
                    presentation: {
                        i18n: {fr: {name: "French election"}},
                        language_conf: {default_language_code: "en"},
                    },
                },
                "name",
                "de-DE"
            )
        ).toBeUndefined()
    })

    it("preserves the legacy value when no presentation translations exist", () => {
        expect(translateFromPresentation({name: "Election"}, "name", "en-GB")).toBe("Election")
    })
})

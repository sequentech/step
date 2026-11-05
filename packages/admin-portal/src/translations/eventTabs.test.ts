// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {readFileSync} from "fs"
import {join} from "path"
import english from "./en"
import catalan from "./cat"
import spanish from "./es"
import basque from "./eu"
import french from "./fr"
import galician from "./gl"
import dutch from "./nl"
import tagalog from "./tl"

const LOCALES = {
    en: english,
    cat: catalan,
    es: spanish,
    eu: basque,
    fr: french,
    gl: galician,
    nl: dutch,
    tl: tagalog,
}

// The election event's tab labels the screen asks for, by key.
const source = readFileSync(
    join(__dirname, "../resources/ElectionEvent/ElectionEventTabs.tsx"),
    "utf8"
)
const TAB_KEYS = Array.from(source.matchAll(/t\("electionEventScreen\.tabs\.(\w+)"\)/g)).map(
    (match) => match[1]
)

describe("the election event's tab labels", () => {
    it("include the Transparency tab", () => {
        expect(TAB_KEYS).toContain("transparency")
    })

    it.each(Object.entries(LOCALES))("%s names every tab the screen shows", (_lang, locale) => {
        const tabs = locale.translations.electionEventScreen.tabs as Record<string, string>
        const missing = TAB_KEYS.filter((key) => !tabs[key])
        expect(missing).toEqual([])
    })
})

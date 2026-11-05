// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {readdirSync} from "node:fs"
import {fileURLToPath} from "node:url"
import {expect, test} from "vitest"
import {CATALOGUES, problemTranslations} from "./problemTranslations"

test("every catalogue ui-core ships is listed", () => {
    const shipped = readdirSync(
        fileURLToPath(new URL("../../ui-core/src/translations/", import.meta.url))
    )
        .filter((file) => /^[a-z]+\.ts$/.test(file))
        .map((file) => file.replace(/\.ts$/, ""))
        .sort()
    expect(Object.keys(CATALOGUES).sort()).toEqual(shipped)
})

test("carries each language's problems and nothing else of its catalogue", () => {
    const translations = problemTranslations()
    expect(Object.keys(translations)).toEqual(expect.arrayContaining(["en", "es"]))
    for (const catalogue of Object.values(translations)) {
        expect(Object.keys(catalogue)).toEqual(["problems"])
    }
    expect(Object.keys(translations.en.problems).length).toBeGreaterThan(0)
})

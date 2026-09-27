// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"
import {ivrKeypadHint} from "./ivrKeypadHint"
import en from "../translations/en"

// The real shared wording choice, without loading the whole component library.
jest.mock(
    "@sequentech/ui-essentials",
    () => jest.requireActual("../../../ui-essentials/src/ballot/keypadHint"),
    {virtual: true}
)

describe("ivrKeypadHint", () => {
    const emulator = en.translations.electionEventScreen.ivr.emulator
    // i18next's interpolation, over the English strings.
    const t = ((key: string, values: Record<string, string | number>) => {
        const template = key
            .split(".")
            .reduce<unknown>(
                (node, part) =>
                    typeof node === "object" && node !== null
                        ? (node as Record<string, unknown>)[part]
                        : undefined,
                en.translations
            )
        if (typeof template !== "string") throw new Error(`No English wording for ${key}`)
        return template.replace(/\{\{(\w+)\}\}/g, (_, name: string) => String(values[name]))
    }) as unknown as TFunction

    it("names the keys a prompt accepts", () => {
        expect(ivrKeypadHint(t, {valid_inputs: "0-9", max_digits: 3, timeout: 10})).toBe(
            "Enter your input (max digits=3, valid inputs=0-9, timeout=10s)"
        )
    })

    it("says any digits will do when the IVR lists no valid inputs", () => {
        const hint = ivrKeypadHint(t, {valid_inputs: "", max_digits: 8, timeout: 10})
        expect(hint).toBe("Enter up to 8 digits (any digits, timeout=10s)")
        expect(hint).not.toMatch(/valid inputs=[,)]|\{\{/)
    })

    it("has the any-digits wording in every language", () => {
        for (const lang of ["cat", "en", "es", "eu", "fr", "gl", "nl", "tl"]) {
            // eslint-disable-next-line @typescript-eslint/no-require-imports
            const translations = require(`../translations/${lang}`).default
            const wording: string =
                translations.translations.electionEventScreen.ivr.emulator.inputPlaceholderAnyKeys
            expect(wording).toContain("{{maxDigits}}")
            expect(wording).toContain("{{timeout}}")
            expect(wording).not.toContain("{{validInputs}}")
        }
        expect(emulator.inputPlaceholder).toContain("{{validInputs}}")
    })
})

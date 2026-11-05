// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import i18next from "i18next"
import {EMonitoringNotice} from "../types"
import {humaniseCode, noticeText} from "./notices"
import en from "../../../translations/en"

const LANGUAGES = ["cat", "en", "es", "eu", "fr", "gl", "nl", "tl"]

describe("humaniseCode", () => {
    it("reads a code as words", () => {
        expect(humaniseCode("SOMETHING_NEW_HAPPENED")).toBe("Something new happened")
        expect(humaniseCode("X")).toBe("X")
    })

    it("leaves text that is not a code as it is", () => {
        expect(humaniseCode("Already a sentence.")).toBe("Already a sentence.")
    })
})

describe("noticeText", () => {
    let t: typeof i18next.t
    beforeAll(async () => {
        const instance = i18next.createInstance()
        await instance.init({
            lng: "en",
            resources: {en: {translation: en.translations}},
            interpolation: {escapeValue: false},
        })
        t = instance.t
    })

    it("words every notice the server sends, not its code", () => {
        for (const code of Object.values(EMonitoringNotice)) {
            const text = noticeText(t, code)
            expect(text).not.toBe(code)
            expect(text).toBe(en.translations.monitoring.notices[code])
        }
        expect(noticeText(t, EMonitoringNotice.UNREGISTERED_ATTEMPTS_AT_EVENT_SCOPE_ONLY)).toMatch(
            /whole event/
        )
    })

    it("reads a notice this build does not know as words", () => {
        expect(noticeText(t, "SOMETHING_NEW")).toBe("Something new")
    })

    it.each(LANGUAGES)("has every notice in %s", (lang) => {
        // eslint-disable-next-line @typescript-eslint/no-require-imports
        const translations = require(`../../../translations/${lang}`).default
        for (const code of Object.values(EMonitoringNotice)) {
            expect(translations.translations.monitoring.notices[code]).toEqual(expect.any(String))
        }
    })
})

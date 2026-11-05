// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {parseIvrEntityAnnotations, serializeIvrEntityAnnotations} from "./ivr"

const prompts = {en: {prompt: "Press 1 for Alice"}, es: {prompt: "Pulse 1 para Alice"}}
const wire = '{"en":{"prompt":"Press 1 for Alice"},"es":{"prompt":"Pulse 1 para Alice"}}'

describe("IVR entity annotations", () => {
    beforeEach(() => jest.spyOn(console, "error").mockImplementation(() => undefined))
    afterEach(() => jest.restoreAllMocks())

    it("parses the prompts annotation and keeps the others", () => {
        const annotations = {"ivr:i18n": wire, "color": "blue"}
        expect(parseIvrEntityAnnotations(annotations)).toEqual({
            "ivr:i18n": prompts,
            "color": "blue",
        })
        expect(annotations["ivr:i18n"]).toBe(wire)
    })

    it.each([
        ["missing annotations", null, false],
        ["no prompts annotation", {color: "blue"}, false],
        ["malformed JSON", {"ivr:i18n": "{en:"}, true],
        ["an object instead of a string", {"ivr:i18n": prompts}, true],
    ])("reads %s as no prompts", (_, annotations, reported) => {
        expect(parseIvrEntityAnnotations(annotations)["ivr:i18n"]).toEqual({})
        expect(console.error).toHaveBeenCalledTimes(reported ? 1 : 0)
    })

    it("serializes the prompts object for the string-valued annotations column", () => {
        expect(serializeIvrEntityAnnotations({"ivr:i18n": prompts, "color": "blue"})).toEqual({
            "ivr:i18n": wire,
            "color": "blue",
        })
    })

    it("leaves serialized or missing prompts as they are", () => {
        expect(serializeIvrEntityAnnotations({"ivr:i18n": wire})).toEqual({"ivr:i18n": wire})
        expect(serializeIvrEntityAnnotations(undefined)).toEqual({})
    })
})

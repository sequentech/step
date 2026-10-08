// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {ITemplateMethod} from "@/types/templates"
import {EMessageChannel} from "@/types/messaging"
import {
    methodChannel,
    primaryTemplateMethod,
    templateContentKey,
    templateOffersMethod,
} from "./templateMethods"

describe("primaryTemplateMethod", () => {
    it("is the first selected method in offer order", () => {
        expect(primaryTemplateMethod({SMS: true})).toBe(ITemplateMethod.SMS)
        expect(primaryTemplateMethod({VIBER: true, SMS: true})).toBe(ITemplateMethod.SMS)
        expect(primaryTemplateMethod({DOCUMENT: true, MESSENGER: true})).toBe(
            ITemplateMethod.MESSENGER
        )
        expect(primaryTemplateMethod({DOCUMENT: true})).toBe(ITemplateMethod.DOCUMENT)
    })

    it("is undefined when nothing is selected", () => {
        expect(primaryTemplateMethod(undefined)).toBeUndefined()
        expect(primaryTemplateMethod({EMAIL: false})).toBeUndefined()
    })
})

describe("templateOffersMethod", () => {
    it("uses the selected methods of the template", () => {
        const row = {
            communication_method: ITemplateMethod.EMAIL,
            template: {selected_methods: {EMAIL: true, SMS: true}},
        }
        expect(templateOffersMethod(row, ITemplateMethod.SMS)).toBe(true)
        expect(templateOffersMethod(row, ITemplateMethod.VIBER)).toBe(false)
    })

    it("falls back to the stored method for templates without selected methods", () => {
        const row = {communication_method: ITemplateMethod.SMS, template: {}}
        expect(templateOffersMethod(row, ITemplateMethod.SMS)).toBe(true)
        expect(templateOffersMethod(row, ITemplateMethod.EMAIL)).toBe(false)
    })
})

describe("method channels", () => {
    it("maps messaging methods to channels and content keys", () => {
        expect(methodChannel(ITemplateMethod.WHATSAPP)).toBe(EMessageChannel.WHATSAPP)
        expect(methodChannel(ITemplateMethod.DOCUMENT)).toBeNull()
        expect(templateContentKey(ITemplateMethod.MESSENGER)).toBe("messenger")
    })
})

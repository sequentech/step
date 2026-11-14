// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ITemplateMethod} from "@/types/templates"
import {
    ITemplateRow,
    findTemplateForMethod,
    templateSupportsMethod,
    templatesForMethod,
} from "./sendTemplateSelection"

const row = (alias: string, selected_methods?: {EMAIL: boolean; SMS: boolean}): ITemplateRow => ({
    communication_method: ITemplateMethod.EMAIL,
    template: {
        alias,
        name: alias,
        sms: {message: `${alias} sms`},
        selected_methods: selected_methods && {...selected_methods, DOCUMENT: false},
    },
})

const both = row("initial-contact", {EMAIL: true, SMS: true})
const emailOnly = row("email-only", {EMAIL: true, SMS: false})
const smsOnly = row("sms-only", {EMAIL: false, SMS: true})
const legacy = row("legacy")

describe("templateSupportsMethod", () => {
    it("follows the template's enabled methods, not the row's column", () => {
        expect(templateSupportsMethod(both, ITemplateMethod.SMS)).toBe(true)
        expect(templateSupportsMethod(smsOnly, ITemplateMethod.EMAIL)).toBe(false)
    })

    it("falls back to the row's column when the template has no enabled methods", () => {
        const noneEnabled = row("none", {EMAIL: false, SMS: false})
        expect(templateSupportsMethod(legacy, ITemplateMethod.EMAIL)).toBe(true)
        expect(templateSupportsMethod(legacy, ITemplateMethod.SMS)).toBe(false)
        expect(templateSupportsMethod(noneEnabled, ITemplateMethod.EMAIL)).toBe(true)
    })
})

describe("templatesForMethod", () => {
    const rows = [both, emailOnly, smsOnly, legacy]

    it("lists every template that offers SMS", () => {
        expect(templatesForMethod(rows, ITemplateMethod.SMS).map((t) => t.alias)).toEqual([
            "initial-contact",
            "sms-only",
        ])
    })

    it("lists every template that offers email", () => {
        expect(templatesForMethod(rows, ITemplateMethod.EMAIL).map((t) => t.alias)).toEqual([
            "initial-contact",
            "email-only",
            "legacy",
        ])
    })

    it("handles templates that have not loaded yet", () => {
        expect(templatesForMethod(undefined, ITemplateMethod.SMS)).toEqual([])
    })
})

describe("findTemplateForMethod", () => {
    const rows = [both, emailOnly]

    it("keeps a template that offers the new method", () => {
        expect(findTemplateForMethod(rows, ITemplateMethod.SMS, "initial-contact")?.sms).toEqual({
            message: "initial-contact sms",
        })
    })

    it("drops a template that does not offer the new method", () => {
        expect(findTemplateForMethod(rows, ITemplateMethod.SMS, "email-only")).toBeUndefined()
        expect(findTemplateForMethod(rows, ITemplateMethod.SMS, undefined)).toBeUndefined()
    })
})

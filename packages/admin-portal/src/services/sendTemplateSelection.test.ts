// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETemplateType, ITemplateMethod} from "../types/templates"
import {
    ITemplateRow,
    findTemplateForMethod,
    templateSupportsMethod,
    templatesForMethod,
} from "./sendTemplateSelection"

const row = (
    alias: string,
    selected_methods?: {EMAIL: boolean; SMS: boolean},
    type: ETemplateType = ETemplateType.CREDENTIALS
): ITemplateRow => ({
    communication_method: ITemplateMethod.EMAIL,
    type,
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
const receipt = row("receipt", {EMAIL: true, SMS: true}, ETemplateType.BALLOT_RECEIPT)

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
    const rows = [both, emailOnly, smsOnly, legacy, receipt]

    it("lists every template of the type that offers SMS", () => {
        expect(
            templatesForMethod(rows, ITemplateMethod.SMS, ETemplateType.CREDENTIALS).map(
                (t) => t.alias
            )
        ).toEqual(["initial-contact", "sms-only"])
    })

    it("lists every template of the type that offers email", () => {
        expect(
            templatesForMethod(rows, ITemplateMethod.EMAIL, ETemplateType.CREDENTIALS).map(
                (t) => t.alias
            )
        ).toEqual(["initial-contact", "email-only", "legacy"])
    })

    it("only lists templates of the chosen type", () => {
        expect(
            templatesForMethod(rows, ITemplateMethod.SMS, ETemplateType.BALLOT_RECEIPT).map(
                (t) => t.alias
            )
        ).toEqual(["receipt"])
    })

    it("handles templates that have not loaded yet", () => {
        expect(
            templatesForMethod(undefined, ITemplateMethod.SMS, ETemplateType.CREDENTIALS)
        ).toEqual([])
    })
})

describe("findTemplateForMethod", () => {
    const rows = [both, emailOnly]
    const type = ETemplateType.CREDENTIALS

    it("keeps a template that offers the new method", () => {
        expect(
            findTemplateForMethod(rows, ITemplateMethod.SMS, type, "initial-contact")?.sms
        ).toEqual({message: "initial-contact sms"})
    })

    it("drops a template that does not offer the new method or type", () => {
        expect(findTemplateForMethod(rows, ITemplateMethod.SMS, type, "email-only")).toBeUndefined()
        expect(
            findTemplateForMethod(
                rows,
                ITemplateMethod.EMAIL,
                ETemplateType.BALLOT_RECEIPT,
                "initial-contact"
            )
        ).toBeUndefined()
        expect(findTemplateForMethod(rows, ITemplateMethod.SMS, type, undefined)).toBeUndefined()
    })
})

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessagePurpose} from "@/types/messaging"
import {ETemplateType} from "@/types/templates"
import {templateApprovalRows, templatePurpose} from "./templateApproval"

describe("templateApprovalRows", () => {
    const check = {
        connected: true,
        production_access: true,
        approved_templates: {[EMessagePurpose.NOTICE]: ["en", "tl"]},
    }

    it("lists every language with its approval for the purpose", () => {
        expect(templateApprovalRows(check, EMessagePurpose.NOTICE, ["en", "es"])).toEqual([
            {language: "en", approved: true},
            {language: "es", approved: false},
            {language: "tl", approved: true},
        ])
    })

    it("shows nothing approved for an account never checked", () => {
        expect(templateApprovalRows(null, EMessagePurpose.OTP, ["en"])).toEqual([
            {language: "en", approved: false},
        ])
    })
})

describe("templatePurpose", () => {
    it("treats code templates as OTPs and everything else as notices", () => {
        expect(templatePurpose("OTP")).toBe(EMessagePurpose.OTP)
        expect(templatePurpose(ETemplateType.CREDENTIALS)).toBe(EMessagePurpose.NOTICE)
        expect(templatePurpose(undefined)).toBe(EMessagePurpose.NOTICE)
    })
})

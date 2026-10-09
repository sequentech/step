// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {EMessagePurpose, IAccountCheck} from "@/types/messaging"

const OTP_TEMPLATE_TYPE = "OTP"

export interface ITemplateApprovalRow {
    language: string
    approved: boolean
}

/** Per language, whether the account's provider approved a template for `purpose`. */
export const templateApprovalRows = (
    check: IAccountCheck | null | undefined,
    purpose: EMessagePurpose,
    languages: string[]
): ITemplateApprovalRow[] => {
    const approved = check?.approved_templates?.[purpose] ?? []
    return Array.from(new Set([...languages, ...approved]))
        .sort()
        .map((language) => ({language, approved: approved.includes(language)}))
}

/** The purpose a template of `type` is sent for. */
export const templatePurpose = (type: string | null | undefined): EMessagePurpose =>
    type === OTP_TEMPLATE_TYPE ? EMessagePurpose.OTP : EMessagePurpose.NOTICE

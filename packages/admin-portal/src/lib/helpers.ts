// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ETemplateType} from "@/types/templates"

export const convertToNumber = <T>(val: T) => {
    if (val === null || val === undefined) {
        return null
    }

    if (typeof val === "number") {
        return val
    }

    if (typeof val === "boolean") {
        return val ? 1 : 0
    }

    if (typeof val === "string") {
        const num = parseFloat(val)
        return isNaN(num) ? null : num
    }

    // For any other types return null
    return null
}

export const getPreferenceKey = (key: string, subkey: string) => {
    return `${key.replaceAll("/", "_").replaceAll("-", "_")}_${subkey}`
}

const is_communication_template = [
    ETemplateType.CREDENTIALS,
    ETemplateType.BALLOT_RECEIPT,
    ETemplateType.PARTICIPATION_REPORT,
    ETemplateType.ELECTORAL_RESULTS,
    ETemplateType.OTP,
    ETemplateType.TALLY_REPORT,
    ETemplateType.MANUAL_VERIFICATION_VOTER,
    ETemplateType.MANUAL_VERIFICATION_APPROVAL,
]

export const is_communication_template_type = (templateType: ETemplateType) =>
    is_communication_template.includes(templateType)

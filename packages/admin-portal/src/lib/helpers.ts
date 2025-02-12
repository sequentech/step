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

/**
 * This list contains all the template types that are used for sending
 * automatic communication. If a template type is not in this list, it
 * is considered a non-communication template and will not be used
 * for sending automatic communication.
 */
const is_communication_template: ETemplateType[] = [
    ETemplateType.PARTICIPATION_REPORT,
    ETemplateType.TALLY_REPORT,
    ETemplateType.BALLOT_RECEIPT,
    ETemplateType.PARTICIPATION_REPORT,
    ETemplateType.ELECTORAL_RESULTS,
    ETemplateType.OTP,
    ETemplateType.TALLY_REPORT,
    ETemplateType.MANUAL_VERIFICATION_VOTER,
    ETemplateType.MANUAL_VERIFICATION_APPROVAL,
]

/**
 * This function takes a template type and returns a boolean indicating
 * if the template type is a communication template or not.
 * @param {ETemplateType} templateType - The template type to check.
 * @returns {boolean} Whether the template type is a communication template or not.
 */
export const is_communication_template_type = (templateType: ETemplateType) =>
    is_communication_template.includes(templateType)

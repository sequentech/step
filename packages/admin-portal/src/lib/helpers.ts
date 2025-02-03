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
    ETemplateType.VOTE_RECEIPT,
    ETemplateType.INITIALIZATION_REPORT,
    ETemplateType.TRANSMISSION_REPORT,
    ETemplateType.ACTIVITY_LOGS,
    ETemplateType.ELECTORAL_RESULTS,
    ETemplateType.TRANSMISSION_REPORT,
    ETemplateType.STATISTICAL_REPORT,
    ETemplateType.AUDIT_LOGS,
    ETemplateType.OVCS_INFORMATION,
    ETemplateType.OV_TURNOUT_PERCENTAGE,
    ETemplateType.OVCS_EVENTS,
    ETemplateType.OVCS_STATISTICS,
    ETemplateType.OV_TURNOUT_PER_ABOARD_STATUS_SEX,
    ETemplateType.OV_TURNOUT_PER_ABOARD_STATUS_SEX_PERCENTAGE,
    ETemplateType.LIST_OF_OVERSEAS_VOTERS,
    ETemplateType.OV_PRE_ENROLLED_APPROVED,
    ETemplateType.OV_NOT_YET_PRE_ENROLLED_LIST,
    ETemplateType.OV_WITH_VOTING_STATUS,
    ETemplateType.OV_WHO_VOTED,
    ETemplateType.OV_NOT_YET_PRE_ENROLLED_NUMBER,
]

/**
 * This function takes a template type and returns a boolean indicating
 * if the template type is a communication template or not.
 * @param {ETemplateType} templateType - The template type to check.
 * @returns {boolean} Whether the template type is a communication template or not.
 */
export const is_communication_template_type = (templateType: ETemplateType) =>
    is_communication_template.includes(templateType)

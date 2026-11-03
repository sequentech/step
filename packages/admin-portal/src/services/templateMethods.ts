// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {IMethods, ITemplateMethod} from "@/types/templates"
import {EMessageChannel} from "@/types/messaging"

/**
 * The order a template's stored method is chosen in. It is data, not display:
 * it keeps the order templates were saved with before the channel order
 * (`MESSAGE_TEMPLATE_METHODS`) changed, so saving a template stores the same method.
 */
const STORED_METHOD_ORDER: ITemplateMethod[] = [
    ITemplateMethod.EMAIL,
    ITemplateMethod.SMS,
    ITemplateMethod.WHATSAPP,
    ITemplateMethod.VIBER,
    ITemplateMethod.MESSENGER,
    ITemplateMethod.DOCUMENT,
]

/** The method a template is stored under: the first selected one. */
export const primaryTemplateMethod = (
    selected: IMethods | null | undefined
): ITemplateMethod | undefined => STORED_METHOD_ORDER.find((method) => selected?.[method])

interface ITemplateRow {
    communication_method?: string | null
    template?: {selected_methods?: IMethods | null} | null
}

/** Whether a template has content for `method`. */
export const templateOffersMethod = (row: ITemplateRow, method: ITemplateMethod): boolean => {
    const selected = row.template?.selected_methods
    if (selected && Object.values(selected).some(Boolean)) {
        return selected[method] === true
    }
    return row.communication_method === method
}

export const methodChannel = (method: ITemplateMethod): EMessageChannel | null => {
    switch (method) {
        case ITemplateMethod.EMAIL:
            return EMessageChannel.EMAIL
        case ITemplateMethod.SMS:
            return EMessageChannel.SMS
        case ITemplateMethod.WHATSAPP:
            return EMessageChannel.WHATSAPP
        case ITemplateMethod.VIBER:
            return EMessageChannel.VIBER
        case ITemplateMethod.MESSENGER:
            return EMessageChannel.MESSENGER
        case ITemplateMethod.DOCUMENT:
            return null
    }
}

export type TemplateContentKey = "email" | "sms" | "whatsapp" | "viber" | "messenger" | "document"

/** Key of a method's content in the template JSON. */
export const templateContentKey = (method: ITemplateMethod): TemplateContentKey => {
    switch (method) {
        case ITemplateMethod.EMAIL:
            return "email"
        case ITemplateMethod.SMS:
            return "sms"
        case ITemplateMethod.WHATSAPP:
            return "whatsapp"
        case ITemplateMethod.VIBER:
            return "viber"
        case ITemplateMethod.MESSENGER:
            return "messenger"
        case ITemplateMethod.DOCUMENT:
            return "document"
    }
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {ISendTemplateBody, ITemplateMethod} from "@/types/templates"

export interface ITemplateRow {
    communication_method: string
    template: ISendTemplateBody
}

// The row's communication_method is always EMAIL for templates created in the
// admin portal; the methods a template really offers are its selected_methods.
// Older templates without any selected method still rely on the row's column.
export const templateSupportsMethod = (row: ITemplateRow, method: ITemplateMethod): boolean => {
    const selectedMethods = row.template?.selected_methods
    if (selectedMethods && Object.values(selectedMethods).some(Boolean)) {
        return selectedMethods[method] === true
    }
    return row.communication_method === method
}

export const templatesForMethod = <T extends ITemplateRow>(
    rows: Array<T> | undefined,
    method: ITemplateMethod
): Array<ISendTemplateBody> =>
    (rows ?? []).filter((row) => templateSupportsMethod(row, method)).map((row) => row.template)

export const findTemplateForMethod = <T extends ITemplateRow>(
    rows: Array<T> | undefined,
    method: ITemplateMethod,
    alias: string | undefined
): ISendTemplateBody | undefined =>
    alias
        ? templatesForMethod(rows, method).find((template) => template.alias === alias)
        : undefined

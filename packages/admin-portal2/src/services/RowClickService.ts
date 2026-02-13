// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {isUndefined} from "@sequentech/ui-core"
import {Identifier, RaRecord, RowClickFunction} from "react-admin"

export const stringifyFields = (record: RaRecord, filterFields: Array<string>): string => {
    let o: Record<string, any> = {}
    if (filterFields) {
        for (let key of filterFields) {
            if (!isUndefined(record[key])) {
                o[key] = record[key]
            }
        }
    }
    return JSON.stringify(o)
}

export const generateRowClickHandler =
    (fields: Array<string>, show?: boolean): RowClickFunction =>
    (id: Identifier, resource: string, record: RaRecord): string => {
        return `/${resource}/${id}${show ? "/show" : ""}${
            fields ? `?filter=${stringifyFields(record, fields)}` : ""
        }`
    }

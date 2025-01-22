// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useRecordContext} from "react-admin"
import {parseISO, format} from "date-fns"

const CustomDateField = ({
    base,
    source,
    label,
    emptyText,
}: {
    base: string
    source: string
    label: string
    emptyText: string
}) => {
    const record = useRecordContext()

    if (!record) {
        return <span>{emptyText}</span>
    }

    const dateValue = record[base][source]

    if (!dateValue) {
        return <span>{emptyText}</span>
    }

    try {
        // using date-fns
        return (
            <span>
                {format(parseISO(Array.isArray(dateValue) ? dateValue[0] : dateValue), "PP")}
            </span>
        )
    } catch {
        return <span>{emptyText}</span>
    }
}

export default CustomDateField

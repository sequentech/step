// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useRecordContext} from "react-admin"
import {parseISO, format} from "date-fns"

const CustomDateField = ({
    source,
    label,
    emptyText,
}: {
    source: string
    label: string
    emptyText: string
}) => {
    const record = useRecordContext()

    if (!record) {
        return <span>{emptyText}</span>
    }

    const dateValue = record[`attributes`][source]
    if (!dateValue) {
        return <span>{emptyText}</span>
    }

    try {
        // using date-fns
        return <span>{format(parseISO(dateValue[0]), "PP")}</span>
    } catch {
        return <span>{emptyText}</span>
    }
}

export default CustomDateField

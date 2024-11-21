// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useRecordContext} from "react-admin"

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
        const date = new Date(dateValue)
        if (isNaN(date.getTime())) {
            throw new Error("Invalid date")
        }
        return <span>{date.toLocaleDateString()}</span>
    } catch {
        return <span>{emptyText}</span>
    }
}

export default CustomDateField

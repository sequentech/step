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
        // TODO: ask which date format is better
        // native
        // const [year, month, day] = dateValue[0].split("-")
        // const date = new Date(Date.UTC(+year, +month - 1, +day))
        // if (isNaN(date.getTime())) {
        //     throw new Error("Invalid date")
        // }

        // const isoString = date.toISOString().split("T")[0].split("-").reverse().join("/")

        // return <span>{isoString}</span>

        // using date-fns
        return <span>{format(parseISO(dateValue[0]), "PP")}</span>
    } catch {
        return <span>{emptyText}</span>
    }
}

export default CustomDateField

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useRecordContext} from "react-admin"
import {useEventZonedFormat, useZonedFormat, type ZonedEventRef} from "@/hooks/useZonedFormat"
import type {DateValue} from "@/lib/timezones/zonedFormat"

export interface AdminDateTimeProps {
    value: DateValue
    /** The election event the value belongs to (record or id): shown in its primary zone. */
    event?: ZonedEventRef
    /** Show seconds (task and log times). */
    seconds?: boolean
    /** Shown when there is no value. */
    emptyText?: string
}

/**
 * An Admin Portal time as one value with a label ("09 Apr 2028, 03:41:15
 * PhST"): in the election event's primary zone, else the browser's.
 */
export const AdminDateTime: React.FC<AdminDateTimeProps> = ({
    value,
    event,
    seconds,
    emptyText = "",
}) => {
    const format = useEventZonedFormat(event, {seconds})
    return <span className="admin-date-time">{format.format(value) || emptyText}</span>
}

/** The same in a zone the caller already knows (a server-given zone, a row's zone). */
export const ZonedDateTimeText: React.FC<{
    value: DateValue
    zone?: string | null
    seconds?: boolean
}> = ({value, zone, seconds}) => {
    const format = useZonedFormat(zone, {seconds})
    return <span className="admin-date-time">{format.format(value)}</span>
}

export interface AdminDateFieldProps {
    source: string
    label?: string
    sortable?: boolean
    sortBy?: string
    /** The event of the rows (record or id); else each row's `election_event_id`. */
    event?: ZonedEventRef
    seconds?: boolean
    emptyText?: string
}

/** A react-admin field: the record's `source` time with its label. */
export const AdminDateField: React.FC<AdminDateFieldProps> = ({
    source,
    event,
    seconds,
    emptyText,
}) => {
    const record = useRecordContext()
    return (
        <AdminDateTime
            value={record?.[source]}
            event={event ?? record?.election_event_id}
            seconds={seconds}
            emptyText={emptyText}
        />
    )
}

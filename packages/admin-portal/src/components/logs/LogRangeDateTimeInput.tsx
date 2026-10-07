// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {DateTimeInput} from "react-admin"
import {useWatch} from "react-hook-form"
import {browserTimeZone} from "@sequentech/ui-core"
import {
    ELECTORAL_LOG_DEFAULT_ZONE_FILTER,
    ELECTORAL_LOG_ZONE_FILTER,
} from "@/queries/ListElectoralLog"
import {useTimeZoneService} from "@/components/timezones/timeZoneService"

type Props = React.ComponentProps<typeof DateTimeInput> & {defaultZone?: string}

/** Range values stay wall times; expose the same DST resolution the query uses. */
export const LogRangeDateTimeInput = ({source, defaultZone, helperText, ...props}: Props) => {
    const service = useTimeZoneService()
    const [local, chosen, fallback] = useWatch({
        name: [source, ELECTORAL_LOG_ZONE_FILTER, ELECTORAL_LOG_DEFAULT_ZONE_FILTER],
    })
    const zone =
        [chosen, fallback, defaultZone].find(
            (value): value is string => typeof value === "string" && !!value
        ) ?? browserTimeZone()
    let note: React.ReactNode = helperText
    if (typeof local === "string" && local) {
        try {
            const wall = local.slice(0, 16)
            const warning = service.zonedTimeNote(wall, zone, service.text)
            if (warning) {
                const resolved = service.zonedToInstant(wall, zone)
                note = (
                    <>
                        {warning} {service.formatDateTimeZone(resolved.instant, zone, service.text)}
                    </>
                )
            }
        } catch {
            // Keep the existing input validation while a wall time is incomplete.
        }
    }
    return <DateTimeInput {...props} source={source} helperText={note} />
}

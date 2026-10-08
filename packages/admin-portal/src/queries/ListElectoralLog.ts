// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {gql} from "@apollo/client"
import {browserTimeZone, zonedToInstant} from "@sequentech/ui-core"

/** The range filters: wall times ("YYYY-MM-DDTHH:MM") in the `time_zone` filter's zone. */
export const ELECTORAL_LOG_RANGE_FILTERS = [
    "created_from",
    "created_to",
    "statement_timestamp_from",
    "statement_timestamp_to",
] as const

/** The zone the range filters are entered in; it isn't sent. */
export const ELECTORAL_LOG_ZONE_FILTER = "time_zone"

/**
 * The zone of the range when none is chosen: the list's permanent filter
 * (the event's primary), which survives the list's filter resets.
 */
export const ELECTORAL_LOG_DEFAULT_ZONE_FILTER = "default_time_zone"

/**
 * The filter as the log list action takes it: each range bound becomes the
 * RFC 3339 instant of its wall time in the chosen zone (else the default
 * zone, else the browser's). The server includes both ends to the minute.
 */
export const electoralLogInstantFilters = (
    filter: Record<string, unknown>
): Record<string, unknown> => {
    const {
        [ELECTORAL_LOG_ZONE_FILTER]: chosen,
        [ELECTORAL_LOG_DEFAULT_ZONE_FILTER]: fallback,
        ...rest
    } = filter
    const zone = [chosen, fallback].find((value) => typeof value === "string" && value)
    const inZone = () => (zone as string | undefined) ?? browserTimeZone()
    for (const key of ELECTORAL_LOG_RANGE_FILTERS) {
        const local = rest[key]
        if (typeof local === "string" && local) {
            rest[key] = zonedToInstant(local.slice(0, 16), inZone()).instant
        } else {
            delete rest[key]
        }
    }
    return rest
}

const validOrderBy = ["id", "created", "statement_timestamp", "statement_kind", "user_id"]

export const getElectoralLogVariables = (input: any) => {
    return {
        ...input,
        filter: input?.where?._and?.reduce((acc: any, condition: any) => {
            Object.keys(condition).forEach((key) => {
                if (key !== "election_event_id") {
                    acc[key] = condition[key]?._eq
                }
            })
            return acc
        }, {}),
        order_by:
            Object.fromEntries(
                Object.entries(input?.order_by || {}).filter(([key]) => validOrderBy.includes(key))
            ) ?? undefined,
    }
}

export const getElectoralLog = (fields: any) => {
    let election_event_id = fields?.filter?.election_event_id ?? ""
    return gql`
        query listElectoralLog(
            $limit: Int
            $offset: Int
            $filter: ElectoralLogFilter
            $election_event_id: String = "${election_event_id}"
            $order_by: ElectoralLogOrderBy
        ) {
            listElectoralLog(
                limit: $limit
                offset: $offset
                filter: $filter
                election_event_id: $election_event_id
                order_by: $order_by
            ) {
                items {
                    id
                    created
                    statement_timestamp
                    statement_kind
                    message
                    user_id
                }
                total {
                    aggregate {
                        count
                    }
                }
            }
        }
    `
}

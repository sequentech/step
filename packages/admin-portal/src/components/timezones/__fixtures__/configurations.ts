// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// The two configurations every timezone screen is checked under (design §10):
// an overseas election with 80 zones, Manila primary and logs in the primary,
// and an association in Madrid with a Canary Islands office, logs per election.
// Names, ids and times are invented; expectations come from these values.
import {
    ELogTimeZonePolicy,
    EUnsignedScheduledClosePolicy,
    EInitializationScope,
    type IElectionEventPresentation,
} from "@sequentech/ui-core"

/**
 * "My time" in tests and stories: New York, as in the ticket's example; never
 * the machine's zone, so results don't depend on where they run.
 */
export const MY_TIME_ZONE = "America/New_York"

export interface IFixtureElection {
    id: string
    name: string
    /** The election's own zone; absent uses the primary. */
    timezone?: string
}

export interface IFixtureSchedule {
    id: string
    event_processor: string
    election_id: string | null
    local: string
    timezone: string
    stopped_at?: string
}

export interface ITimeZoneConfiguration {
    eventName: string
    presentation: Pick<IElectionEventPresentation, "timezones" | "lifecycle_policies">
    elections: Array<IFixtureElection>
    schedule: Array<IFixtureSchedule>
}

/**
 * 80 zones of overseas Posts; Asia/Manila is the primary. The same zones as the
 * janitor's COMELEC preset (`windmill/external-bin/janitor/templates/COMELEC/lifecycle.json`,
 * checked by `configurations.test.ts`), in the drafts' order.
 */
export const OVERSEAS_ZONES = [
    "Asia/Manila",
    "Asia/Dubai",
    "Asia/Tokyo",
    "Europe/Rome",
    "Africa/Cairo",
    "America/Los_Angeles",
    "Asia/Kolkata",
    "Africa/Nairobi",
    "America/Toronto",
    "Asia/Kathmandu",
    "Asia/Tehran",
    "Asia/Yangon",
    "Asia/Riyadh",
    "Asia/Qatar",
    "Asia/Kuwait",
    "Asia/Bahrain",
    "Asia/Muscat",
    "Asia/Amman",
    "Asia/Beirut",
    "Asia/Jerusalem",
    "Asia/Baghdad",
    "Europe/Istanbul",
    "Asia/Makassar",
    "Asia/Colombo",
    "Asia/Karachi",
    "Asia/Dhaka",
    "Asia/Bangkok",
    "Asia/Ho_Chi_Minh",
    "Asia/Jakarta",
    "Asia/Kuala_Lumpur",
    "Asia/Singapore",
    "Asia/Brunei",
    "Asia/Shanghai",
    "Asia/Hong_Kong",
    "Asia/Taipei",
    "Asia/Seoul",
    "Asia/Phnom_Penh",
    "Asia/Vientiane",
    "Asia/Dili",
    "Australia/Sydney",
    "Pacific/Pago_Pago",
    "Australia/Melbourne",
    "Pacific/Auckland",
    "Pacific/Guam",
    "Pacific/Honolulu",
    "Pacific/Port_Moresby",
    "Pacific/Fiji",
    "Europe/London",
    "Europe/Madrid",
    "Europe/Paris",
    "Europe/Berlin",
    "Europe/Brussels",
    "Europe/Amsterdam",
    "Europe/Vienna",
    "Europe/Zurich",
    "Europe/Prague",
    "Europe/Warsaw",
    "Europe/Budapest",
    "Europe/Bucharest",
    "Europe/Athens",
    "Europe/Stockholm",
    "Europe/Oslo",
    "Europe/Copenhagen",
    "Europe/Lisbon",
    "Europe/Dublin",
    "Europe/Moscow",
    "Africa/Lagos",
    "Asia/Macau",
    "Africa/Johannesburg",
    "Africa/Casablanca",
    "Africa/Tripoli",
    "Africa/Addis_Ababa",
    "America/New_York",
    "America/Chicago",
    "America/Mexico_City",
    "America/Sao_Paulo",
    "America/Argentina/Buenos_Aires",
    "America/Vancouver",
    "America/Santiago",
    "America/Edmonton",
]

const id = (kind: number, n: number) =>
    `${String(kind).repeat(8)}-${String(kind).repeat(4)}-4${String(kind).repeat(3)}-8${String(kind).repeat(3)}-${String(n).padStart(12, "0")}`

const OVERSEAS_POSTS: Array<IFixtureElection> = [
    {id: id(3, 1), name: "Dubai PCG", timezone: "Asia/Dubai"},
    {id: id(3, 2), name: "Tokyo PE", timezone: "Asia/Tokyo"},
    {id: id(3, 3), name: "Rome PE", timezone: "Europe/Rome"},
    {id: id(3, 4), name: "Cairo PE", timezone: "Africa/Cairo"},
    {id: id(3, 5), name: "Los Angeles PCG", timezone: "America/Los_Angeles"},
    {id: id(3, 6), name: "New Delhi PE", timezone: "Asia/Kolkata"},
    {id: id(3, 7), name: "Nairobi PE", timezone: "Africa/Nairobi"},
    {id: id(3, 8), name: "Toronto PCG", timezone: "America/Toronto"},
]

/** Voting opens at 00:00 Post time on 9 April; one close for every Post at 19:00 Manila on 8 May. */
export const overseasConfiguration = (): ITimeZoneConfiguration => ({
    eventName: "Overseas Voting 2028",
    presentation: {
        timezones: {
            configured: OVERSEAS_ZONES,
            primary: "Asia/Manila",
            logs: ELogTimeZonePolicy.PRIMARY,
        },
        lifecycle_policies: {
            initialization_scope: EInitializationScope.POST_AND_COUNTRY,
            unsigned_scheduled_close: EUnsignedScheduledClosePolicy.RUN_AS_SYSTEM,
        },
    },
    elections: OVERSEAS_POSTS,
    schedule: [
        ...OVERSEAS_POSTS.slice(0, 7).map((post, index) => ({
            id: id(5, index + 1),
            event_processor: "START_VOTING_PERIOD",
            election_id: post.id,
            local: "2028-04-09T00:00",
            timezone: post.timezone ?? "Asia/Manila",
        })),
        {
            id: id(5, 20),
            event_processor: "END_ENROLLMENT_PERIOD",
            election_id: null,
            local: "2028-05-08T18:00",
            timezone: "Asia/Manila",
        },
        {
            id: id(5, 21),
            event_processor: "END_VOTING_PERIOD",
            election_id: null,
            local: "2028-05-08T19:00",
            timezone: "Asia/Manila",
        },
        {
            id: id(5, 22),
            event_processor: "START_ENROLLMENT_PERIOD",
            election_id: OVERSEAS_POSTS[0].id,
            local: "2028-02-09T00:00",
            timezone: "Asia/Dubai",
            stopped_at: "2028-02-08T20:00:04Z",
        },
    ],
})

const COUNCIL_CENTRAL = {id: id(3, 11), name: "Council: Central office"}
const COUNCIL_CANARY = {
    id: id(3, 12),
    name: "Council: Canary Islands office",
    timezone: "Atlantic/Canary",
}
const PENSION_BOARD = {id: id(3, 13), name: "Pension board"}

/** An association in Madrid: one office in the Canary Islands, logs per election. */
export const madridConfiguration = (): ITimeZoneConfiguration => ({
    eventName: "Staff Association Council 2028",
    presentation: {
        timezones: {
            configured: ["Europe/Madrid", "Atlantic/Canary"],
            primary: "Europe/Madrid",
            logs: ELogTimeZonePolicy.ELECTION,
        },
    },
    elections: [COUNCIL_CENTRAL, COUNCIL_CANARY, PENSION_BOARD],
    schedule: [
        {
            id: id(5, 31),
            event_processor: "START_VOTING_PERIOD",
            election_id: COUNCIL_CENTRAL.id,
            local: "2028-06-02T09:00",
            timezone: "Europe/Madrid",
        },
        {
            id: id(5, 32),
            event_processor: "START_VOTING_PERIOD",
            election_id: COUNCIL_CANARY.id,
            local: "2028-06-02T09:00",
            timezone: "Atlantic/Canary",
        },
        {
            id: id(5, 33),
            event_processor: "START_VOTING_PERIOD",
            election_id: PENSION_BOARD.id,
            local: "2028-06-02T09:00",
            timezone: "Europe/Madrid",
        },
        {
            id: id(5, 34),
            event_processor: "END_VOTING_PERIOD",
            election_id: null,
            local: "2028-06-09T18:00",
            timezone: "Europe/Madrid",
        },
    ],
})

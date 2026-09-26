// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {VotingStatusChannel} from "@sequentech/ui-core"
import type {CastVotesPerDay, VotersByChannel} from "@/gql/graphql"
import {STORY_IDS, electionPresentation} from "@/__stories__/fixtures"
import {daysBefore, formatDate, getToday} from "../charts/Charts"

/** The date range the dashboards ask for: the last seven days up to today. */
export const statsDateRange = () => {
    const endDate = getToday()
    return {
        startDate: formatDate(daysBefore(endDate, 6)),
        endDate: formatDate(endDate),
        userTimezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    }
}

export const votesPerDay: CastVotesPerDay[] = [
    {day: "2026-01-12", bucket: "2026-01-12", channel: VotingStatusChannel.Online, day_count: 14},
    {day: "2026-01-13", bucket: "2026-01-13", channel: VotingStatusChannel.Online, day_count: 21},
    {day: "2026-01-13", bucket: "2026-01-13", channel: VotingStatusChannel.Kiosk, day_count: 5},
]

export const votersByChannel: VotersByChannel[] = [
    {channel: VotingStatusChannel.Online, count: 33},
    {channel: VotingStatusChannel.Kiosk, count: 5},
]

/** One row of the `ip_address` resource that ListIpAddress lists. */
export interface IpAddressRow {
    id: string
    ip: string
    country: string
    vote_count: number
    election_id: string
    election_presentation: ReturnType<typeof electionPresentation>
    voters_id: string
}

export const ipAddressRows = (): IpAddressRow[] => [
    {
        id: "ip-1",
        ip: "192.0.2.10",
        country: "ES",
        vote_count: 3,
        election_id: STORY_IDS.election,
        election_presentation: electionPresentation("Council election"),
        voters_id: "voter-1, voter-2, voter-3",
    },
]

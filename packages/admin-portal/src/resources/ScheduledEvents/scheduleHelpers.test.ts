// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {newestPerTarget, type ILifecycleSnapshotEntry} from "@/queries/Lifecycle"
import {publicationFor, unpublishedEventIds} from "./unpublishedChanges"
import {outcomeCount, outcomeFilterIds, outcomesByRow} from "./ScheduleBanners"
import {previewOrder} from "./ImportScheduleDrawer"

jest.mock("@sequentech/ui-core", () => ({
    ...jest.requireActual("../../../../ui-core/src/types/ScheduledOutcome"),
}))
jest.mock("react-admin", () => ({}))
jest.mock("@apollo/client", () => ({gql: (s: TemplateStringsArray) => s.join("")}))
jest.mock("react-i18next", () => ({}))
jest.mock("@/components/timezones/timeZoneService", () => ({}))

const PUBLISHED_AT = "2028-03-01T00:00:00Z"
const START = {
    id: "start",
    event_processor: "START_VOTING_PERIOD",
    cron_config: {
        scheduled_date: "2028-04-08T20:00:00Z",
        local: "2028-04-09T00:00",
        timezone: "Asia/Dubai",
    },
    event_payload: {election_id: "dubai"},
    created_at: "2028-02-01T00:00:00Z",
    stopped_at: null,
}
const eventPublication = (schedule = [START]): ILifecycleSnapshotEntry => ({
    election_id: null,
    publication_id: "publication",
    published_at: PUBLISHED_AT,
    approval_request_id: null,
    approval_code: null,
    signed: false,
    snapshot: {
        schedule: schedule.map((row) => ({
            scheduled_event_id: row.id,
            event_processor: row.event_processor,
            election_id: row.event_payload.election_id,
            scheduled_date: row.cron_config.scheduled_date,
            local: row.cron_config.local,
            timezone: row.cron_config.timezone,
            voting_channels: null,
            fingerprint: "f",
        })),
    },
})

describe("unpublishedEventIds", () => {
    it("marks nothing before the first publication", () => {
        expect(unpublishedEventIds([START], [])).toEqual(new Set())
    })

    it("marks nothing when the row is as published", () => {
        expect(unpublishedEventIds([START], [eventPublication()])).toEqual(new Set())
    })

    it("marks an opening whose wall time changed after publication", () => {
        const edited = {...START, cron_config: {...START.cron_config, local: "2028-04-09T08:00"}}
        expect(unpublishedEventIds([edited], [eventPublication()])).toEqual(new Set(["start"]))
    })

    it("marks an opening the publication doesn't list", () => {
        expect(unpublishedEventIds([START], [eventPublication([])])).toEqual(new Set(["start"]))
    })

    it("marks other rows created after the publication, and skips rows that ran", () => {
        const enrollment = {
            ...START,
            id: "enrollment",
            event_processor: "START_ENROLLMENT_PERIOD",
            created_at: "2028-03-02T00:00:00Z",
        }
        const ran = {...enrollment, id: "ran", stopped_at: "2028-03-03T00:00:00Z"}
        expect(unpublishedEventIds([enrollment, ran], [eventPublication()])).toEqual(
            new Set(["enrollment"])
        )
    })

    it("reads a Post's rows against the newest publication covering that Post", () => {
        const other = {...eventPublication([]), publication_id: "other", election_id: "tokyo"}
        expect(publicationFor([other, eventPublication()], "dubai")?.publication_id).toBe(
            "publication"
        )
        expect(publicationFor([other, eventPublication()], "tokyo")?.publication_id).toBe("other")
    })

    it("marks an event-wide row unless every election's publication has it unchanged", () => {
        const close = {
            ...START,
            id: "close",
            event_processor: "END_VOTING_PERIOD",
            event_payload: {election_id: null},
        }
        const both = eventPublication([close as never])
        // Tokyo republished on its own, without the close in its snapshot.
        const tokyo = {
            ...eventPublication([]),
            publication_id: "tokyo",
            election_id: "tokyo",
            published_at: "2028-03-05T00:00:00Z",
        }
        expect(unpublishedEventIds([close], [both], ["dubai", "tokyo"])).toEqual(new Set())
        // Targets come newest first, as get_lifecycle_snapshots orders them.
        expect(unpublishedEventIds([close], [tokyo, both], ["dubai", "tokyo"])).toEqual(
            new Set(["close"])
        )
    })
})

describe("newestPerTarget", () => {
    it("takes each target's first (newest) snapshot, not its signed one behind it", () => {
        const newest = {...eventPublication(), publication_id: "newest", signed: false}
        const signed = {...eventPublication(), publication_id: "signed", signed: true}
        const post = {...eventPublication(), publication_id: "post", election_id: "tokyo"}
        expect(
            newestPerTarget([newest, signed, post]).map((entry) => entry.publication_id)
        ).toEqual(["newest", "post"])
    })
})

describe("outcome totals", () => {
    const explanation = (outcome: string) => ({outcome}) as never
    const outcomes = outcomesByRow([
        {scheduled_event_id: "close", election_id: "a", explanation: explanation("refused")},
        {scheduled_event_id: "close", election_id: "b", explanation: explanation("runs")},
        {scheduled_event_id: "open", election_id: "a", explanation: explanation("refused")},
    ])

    it("counts each row and Post, and filters by row", () => {
        expect(outcomeCount(outcomes, "refused" as never)).toBe(2)
        expect(outcomeFilterIds(outcomes, "refused" as never)).toEqual(["close", "open"])
        expect(outcomeFilterIds(outcomes, "runs" as never)).toEqual(["close"])
    })
})

describe("previewOrder", () => {
    it("puts rows with errors first, then keeps the file's order", () => {
        const row = (n: number, error_code: string | null = null) => ({
            row: n,
            election_alias: "x",
            event_type: "y",
            local: "2028-01-01T00:00",
            time_zone: "UTC",
            error_code,
        })
        expect(
            previewOrder([row(2), row(5, "dst-gap"), row(3), row(4, "duplicate")]).map((r) => r.row)
        ).toEqual([4, 5, 2, 3])
    })
})

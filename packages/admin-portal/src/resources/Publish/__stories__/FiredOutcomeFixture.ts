// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// `scheduled_event.annotations.fired_outcome` as windmill writes it when a
// scheduled opening or closing fires (signing/actions/voting.rs `run_at_post`
// and `refuse_at_post`): one entry per Post, with the channels it changes, the
// outcome's explanation, a close's record (without code or payload hash) and
// the requests it cancelled. Values are invented.
import {EScheduledOutcomeKind} from "@sequentech/ui-core"
import type {IFiredOutcome, IFiredPost} from "@/types/lifecycle"
import {
    refusedEdited,
    runsAuthorized,
    runsUnsigned,
} from "@/components/timezones/__fixtures__/explanations"

export const FIRED_AT = "2028-05-08T11:00:00.123Z"

/** A close authorized by the signed configuration. */
export const closedAuthorized = (electionId: string, scheduledEventId: string): IFiredPost => {
    const explanation = runsAuthorized()
    return {
        action: "close-voting",
        election_id: electionId,
        scheduled_event_id: scheduledEventId,
        outcome: EScheduledOutcomeKind.RUNS,
        authorized_by: explanation.authorized_by,
        unsigned: false,
        fingerprint: "c4f1e0",
        channels: ["ONLINE"],
        explanation,
        record: {
            closed_at: FIRED_AT,
            election_id: electionId,
            channels: ["ONLINE"],
            from: ["ONLINE=OPEN"],
            signatures: [],
            seals: [],
            authorized_by: explanation.authorized_by,
        },
        cancelled: [],
    }
}

/** A close outside the signed configuration, run at its deadline; it cancelled a waiting close. */
export const closedUnsigned = (electionId: string, scheduledEventId: string): IFiredPost => ({
    action: "close-voting",
    election_id: electionId,
    scheduled_event_id: scheduledEventId,
    outcome: EScheduledOutcomeKind.RUNS_UNSIGNED,
    authorized_by: null,
    unsigned: true,
    fingerprint: "c4f1e0",
    channels: ["ONLINE"],
    reason: "unsigned-scheduled-close",
    explanation: runsUnsigned(),
    record: {
        closed_at: FIRED_AT,
        election_id: electionId,
        channels: ["ONLINE"],
        from: ["ONLINE=OPEN"],
        signatures: [],
        seals: [],
        unsigned: true,
    },
    cancelled: [
        {
            action: "close-voting",
            request_id: "77777777-7777-4777-8777-000000000001",
            code: "R4D-9KX",
            signatures: 1,
            required: 2,
        },
    ],
})

/** An opening refused because it was edited after its configuration was signed. */
export const openedRefused = (electionId: string, scheduledEventId: string): IFiredPost => ({
    action: "open-voting",
    election_id: electionId,
    scheduled_event_id: scheduledEventId,
    outcome: EScheduledOutcomeKind.REFUSED,
    authorized_by: null,
    unsigned: false,
    fingerprint: "a91b27",
    channels: ["ONLINE"],
    reason: "not-in-signed-configuration",
    explanation: refusedEdited(),
})

/** An opening authorized by the signed configuration. */
export const openedAuthorized = (electionId: string, scheduledEventId: string): IFiredPost => {
    const explanation = runsAuthorized()
    return {
        action: "open-voting",
        election_id: electionId,
        scheduled_event_id: scheduledEventId,
        outcome: EScheduledOutcomeKind.RUNS,
        authorized_by: explanation.authorized_by,
        unsigned: false,
        fingerprint: "a91b27",
        channels: ["ONLINE"],
        explanation,
    }
}

/** A close at a Post whose channels were already closed: nothing to change. */
export const closedNothingToChange = (
    electionId: string,
    scheduledEventId: string
): IFiredPost => ({
    ...closedUnsigned(electionId, scheduledEventId),
    channels: [],
    nothing_to_change: true,
    cancelled: [],
})

export const firedOutcome = (posts: Array<IFiredPost>, at = FIRED_AT): IFiredOutcome => ({
    at,
    posts,
})

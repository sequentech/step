// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Annotation key marking an election event as configured for the Datafix
 * integration. Mirrors `DATAFIX_ID_KEY` in
 * `windmill/src/services/datafix/utils.rs`.
 */
export const DATAFIX_ID_ANNOTATION = "datafix:id"

interface AnnotatedElectionEvent {
    annotations?: unknown
}

/**
 * True when the election event carries the Datafix marker annotation.
 *
 * This is the same marker check that `datafix_annotations` in
 * `windmill/src/services/datafix/utils.rs` uses to decide whether an event is
 * Datafix-configured at all: the `datafix:id` key is present, whatever value
 * it holds. It is deliberately only that check. The backend's
 * `is_datafix_election_event` additionally requires the rest of the Datafix
 * annotation block to deserialize, which is validation the UI has no business
 * repeating; gating presentation on the marker alone fails open, showing the
 * column on a half-configured event rather than hiding it on a working one.
 *
 * `annotations` is a free-form jsonb column, so it may be absent, null or not
 * an object at all; all of those mean "not configured".
 */
export const isDatafixElectionEvent = (electionEvent?: AnnotatedElectionEvent | null): boolean => {
    const annotations = electionEvent?.annotations
    if (typeof annotations !== "object" || annotations === null || Array.isArray(annotations)) {
        return false
    }
    return DATAFIX_ID_ANNOTATION in (annotations as Record<string, unknown>)
}

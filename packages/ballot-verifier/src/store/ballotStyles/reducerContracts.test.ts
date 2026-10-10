// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import reducer, {
    IBallotStyle,
    setBallotStyle,
    selectBallotStyleByElectionId,
    selectBallotStyleByElectionEventId,
    selectBallotStyleElectionIds,
    selectFirstBallotStyle,
} from "./ballotStylesSlice"
import type {RootState} from "../store"
const style = (election: string, publication: string) =>
    ({
        id: `style-${publication}`,
        election_id: election,
        election_event_id: `event-${election}`,
        ballot_publication_id: publication,
        publication_published_at: "2026-08-18T00:00:00Z",
    }) as IBallotStyle
it("replaces only the selected election while preserving the previous Redux state", () => {
    const first = style("e1", "a")
    const second = style("e2", "b")
    const original = reducer(reducer(undefined, setBallotStyle(first)), setBallotStyle(second))
    const replacement = style("e1", "z")
    const current = reducer(original, setBallotStyle(replacement))
    expect(original).toEqual({e1: first, e2: second})
    expect(current).toEqual({e1: replacement, e2: second})
    const state = {ballotStyles: current} as RootState
    expect(selectBallotStyleByElectionId("e1")(state)).toBe(replacement)
    expect(selectBallotStyleByElectionId("missing")(state)).toBeUndefined()
    expect(selectBallotStyleElectionIds(state)).toEqual(["e1", "e2"])
    expect(selectFirstBallotStyle(state)).toBe(replacement)
    expect(selectBallotStyleByElectionEventId("event-e1")(state)).toBe(replacement)
    expect(selectBallotStyleByElectionEventId(undefined)(state)).toBeUndefined()
})
it("returns empty selectors for an initial store and ignores absent cached entries", () => {
    const state = {ballotStyles: reducer(undefined, {type: "unrelated"})} as RootState
    expect(selectFirstBallotStyle(state)).toBeUndefined()
    expect(selectBallotStyleElectionIds(state)).toEqual([])
    const present = style("e1", "a")
    const partial = {ballotStyles: {missing: undefined, e1: present}} as RootState
    expect(selectBallotStyleByElectionEventId("event-e1")(partial)).toBe(present)
})

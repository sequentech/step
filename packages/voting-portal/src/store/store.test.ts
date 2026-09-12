// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import {store, clearVoterSession} from "./store"
import {addCastVotes, CastVoteStatus} from "./castVotes/castVotesSlice"
import {setElection} from "./elections/electionsSlice"

test("a voter scope change removes prior eligibility and participation", () => {
    store.dispatch(clearVoterSession())
    const initial = store.getState()
    store.dispatch(
        setElection({
            id: "election",
            tenant_id: "tenant",
            election_event_id: "event",
            image_document_id: "",
            contests: [],
        })
    )
    store.dispatch(
        addCastVotes([
            {
                id: "cast",
                tenant_id: "tenant",
                election_id: "election",
                election_event_id: "event",
                status: CastVoteStatus.VALID,
            },
        ])
    )
    expect(store.getState().castVotes.election).toHaveLength(1)
    store.dispatch(clearVoterSession())
    expect(store.getState()).toEqual(initial)
})

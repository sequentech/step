// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {getCompleteElectionIds} from "./resultsCompleteness"

const fixture = () => {
    const tally: any = {id: "tally", election_ids: ["one", "two"], area_ids: ["a", "b"]}
    const execution: any = {
        tally_session_id: "tally",
        results_event_id: "result",
        status: {
            elections_status: [
                {election_id: "one", status: "SUCCESS", progress: 100},
                {election_id: "two", status: "DECRYPTING", progress: 40},
            ],
        },
    }
    const data: any = {
        sequent_backend_results_event: [{id: "result"}],
        sequent_backend_election: [{id: "one"}, {id: "two"}],
        sequent_backend_contest: [
            {id: "c1", election_id: "one"},
            {id: "c2", election_id: "two"},
        ],
        sequent_backend_area_contest: [
            {contest_id: "c1", area_id: "a"},
            {contest_id: "c1", area_id: "b"},
            {contest_id: "c2", area_id: "a"},
        ],
        sequent_backend_results_election: [{election_id: "one", results_event_id: "result"}],
        sequent_backend_results_contest: [
            {election_id: "one", contest_id: "c1", results_event_id: "result"},
        ],
        sequent_backend_results_area_contest: [
            {
                election_id: "one",
                contest_id: "c1",
                area_id: "a",
                results_event_id: "result",
                total_votes: 12,
            },
            {
                election_id: "one",
                contest_id: "c1",
                area_id: "b",
                results_event_id: "result",
                total_votes: 0,
            },
        ],
    }
    return {tally, execution, data}
}

test("a complete election does not wait for an unrelated incomplete election or PDFs", () => {
    const {tally, execution, data} = fixture()
    tally.annotations = {is_post_task_completed: false}
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
    tally.annotations.is_post_task_completed = true
    execution.documents = {sqlite: "post-pdf"}
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
})

test("a missing area contest blocks the whole election, even with SUCCESS progress", () => {
    const {tally, execution, data} = fixture()
    data.sequent_backend_results_area_contest.pop()
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test("all result rows are insufficient while weight batches are still decrypting", () => {
    const {tally, execution, data} = fixture()
    execution.status.elections_status[0] = {
        election_id: "one",
        status: "DECRYPTING",
        progress: 99.9,
    }
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test.each(["ERROR", "WAITING", "MIXING"])("%s progress cannot produce final results", (status) => {
    const {tally, execution, data} = fixture()
    execution.status.elections_status[0].status = status
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test("intentionally excluded areas are outside the expected scope", () => {
    const {tally, execution, data} = fixture()
    tally.area_ids = ["a"]
    data.sequent_backend_results_area_contest.pop()
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
})

test("missing global contest results cannot produce a final election total", () => {
    const {tally, execution, data} = fixture()
    data.sequent_backend_results_contest = []
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test("a stale execution, snapshot, or area result cannot establish completeness", () => {
    const {tally, execution, data} = fixture()
    expect(getCompleteElectionIds(tally, {...execution, tally_session_id: "old"}, data)).toEqual([])
    expect(getCompleteElectionIds(tally, {...execution, results_event_id: "new"}, data)).toEqual([])
    data.sequent_backend_results_area_contest[1].results_event_id = "old"
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test("loading or missing progress cannot be treated as success", () => {
    const {tally, execution, data} = fixture()
    expect(getCompleteElectionIds(tally, execution, null)).toEqual([])
    expect(getCompleteElectionIds(tally, {...execution, status: {}}, data)).toEqual([])
})

test("contests assigned only to excluded areas do not block selected-area results", () => {
    const {tally, execution, data} = fixture()
    tally.area_ids = ["a"]
    data.sequent_backend_contest.push({id: "excluded-contest", election_id: "one"})
    data.sequent_backend_area_contest.push({contest_id: "excluded-contest", area_id: "b"})
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
})

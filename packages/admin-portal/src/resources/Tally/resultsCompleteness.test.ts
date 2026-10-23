// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only
import {getCompleteElectionIds, isNumericallyComplete} from "./resultsCompleteness"

const fixture = () => {
    const tally: any = {
        annotations: {result_readiness_version: 1},
        id: "tally",
        election_ids: ["one", "two"],
        area_ids: ["a", "b"],
    }
    const execution: any = {
        documents: {sqlite: "db"},
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
        sqliteDocumentId: "db",
        sequent_backend_area: [
            {id: "a", parent_id: null},
            {id: "b", parent_id: null},
        ],
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
    tally.annotations = {result_readiness_version: 1, is_post_task_completed: false}
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
    tally.annotations.is_post_task_completed = true
    execution.documents = {sqlite: "post-pdf"}
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
    data.sqliteDocumentId = "post-pdf"
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

test("a missing SQLite artifact or older loaded document cannot satisfy completeness", () => {
    const {tally, execution, data} = fixture()
    expect(getCompleteElectionIds(tally, {...execution, documents: null}, data)).toEqual([])
    expect(getCompleteElectionIds(tally, execution, {...data, sqliteDocumentId: "old"})).toEqual([])
})

function inheritedFixture() {
    const fixtureData = fixture()
    fixtureData.tally.area_ids = ["a", "b"]
    fixtureData.data.sequent_backend_area[1].parent_id = "a"
    fixtureData.data.sequent_backend_area_contest = [{contest_id: "c1", area_id: "a"}]
    fixtureData.data.sequent_backend_results_area_contest.pop()
    return fixtureData
}

test("an inherited child contest must be present before its election is complete", () => {
    const {tally, execution, data} = inheritedFixture()
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
    data.sequent_backend_results_area_contest.push({
        election_id: "one",
        contest_id: "c1",
        area_id: "b",
        results_event_id: "result",
    })
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
})

test("inherited contests propagate through multiple generations", () => {
    const {tally, execution, data} = inheritedFixture()
    data.sequent_backend_area.push({id: "grandchild", parent_id: "b"})
    tally.area_ids.push("grandchild")
    data.sequent_backend_results_area_contest.push({
        election_id: "one",
        contest_id: "c1",
        area_id: "b",
        results_event_id: "result",
    })
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
    data.sequent_backend_results_area_contest.push({
        election_id: "one",
        contest_id: "c1",
        area_id: "grandchild",
        results_event_id: "result",
    })
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
})

test("an excluded child does not block its parent's complete election result", () => {
    const {tally, execution, data} = inheritedFixture()
    tally.area_ids = ["a"]
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
})

test("missing selected area metadata cannot prove the expected contest scope", () => {
    const {tally, execution, data} = inheritedFixture()
    data.sequent_backend_area.pop()
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test.each(["unassigned", "excluded"])(
    "an election with only %s areas needs no result row and does not block the event",
    (scope) => {
        const {tally, execution, data} = fixture()
        execution.status.elections_status[1] = {
            election_id: "two",
            status: "SUCCESS",
            progress: 100,
        }
        data.sequent_backend_area_contest = data.sequent_backend_area_contest.filter(
            (row: any) => row.contest_id !== "c2"
        )
        if (scope === "excluded") {
            data.sequent_backend_area.push({id: "excluded", parent_id: null})
            data.sequent_backend_area_contest.push({contest_id: "c2", area_id: "excluded"})
        }
        expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one", "two"])
    }
)

test("a missing election result still blocks an election with expected area contests", () => {
    const {tally, execution, data} = fixture()
    data.sequent_backend_results_election = []
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test("legacy tallies retain result availability without new completeness restrictions", () => {
    const {tally, execution, data} = fixture()
    delete tally.annotations.result_readiness_version
    data.sequent_backend_results_area_contest = []
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one", "two"])
    expect(getCompleteElectionIds(tally, execution, null)).toEqual(["one", "two"])
})

test("unpublished inherited areas are outside the saved tally scope", () => {
    const {tally, execution, data} = inheritedFixture()
    tally.annotations.election_areas = JSON.stringify({one: ["a"], two: ["a"]})
    expect(getCompleteElectionIds(tally, execution, data)).toEqual(["one"])
    tally.annotations.election_areas = JSON.stringify({one: ["a", "b"]})
    expect(getCompleteElectionIds(tally, execution, data)).toEqual([])
})

test("empty tallies complete without artifacts only after server finalization", () => {
    const {tally, execution} = fixture()
    tally.election_ids = ["one"]
    tally.is_execution_completed = true
    tally.execution_status = "SUCCESS"
    tally.annotations.is_post_task_completed = true
    execution.results_event_id = null
    execution.documents = null
    expect(getCompleteElectionIds(tally, execution, null)).toEqual(["one"])
    tally.annotations.is_post_task_completed = false
    expect(getCompleteElectionIds(tally, execution, null)).toEqual([])
    tally.annotations.is_post_task_completed = true
    execution.tally_session_id = "old"
    expect(getCompleteElectionIds(tally, execution, null)).toEqual([])
})

test("recount accepts finalized numbers during new-policy report finalization only", () => {
    const tally = {
        execution_status: "IN_PROGRESS",
        is_execution_completed: true,
        annotations: {result_readiness_version: 1},
    }
    expect(isNumericallyComplete(tally)).toBe(true)
    expect(isNumericallyComplete({...tally, annotations: {}})).toBe(false)
    expect(isNumericallyComplete({...tally, is_execution_completed: false})).toBe(false)
    expect(isNumericallyComplete({...tally, execution_status: "CANCELLED"})).toBe(false)
})

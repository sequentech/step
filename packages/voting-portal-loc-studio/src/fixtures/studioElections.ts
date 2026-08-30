// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {
    IBallotStyle,
} from "@sequentech/ui-core"
import {ELECTION_WRITEINS_SIMPLE} from "@voting-portal/fixtures/election"

export const STUDIO_WRITEIN_ELECTION_ID = ELECTION_WRITEINS_SIMPLE.election_id

const cloneBallot = (ballot: IBallotStyle): IBallotStyle =>
    JSON.parse(JSON.stringify(ballot)) as IBallotStyle

export const STUDIO_WRITEINS_PAGINATED: IBallotStyle = (() => {
    const base = cloneBallot(ELECTION_WRITEINS_SIMPLE)
    base.id = "a1a1a1a1-a1a1-4a1a-a1a1-a1a1a1a1a1a1"
    base.tenant_id = "a1a1a1a1-a1a1-4a1a-a1a1-a1a1a1a1a1a1"
    base.election_event_id = "a1a1a1a1-a1a1-4a1a-a1a1-a1a1a1a1a1a1"
    base.election_id = "b2b2b2b2-b2b2-4b2b-b2b2-b2b2b2b2b2b2"
    base.area_id = "a1a1a1a1-a1a1-4a1a-a1a1-a1a1a1a1a1a1"
    base.description = "Write-ins on separate ballot page"

    const regularContest = cloneBallot(ELECTION_WRITEINS_SIMPLE).contests[0]
    regularContest.presentation = {
        ...regularContest.presentation,
        pagination_policy: "main",
    }
    regularContest.candidates = regularContest.candidates.filter(
        (candidate) => !candidate.presentation?.is_write_in
    )

    const writeInContest = cloneBallot(ELECTION_WRITEINS_SIMPLE).contests[0]
    writeInContest.id = "c3c3c3c3-c3c3-4c3c-c3c3-c3c3c3c3c3c3"
    writeInContest.name = "Write-in choices"
    writeInContest.presentation = {
        ...writeInContest.presentation,
        pagination_policy: "write-ins",
        allow_writeins: true,
    }
    writeInContest.candidates = writeInContest.candidates.filter(
        (candidate) => candidate.presentation?.is_write_in
    )
    writeInContest.candidates.forEach((candidate) => {
        candidate.contest_id = writeInContest.id
    })

    base.contests = [regularContest, writeInContest]
    return base
})()

export const STUDIO_WRITEINS_ONLY_PAGE: IBallotStyle = (() => {
    const base = cloneBallot(ELECTION_WRITEINS_SIMPLE)
    base.description = "Write-in page"
    const contest = base.contests[0]
    contest.candidates = contest.candidates.filter(
        (candidate) => candidate.presentation?.is_write_in
    )
    contest.name = "Write-in choices"
    return base
})()

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// Ballot boxes of the council election: the north district's paper box has
// three versions, the last one imported and waiting for review, and the south
// district's postal box has one approved version.
import type {DataProvider, GetListParams, RaRecord} from "react-admin"
import type {Sequent_Backend_Election, Sequent_Backend_Tally_Sheet} from "@/gql/graphql"
import {
    FIXED_TIME,
    STORY_IDS,
    areaRecords,
    candidateRecords,
    contestRecord,
    electionRecord,
    storyId,
    type StoryRecord,
} from "@/__stories__/fixtures"
import {EStatus, ETallySheetImportStatus, type IAreaContestResults} from "@/types/TallySheets"

export const SHEET_IDS = {
    firstPaper: storyId(0, 1),
    secondPaper: storyId(0, 2),
    latestPaper: storyId(0, 3),
    postal: storyId(0, 4),
    import: storyId(0, 5),
    sourceDocument: storyId(0, 6),
}

const scope = {
    tenant_id: STORY_IDS.tenant,
    election_event_id: STORY_IDS.event,
    election_id: STORY_IDS.election,
}

export const ELECTION = {
    ...electionRecord(),
    contests: [],
    contests_aggregate: {nodes: []},
} as Sequent_Backend_Election

export const CONTEST = contestRecord()
export const CANDIDATES = candidateRecords()
export const AREAS = areaRecords()

export function sheetContent(
    areaId: string = STORY_IDS.area,
    [alice, bob]: [number, number] = [30, 20]
): IAreaContestResults {
    return {
        area_id: areaId,
        contest_id: STORY_IDS.contest,
        census: 80,
        total_votes: 60,
        total_valid_votes: 55,
        total_blank_votes: 5,
        blank_ballots: 3,
        invalid_votes: {total_invalid: 5, implicit_invalid: 2, explicit_invalid: 3},
        candidate_results: {
            [STORY_IDS.candidate]: {candidate_id: STORY_IDS.candidate, total_votes: alice},
            [STORY_IDS.secondCandidate]: {
                candidate_id: STORY_IDS.secondCandidate,
                total_votes: bob,
            },
        },
    }
}

export function tallySheetRecord(
    overrides: Partial<StoryRecord<Sequent_Backend_Tally_Sheet>> = {}
): Sequent_Backend_Tally_Sheet {
    return {
        ...scope,
        id: SHEET_IDS.firstPaper,
        area_id: STORY_IDS.area,
        contest_id: STORY_IDS.contest,
        channel: "PAPER",
        content: sheetContent(),
        version: 1,
        status: EStatus.APPROVED,
        created_by_user_id: "clerk.north",
        created_at: FIXED_TIME,
        last_updated_at: FIXED_TIME,
        reviewed_at: FIXED_TIME,
        reviewed_by_user_id: "reviewer.north",
        deleted_at: null,
        import_id: null,
        labels: {},
        annotations: {},
        ...overrides,
    } as Sequent_Backend_Tally_Sheet
}

export const TALLY_SHEETS = [
    tallySheetRecord(),
    tallySheetRecord({
        id: SHEET_IDS.secondPaper,
        version: 2,
        status: EStatus.DISAPPROVED,
        content: sheetContent(STORY_IDS.area, [31, 20]),
    }),
    tallySheetRecord({
        id: SHEET_IDS.latestPaper,
        version: 3,
        status: EStatus.PENDING,
        content: sheetContent(STORY_IDS.area, [32, 20]),
        created_by_user_id: "importer",
        reviewed_at: null,
        reviewed_by_user_id: null,
        import_id: SHEET_IDS.import,
        labels: {source: "ess"},
    }),
    tallySheetRecord({
        id: SHEET_IDS.postal,
        area_id: STORY_IDS.secondArea,
        channel: "POSTAL",
        content: sheetContent(STORY_IDS.secondArea, [12, 9]),
        created_by_user_id: "clerk.south",
        reviewed_by_user_id: "reviewer.south",
    }),
]

export const SHEET_IMPORT = {
    id: SHEET_IDS.import,
    tenant_id: STORY_IDS.tenant,
    election_event_id: STORY_IDS.event,
    status: ETallySheetImportStatus.PENDING_REVIEW,
    source_document_id: SHEET_IDS.sourceDocument,
    source_file_name: "north-paper.xml",
    created_at: FIXED_TIME,
    last_updated_at: FIXED_TIME,
}

const ballotBox = ({area_id, contest_id, channel}: RaRecord) =>
    `${area_id}/${contest_id}/${channel}`

/**
 * The Hasura provider lists one row per ballot box, its latest version, when
 * asked for `distinctBallotBoxes`.
 */
export function withDistinctBallotBoxes(provider: DataProvider): DataProvider {
    return {
        ...provider,
        getList: async (resource: string, params: GetListParams) => {
            if (!params.meta?.distinctBallotBoxes) return provider.getList(resource, params)
            const all = await provider.getList(resource, {
                ...params,
                pagination: {page: 1, perPage: 1000},
                sort: {field: "version", order: "DESC"},
            })
            const latest = all.data.filter(
                (sheet, index) =>
                    all.data.findIndex((other) => ballotBox(other) === ballotBox(sheet)) === index
            )
            return {data: latest, total: latest.length}
        },
    } as DataProvider
}

/**
 * ra-data-hasura filters a String column by substring (`_ilike`), so the tally
 * sheet form's empty area name search lists every area.
 */
export function withAreaNameSearch(provider: DataProvider): DataProvider {
    return {
        ...provider,
        getList: (resource: string, params: GetListParams) => {
            if (
                resource !== "sequent_backend_area" ||
                !params.filter ||
                !("name" in params.filter)
            ) {
                return provider.getList(resource, params)
            }
            const {name, ...filter} = params.filter
            return provider.getList(resource, {...params, filter: {...filter, "name@_ilike": name}})
        },
    } as DataProvider
}

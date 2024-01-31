import {Sequent_Backend_Candidate} from "@/gql/graphql"

export interface Sequent_Backend_Candidate_Extended extends Sequent_Backend_Candidate {
    rowId: number
    id: string
    status: string
    winning_position?: number | null
    cast_votes?: number | null
    cast_votes_percent: number | null
}

export interface IAreasContestTabs {
    id: string
    name?: string | null
}

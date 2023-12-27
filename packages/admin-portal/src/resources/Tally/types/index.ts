import {Sequent_Backend_Candidate} from "@/gql/graphql"

export interface Sequent_Backend_Candidate_Extended extends Sequent_Backend_Candidate {
    rowId: number
    id: string
    status: string
    winning_position: number
    cast_votes: number
    turnout: number
}

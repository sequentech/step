import {Sequent_Backend_Area, Sequent_Backend_Area_Contest} from "@/gql/graphql"
import {Sequent_Backend_Candidate_Extended} from "@/resources/Tally/types"
import {atom} from "jotai"

const tallyCandidates = atom<Array<Sequent_Backend_Candidate_Extended>>([])
export const tallyAreas = atom<Array<Sequent_Backend_Area>>([])
export const tallyAreasContest = atom<Array<Sequent_Backend_Area_Contest>>([])
export const tallyGlobalAreas = atom<Array<Sequent_Backend_Area>>([])
export const tallyCandidatesList = atom<Array<Sequent_Backend_Candidate_Extended>>([])
export const tallySelectedTab = atom<number>(0)

export default tallyCandidates

import {Sequent_Backend_Area, Sequent_Backend_Area_Contest, Sequent_Backend_Results_Area_Contest, Sequent_Backend_Results_Contest} from "@/gql/graphql"
import {IAreasContestTabs, Sequent_Backend_Candidate_Extended} from "@/resources/Tally/types"
import {atom} from "jotai"

const tallyCandidates = atom<Array<Sequent_Backend_Candidate_Extended>>([])
export const tallyAreaCandidates = atom<Array<Sequent_Backend_Candidate_Extended>>([])
export const tallyAreas = atom<Array<Sequent_Backend_Area>>([])
export const tallyAreasContest = atom<Array<IAreasContestTabs>>([])
export const tallyGlobalAreas = atom<Array<Sequent_Backend_Area>>([])
export const tallyCandidatesList = atom<Array<Sequent_Backend_Candidate_Extended>>([])
export const tallySelectedTab = atom<number>(0)
export const tallyResultsEventId = atom<string | null>(null)
export const tallyGeneralData = atom<Sequent_Backend_Results_Contest | null>(null)
export const tallyAreaData = atom<Sequent_Backend_Results_Area_Contest | null>(null)

export default tallyCandidates

import {GetTallyDataQuery} from "@/gql/graphql"
import {atom} from "jotai"

export const tallyResultsEventId = atom<string | null>(null)
export const tallyQueryData = atom<GetTallyDataQuery | null>(null)


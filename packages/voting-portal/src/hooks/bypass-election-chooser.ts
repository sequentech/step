import {useQuery} from "@apollo/client"
import {isUndefined} from "@sequentech/ui-essentials"
import {useEffect, useState} from "react"
import {useParams} from "react-router-dom"
import {GetCastVotesQuery, GetElectionsQuery} from "../gql/graphql"
import {GET_CAST_VOTES} from "../queries/GetCastVotes"
import {GET_ELECTIONS} from "../queries/GetElections"
import {selectBallotStyleElectionIds} from "../store/ballotStyles/ballotStylesSlice"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import {selectElectionIds} from "../store/elections/electionsSlice"
import {useAppSelector} from "../store/hooks"

export function useBypassElectionChooser() {
    const {eventId} = useParams<{eventId?: string; tenantId?: string}>()
    const [bypassElectionChooser, setBypassElectionChooser] = useState<boolean>(false)
    const electionEvent = useAppSelector(selectElectionEventById(eventId))
    const electionIds = useAppSelector(selectElectionIds)
    const ballotStyleElectionIds = useAppSelector(selectBallotStyleElectionIds)

    const {data: castVotes, error: errorCastVote} = useQuery<GetCastVotesQuery>(GET_CAST_VOTES)

    const {data: dataElections} = useQuery<GetElectionsQuery>(GET_ELECTIONS, {
        variables: {
            electionIds: ballotStyleElectionIds,
        },
    })

    useEffect(() => {
        const newBypassChooser =
            1 === electionIds.length &&
            !errorCastVote &&
            !isUndefined(castVotes) &&
            !!electionEvent &&
            !!dataElections
        if (newBypassChooser && !bypassElectionChooser) {
            setBypassElectionChooser(newBypassChooser)
        }
    }, [castVotes, electionIds, errorCastVote, electionEvent, dataElections, bypassElectionChooser])

    return bypassElectionChooser
}

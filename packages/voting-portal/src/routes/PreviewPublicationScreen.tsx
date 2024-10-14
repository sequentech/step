import {Box} from "@mui/system"
import React, {useEffect, useMemo, useState} from "react"
import {useParams} from "react-router-dom"
import {PreviewPublicationEventType} from ".."
import { GetBallotPublicationChangesOutput } from "../gql/graphql"
import { IBallotStyle as IElectionDTO }  from "@sequentech/ui-core"
import {cloneDeep} from "lodash"
import { useAppDispatch } from "../store/hooks"
import { IBallotStyle, setBallotStyle } from "../store/ballotStyles/ballotStylesSlice"
import ElectionSelectionScreen from "./ElectionSelectionScreen"

const PreviewPublicationScreen: React.FC = () => {
    const {tenantId, documentId, areaId} = useParams<PreviewPublicationEventType>()
    const [ballotStyleJson, setBballotStyleJson] = useState<GetBallotPublicationChangesOutput>() // State to store the JSON data
    const [loading, setLoading] = useState<boolean>(true) // State for loading
    const [error, setError] = useState<string | null>(null) // State for errors
    const dispatch = useAppDispatch();

    const previewUrl = useMemo(() => {
        return `http://127.0.0.1:9000/public/tenant-${tenantId}/document-${documentId}/preview.json`;
      }, [tenantId, documentId])

    useEffect(() => {
        const fetchPreviewData = async () => {
            try {
                const response = await fetch(previewUrl)
                if (!response.ok) {
                    throw new Error(`Error: ${response.statusText}`)
                }
                const data = await response.json()
                setBballotStyleJson(data) 
            } catch (err: any) {
                setError(err.message)
            } finally {
                setLoading(false)
            }
        }

        if (tenantId && documentId) {
            fetchPreviewData()
        }
    }, [tenantId, documentId])
  
    useEffect(() => {
        if (ballotStyleJson && areaId && tenantId) {
            try {
                const ballotStyle = ballotStyleJson.current.ballot_styles.find(
                    (style: any) => style.area_id === areaId
                );
                const eml: IElectionDTO = cloneDeep(ballotStyle);

                const formattedBallotStyle: IBallotStyle = {
                    id: ballotStyle.election_id,
                    election_id: ballotStyle.election_id,
                    election_event_id: ballotStyle.election_event_id,
                    tenant_id: tenantId,
                    ballot_eml: eml,
                    ballot_signature: null,
                    created_at: "",
                    area_id: areaId,
                    annotations: null,
                    labels: null,
                    last_updated_at: "",
                }
                // dispatch(setElection({...election, image_document_id: ""}))
                dispatch(setBallotStyle(formattedBallotStyle))
                // dispatch(clearIsVoted())
                // dispatch(
                //     resetBallotSelection({
                //         ballotStyle: formattedBallotStyle,
                //     })
                // )
                
            } catch (error) {
                console.log(`Error loading fake EML: ${error}`)
                // throw new VotingPortalError(VotingPortalErrorType.INTERNAL_ERROR)
            }
        }
        
    }, [ballotStyleJson])

    return (
        <ElectionSelectionScreen/>
    )
}

export default PreviewPublicationScreen

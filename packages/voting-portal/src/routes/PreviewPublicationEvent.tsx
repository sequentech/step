// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {Outlet, useLocation, useMatch, useNavigate, useParams} from "react-router-dom"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {PageLimit} from "@sequentech/ui-essentials"
import {Box, CircularProgress} from "@mui/material"
import {PreviewPublicationEventType} from ".."
import {GetBallotPublicationChangesOutput} from "../gql/graphql"
import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {cloneDeep} from "lodash"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    IBallotStyle,
    selectBallotStyleByElectionId,
    selectFirstBallotStyle,
    setBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {AppDispatch} from "../store/store"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"

export const updateBallotStyleAndSelection = (
    ballotStyleJson: GetBallotPublicationChangesOutput,
    tenantId: string,
    areaId: string,
    dispatch: AppDispatch
) => {
    for (let ballotStyle of ballotStyleJson.current.ballot_styles) {
        if (ballotStyle.area_id !== areaId) {
            continue
        }
        try {
            const eml: IElectionDTO = cloneDeep(ballotStyle)

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
            dispatch(setBallotStyle(formattedBallotStyle))
            dispatch(
                resetBallotSelection({
                    ballotStyle: formattedBallotStyle,
                    force: true,
                })
            )
        } catch (error) {
            console.log(`Error loading EML: ${error}`)
            console.log(ballotStyle)
            throw error
        }
    }
}

export const PreviewPublicationEvent: React.FC = () => {
    const {globalSettings, setDisableAuth} = useContext(SettingsContext)
    const navigate = useNavigate()
    const {tenantId, documentId, areaId} = useParams<PreviewPublicationEventType>()
    const [ballotStyleJson, setballotStyleJson] = useState<GetBallotPublicationChangesOutput>() // State to store the JSON data
    const dispatch = useAppDispatch()
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    const location = useLocation()

    const previewUrl = useMemo(() => {
        return `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${documentId}/preview.json`
    }, [tenantId, documentId, globalSettings.PUBLIC_BUCKET_URL])

    useEffect(() => {
        const fetchPreviewData = async () => {
            if (!tenantId || !areaId || !documentId || ballotStyle) {
                return
            }
            try {
                const response = await fetch(previewUrl)
                if (!response.ok) {
                    throw new Error(`Error: ${response.statusText}`)
                }
                const ballotStyleJson = (await response.json()) as GetBallotPublicationChangesOutput
                updateBallotStyleAndSelection(ballotStyleJson, tenantId, areaId, dispatch)
            } catch (err) {
                console.log(`Error loading preview: ${err}`)
            }
        }

        fetchPreviewData()
    }, [previewUrl, tenantId, documentId, documentId, ballotStyle])

    useEffect(() => {
        if (ballotStyle?.election_event_id && tenantId && globalSettings.DISABLE_AUTH) {
            navigate(
                `/tenant/${tenantId}/event/${ballotStyle.election_event_id}/election-chooser${location.search}`
            )
        }
    }, [ballotStyle?.election_event_id, tenantId, location.search, globalSettings.DISABLE_AUTH])

    return (
        <Box sx={{flex: 1, display: "flex", justifyContent: "center", alignItems: "center"}}>
            <CircularProgress />
        </Box>
    )
}

export default PreviewPublicationEvent

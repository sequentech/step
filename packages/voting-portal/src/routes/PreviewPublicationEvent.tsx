// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {useLocation, useNavigate, useParams} from "react-router-dom"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {Box, CircularProgress} from "@mui/material"
import {PreviewPublicationEventType} from ".."
import {
    Sequent_Backend_Document,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
    Sequent_Backend_Support_Material,
} from "../gql/graphql"
import {IBallotStyle as IElectionDTO} from "@sequentech/ui-core"
import {cloneDeep} from "lodash"
import {useAppDispatch, useAppSelector} from "../store/hooks"
import {
    IBallotStyle,
    selectFirstBallotStyle,
    setBallotStyle,
} from "../store/ballotStyles/ballotStylesSlice"
import {AppDispatch} from "../store/store"
import {resetBallotSelection} from "../store/ballotSelections/ballotSelectionsSlice"
import {setElection} from "../store/elections/electionsSlice"
import {setElectionEvent} from "../store/electionEvents/electionEventsSlice"
import {setSupportMaterial} from "../store/supportMaterials/supportMaterialsSlice"
import {setDocument} from "../store/documents/documentsSlice"

interface PreviewDocument {
    ballot_styles: Array<IElectionDTO>
    elections: Array<Sequent_Backend_Election>
    election_event: Sequent_Backend_Election_Event
    support_materials: Array<Sequent_Backend_Support_Material>
    documents: Array<Sequent_Backend_Document>
}

export const updateBallotStyleAndSelection = (
    ballotStyleJson: PreviewDocument,
    tenantId: string,
    areaId: string,
    dispatch: AppDispatch
) => {
    dispatch(setElectionEvent(ballotStyleJson.election_event as any))
    for (let document of ballotStyleJson.documents) {
        dispatch(setDocument(document))
    }
    let electionsByAreaId = new Set(
        ballotStyleJson.ballot_styles
            .filter((ballot_style) => ballot_style.area_id === areaId)
            .map((ballot_style) => ballot_style.election_id)
    )
    for (let election of ballotStyleJson.elections) {
        if (electionsByAreaId.has(election.id)) {
            dispatch(
                setElection({
                    ...election,
                    image_document_id: "",
                    contests: [],
                    description: election.description ?? undefined,
                    alias: election.alias ?? undefined,
                })
            )
        }
    }
    for (let material of ballotStyleJson.support_materials) {
        dispatch(setSupportMaterial(material))
    }
    for (let ballotStyle of ballotStyleJson.ballot_styles) {
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
    const {globalSettings} = useContext(SettingsContext)
    const navigate = useNavigate()
    const {tenantId, documentId, areaId, publicationId} = useParams<PreviewPublicationEventType>()
    const dispatch = useAppDispatch()
    const ballotStyle = useAppSelector(selectFirstBallotStyle)
    const location = useLocation()

    const previewUrl = useMemo(() => {
        return `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${documentId}/${publicationId}.json`
    }, [tenantId, documentId, globalSettings.PUBLIC_BUCKET_URL])

    useEffect(() => {
        const fetchPreviewData = async () => {
            if (!tenantId || !areaId || !documentId || !publicationId || ballotStyle) {
                return
            }
            try {
                const response = await fetch(previewUrl)
                if (!response.ok) {
                    throw new Error(`Error: ${response.statusText}`)
                }
                const ballotStyleJson = (await response.json()) as PreviewDocument
                // TODO: filter elections by area_id
                setSessionStorage()
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

    const setSessionStorage = () => {
        if (!areaId || !documentId || !publicationId) {
            return
        }
        sessionStorage.setItem("isDemo", "true")
        sessionStorage.setItem("areaId", areaId)
        sessionStorage.setItem("documentId", documentId)
        sessionStorage.setItem("publicationId", publicationId)
    }

    return (
        <Box sx={{flex: 1, display: "flex", justifyContent: "center", alignItems: "center"}}>
            <CircularProgress />
        </Box>
    )
}

export default PreviewPublicationEvent

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useState} from "react"
import {Outlet, useParams, useNavigate} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import {useVoterContext} from "../hooks/useVoterContext"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {store} from "../store/store"
import {selectBallotStyleByElectionId} from "../store/ballotStyles/ballotStylesSlice"
import {useAppDispatch} from "../store/hooks"
import {setElection} from "../store/elections/electionsSlice"
import {setElectionEvent} from "../store/electionEvents/electionEventsSlice"
import {updateBallotStyleAndSelection} from "../services/BallotStyles"

export default function PublishedBallot() {
    const {electionId} = useParams()
    const {globalSettings} = useContext(SettingsContext)
    const context = useVoterContext(electionId)
    const dispatch = useAppDispatch()
    const navigate = useNavigate()
    const [ready, setReady] = useState<typeof context.data>()
    useEffect(() => {
        if (!context.data) return
        for (const election of context.data.sequent_backend_election) {
            dispatch(
                setElection({
                    ...election,
                    image_document_id: "",
                    contests: [],
                    description: election.description ?? undefined,
                })
            )
        }
        for (const event of context.data.sequent_backend_election_event)
            dispatch(setElectionEvent(event))
        const incoming = context.data.sequent_backend_ballot_style[0]
        const existing = incoming
            ? selectBallotStyleByElectionId(incoming.election_id)(store.getState())
            : undefined
        // Reauthentication may reload the same immutable style during review.
        // Retain its object and choices instead of resetting a vote in progress.
        if (incoming && existing?.id !== incoming.id) {
            updateBallotStyleAndSelection(context.data, dispatch)
            if (existing)
                navigate(
                    `/tenant/${incoming.tenant_id}/event/${incoming.election_event_id}/election/${incoming.election_id}/start`
                )
        }
        setReady(context.data)
    }, [context.data, dispatch, navigate])
    if (context.error) throw context.error
    if (!globalSettings.DISABLE_AUTH && (!ready || ready !== context.data))
        return <CircularProgress className="published-ballot-loading-progress" />
    return <Outlet />
}

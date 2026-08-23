// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {Outlet, useLocation, useMatch, useNavigate, useParams} from "react-router-dom"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {useAppSelector} from "../store/hooks"
import {selectElectionEventById} from "../store/electionEvents/electionEventsSlice"
import useUpdateTranslation from "../hooks/useUpdateTranslation"

export default function TenantEvent() {
    const navigate = useNavigate()
    const params = useParams()
    const location = useLocation()
    const {defaultLanguageTouched, setDefaultLanguageTouched} = useContext(SettingsContext)
    const electionEvent = useAppSelector(selectElectionEventById(params.eventId))

    // Own the event translation layer at the persistent route boundary so it
    // survives chooser -> voting-flow child navigation and is cleared only
    // when the event route itself changes or unmounts.
    useUpdateTranslation({electionEvent}, defaultLanguageTouched, setDefaultLanguageTouched)

    const noMatch = useMatch("/tenant/:tenantId/event/:eventId/")
    const path = `/tenant/${params.tenantId}/event/${params.eventId}/election-chooser${location.search}`

    useEffect(() => {
        if (noMatch) {
            navigate(path)
        }
    }, [navigate, params, noMatch, path])

    return <Outlet />
}

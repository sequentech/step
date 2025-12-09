// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {Outlet, useLocation, useMatch, useNavigate, useParams} from "react-router-dom"

export default function TenantEvent() {
    const navigate = useNavigate()
    const params = useParams()
    const location = useLocation()

    const noMatch = useMatch("/tenant/:tenantId/event/:eventId/")
    const path = `/tenant/${params.tenantId}/event/${params.eventId}/election-chooser${location.search}`

    useEffect(() => {
        if (noMatch) {
            navigate(path)
        }
    }, [navigate, params, noMatch, path])

    return <Outlet />
}

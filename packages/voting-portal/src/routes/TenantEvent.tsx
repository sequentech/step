// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {Outlet, useMatch, useNavigate, useParams} from "react-router-dom"

export default function TenantEvent() {
    const navigate = useNavigate()
    const params = useParams()

    const noMatch = useMatch("/tenant/:tenantId/event/:eventId/")
    const path = `/tenant/${params.tenantId}/event/${params.eventId}/election-chooser`

    useEffect(() => {
        if (noMatch) {
            navigate(path)
        }
    }, [navigate, params, noMatch, path])

    return <Outlet />
}

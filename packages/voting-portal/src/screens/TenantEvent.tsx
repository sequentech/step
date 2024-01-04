// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {Outlet, useMatch, useNavigate, useParams} from "react-router-dom"

export default function TenantEvent() {
    const navigate = useNavigate()
    const params = useParams()

    const electionChooserMatch = useMatch("/tenant/:tenantId/event/:eventId/election-chooser")
    const path = `/tenant/${params.tenantId}/event/${params.eventId}/election-chooser`

    useEffect(() => {
        if (!electionChooserMatch) {
            navigate(path)
        }
    }, [navigate, params, electionChooserMatch, path])

    return <Outlet />
}

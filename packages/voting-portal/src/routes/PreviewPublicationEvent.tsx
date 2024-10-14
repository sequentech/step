// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect} from "react"
import {Outlet, useMatch, useNavigate, useParams} from "react-router-dom"

export default function PreviewPublicationEvent() {
    const navigate = useNavigate()
    const params = useParams()

    const noMatch = useMatch("/preview/:documentId/:areaId/:token");
    const path = `/preview/${params.tenantId}/${params.documentId}/${params.areaId}/${params.token}`

    useEffect(() => {
        console.log(params.token)
        if (params.token) {
            localStorage.setItem('token', params.token);
        }
        if (noMatch) {
            navigate(path)
        }
        console.log("found myself")
    }, [navigate, params, noMatch, path])

    return <Outlet />
}

// SPDX-FileCopyrightText: 2023 Kevin Nguyen <kevin@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect} from "react"
import {Outlet, useMatch, useNavigate, useParams} from "react-router-dom"
import { SettingsContext } from "../providers/SettingsContextProvider"
import {PageLimit} from "@sequentech/ui-essentials"

export const PreviewPublicationEvent: React.FC = () => {
    const {globalSettings, setDisableAuth} = useContext(SettingsContext)
    const navigate = useNavigate()
    const params = useParams()
    console.log("Felix was here 2")

    /*const noMatch = useMatch("/preview/:documentId/:areaId");
    const path = `/preview/${params.tenantId}/${params.documentId}/${params.areaId}`

    useEffect(() => {
        setDisableAuth(true)
    }, [])

    useEffect(() => {
        if (!globalSettings.DISABLE_AUTH) {
            return
        }
        if (noMatch) {
            navigate(path)
        }
    }, [navigate, params, noMatch, path, globalSettings.DISABLE_AUTH])*/

    //return <Outlet />
    return <PageLimit maxWidth="lg" className="election-selection-screen screen">
        <div>test</div>
    </PageLimit>
}

export default PreviewPublicationEvent

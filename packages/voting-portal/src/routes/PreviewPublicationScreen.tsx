// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import {Box} from "@mui/system"
import React, {useEffect} from "react"
import {useParams} from "react-router-dom"
import {CircularProgress} from "@mui/material"
import {PreviewPublicationEventType} from ".."

const PreviewPublicationScreen: React.FC = () => {
    const {documentId, areaId} = useParams<PreviewPublicationEventType>()

    useEffect(() => {
        
    }, [])

    return (
        <Box>
            <CircularProgress />
            {
                "Cookies"
            }
        </Box>
    )
}

export default PreviewPublicationScreen

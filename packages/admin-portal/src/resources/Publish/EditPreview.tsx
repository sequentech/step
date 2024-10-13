// SPDX-FileCopyrightText: 2023-2024 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    Identifier,
    SaveButton,
    SimpleForm,
} from "react-admin"
import {Preview} from "@mui/icons-material"
import {useQuery} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {GetBallotStylesQuery} from "@/gql/graphql"
import SelectArea from "@/components/area/SelectArea"
import { GET_BALLOT_STYLES } from "@/queries/GetBallotStyles"
interface EditPreviewProps {
    electionEventId: Identifier | undefined
    close?: () => void
}

export const EditPreview: React.FC<EditPreviewProps> = (props) => {
    const {close, electionEventId} = props
    const {t} = useTranslation()
    const [renderUI, setRenderUI] = useState(false)
    const [tenantId] = useTenantStore()
    const {data: dataBallotStyles} = useQuery<GetBallotStylesQuery>(GET_BALLOT_STYLES)

    useEffect(() => {
        if (dataBallotStyles) {
            setRenderUI(true)
        }
    }, [dataBallotStyles])

    if (renderUI) {
        return (
            <SimpleForm 
                toolbar={
                    <SaveButton 
                        icon={<Preview />}
                        label={t("Preview")}
                        sx={{marginInline: "1rem"}}
                    />
                }
            >
                <SelectArea
                    tenantId={tenantId}
                    electionEventId={electionEventId}
                    source="parent_id"
                />
           </SimpleForm>
        )
    } else {
        return null
    }
}

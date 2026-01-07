// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext} from "react"

import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"

import {List, TextField, TextInput, DatagridConfigurable} from "react-admin"

import {ListActions} from "@/components/ListActions"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

const Filters: Array<ReactElement> = [
    <TextInput label="Requested By" source="requested_by" key={0} />,
]

export const SettingsPreviews: React.FC<void> = () => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const canReadPreview = authContext.isAuthorized(true, tenantId, IPermissions.PREVIEW_READ)

    const Empty = () => (
        <EmptyBox m={1}>
            <Typography variant="h4" paragraph>
                {t("settings.previewScreen.noContent")}
            </Typography>
        </EmptyBox>
    )

    if (!canReadPreview) {
        return <Empty />
    }

    return (
        <>
            <List
                resource="sequent_backend_preview"
                filters={Filters}
                actions={<ListActions withImport={false} withExport={false} />}
                empty={<Empty />}
            >
                <DatagridConfigurable>
                    <TextField source="requested_by" />
                    <TextField source="url" />
                    <TextField source="document_id" />
                </DatagridConfigurable>
            </List>
        </>
    )
}

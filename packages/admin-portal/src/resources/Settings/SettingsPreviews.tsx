// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {ReactElement, useContext, useState} from "react"
import DownloadIcon from "@mui/icons-material/Download"
import {Box, Typography} from "@mui/material"
import {useTranslation} from "react-i18next"
import {styled} from "@mui/material/styles"

import {
    List,
    TextField,
    TextInput,
    DatagridConfigurable,
    UrlField,
    FunctionField,
    Button,
} from "react-admin"

import {ListActions} from "@/components/ListActions"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {DownloadDocument} from "../User/DownloadDocument"
import {PageHeaderStyles} from "@/components/styles/PageHeaderStyles"

const EmptyBox = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

const StyledButton = styled(Button)`
    .MuiButton-startIcon {
        margin-right: 0;
    }
`

export const SettingsPreviews: React.FC<void> = () => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const [documentId, setDocumentId] = useState(undefined)
    const canReadPreview = authContext.isAuthorized(true, tenantId, IPermissions.PREVIEW_READ)

    const Filters: Array<ReactElement> = [
        <TextInput
            label={t("settings.previewScreen.table.requestedBy")}
            source="requested_by"
            key={0}
        />,
    ]

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
            <PageHeaderStyles.Title>
                {t("settings.previewScreen.table.title")}
            </PageHeaderStyles.Title>
            <PageHeaderStyles.SubTitle>
                {t("settings.previewScreen.table.description")}
            </PageHeaderStyles.SubTitle>
            <List
                resource="sequent_backend_preview"
                filters={Filters}
                actions={<ListActions withImport={false} withExport={false} />}
                empty={<Empty />}
            >
                <DatagridConfigurable>
                    <TextField
                        source="requested_by"
                        label={t("settings.previewScreen.table.requestedBy")}
                    />
                    <UrlField
                        source="url"
                        target="_blank"
                        label={t("settings.previewScreen.table.url")}
                    />
                    <FunctionField
                        source="document_id"
                        label={t("settings.previewScreen.table.document")}
                        render={(record) => (
                            <StyledButton
                                onClick={() => {
                                    setDocumentId(record.document_id)
                                }}
                                startIcon={<DownloadIcon />}
                            />
                        )}
                    />
                </DatagridConfigurable>
            </List>
            {documentId && (
                <>
                    <DownloadDocument
                        documentId={documentId ?? ""}
                        fileName={null}
                        onDownload={() => {
                            setDocumentId(undefined)
                        }}
                    />
                </>
            )}
        </>
    )
}

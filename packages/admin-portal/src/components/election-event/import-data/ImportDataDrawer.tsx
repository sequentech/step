// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {DrawerStyles} from "@/components/styles/DrawerStyles"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {Box, Drawer, MenuItem, TextField} from "@mui/material"
import {useGetList} from "react-admin"
import {ImportScreen} from "./ImportScreen"
import type {Problem} from "@sequentech/ui-essentials"
import {useCreateElectionEventStore} from "@/providers/CreateElectionEventContextProvider"

interface ImportVotersTabsProps {
    open?: boolean | null
    closeDrawer?: () => void | null
    title: string
    subtitle: string
    paragraph: string
    doImport?: (documentId: string, sha256: string, password?: string) => Promise<void> | null
    disableImport?: boolean
    uploadCallback?: (documentId: string) => Promise<void> | null
    errors?: string | null
    problems?: Problem[] | null
}

export const ImportDataDrawer: React.FC<ImportVotersTabsProps> = ({
    open = null,
    closeDrawer = null,
    title,
    subtitle,
    paragraph,
    doImport = null,
    disableImport,
    uploadCallback = null,
    errors,
    problems,
}) => {
    const {t} = useTranslation()

    const {
        authorizationEventId,
        setAuthorizationEventId,
        tenantId,
        importDrawer,
        closeImportDrawer,
        handleImportElectionEvent,
        uploadCallback: doUploadCallback,
        errors: importErrors,
        problems: importProblems,
    } = useCreateElectionEventStore()

    const {data: approvalEvents = []} = useGetList(
        "sequent_backend_election_event",
        {
            pagination: {page: 1, perPage: 250},
            filter: {tenant_id: tenantId, is_archived: false},
        },
        {enabled: !doImport && !!tenantId}
    )

    return (
        <>
            <Drawer
                anchor="right"
                open={open || importDrawer}
                onClose={() => {
                    closeDrawer ? closeDrawer() : closeImportDrawer()
                }}
                PaperProps={{
                    sx: {width: "30%"},
                }}
            >
                <Box sx={{padding: "16px"}}>
                    <ElectionHeader title={title} subtitle={subtitle} />
                    <DrawerStyles.Wrapper>
                        <>
                            <DrawerStyles.SubTitle>{t(paragraph)}</DrawerStyles.SubTitle>
                            {!doImport && (
                                <TextField
                                    select
                                    fullWidth
                                    label="Board approval for import"
                                    value={authorizationEventId}
                                    onChange={(event) =>
                                        setAuthorizationEventId(event.target.value)
                                    }
                                    helperText="Choose a Preparation event whose board has configured Import election data. Create an empty event first if needed."
                                    sx={{mb: 2}}
                                >
                                    {approvalEvents.map((event) => (
                                        <MenuItem key={event.id} value={String(event.id)}>
                                            {event.name ||
                                                event.presentation?.name ||
                                                String(event.id)}
                                        </MenuItem>
                                    ))}
                                </TextField>
                            )}
                            <ImportScreen
                                refresh="electionEventScreen.import.voters"
                                doCancel={() => {
                                    closeDrawer ? closeDrawer() : closeImportDrawer()
                                }}
                                doImport={doImport || handleImportElectionEvent}
                                disableImport={
                                    disableImport || (!doImport && !authorizationEventId)
                                }
                                uploadCallback={uploadCallback ? uploadCallback : doUploadCallback}
                                errors={errors || importErrors}
                                problems={errors ? problems : problems || importProblems}
                            />
                        </>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>
        </>
    )
}

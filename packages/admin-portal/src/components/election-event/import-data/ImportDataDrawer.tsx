// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {DrawerStyles} from "@/components/styles/DrawerStyles"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {Box, Drawer} from "@mui/material"
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
        importDrawer,
        closeImportDrawer,
        handleImportElectionEvent,
        uploadCallback: doUploadCallback,
        errors: importErrors,
        problems: importProblems,
    } = useCreateElectionEventStore()

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
                            <ImportScreen
                                refresh="electionEventScreen.import.voters"
                                doCancel={() => {
                                    closeDrawer ? closeDrawer() : closeImportDrawer()
                                }}
                                doImport={doImport || handleImportElectionEvent}
                                disableImport={disableImport || !!importErrors}
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

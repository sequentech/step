// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react"
import {DrawerStyles} from "@/components/styles/DrawerStyles"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {Box, Drawer} from "@mui/material"
import {ImportScreen} from "./ImportScreen"

interface ImportVotersTabsProps {
    open: boolean
    closeDrawer: () => void
    title: string
    subtitle: string
    paragraph: string
    doImport: (documentId: string, sha256: string, password?: string) => Promise<void>
    disableImport?: boolean
    uploadCallback?: (documentId: string) => Promise<void>
    errors: string | null
}

export const ImportDataDrawer: React.FC<ImportVotersTabsProps> = ({
    open,
    closeDrawer,
    title,
    subtitle,
    paragraph,
    doImport,
    disableImport,
    uploadCallback,
    errors,
}) => {
    const {t} = useTranslation()

    return (
        <>
            <Drawer
                anchor="right"
                open={open}
                onClose={closeDrawer}
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
                                doCancel={closeDrawer}
                                doImport={doImport}
                                disableImport={disableImport}
                                uploadCallback={uploadCallback}
                                errors={errors}
                            />
                        </>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>
        </>
    )
}

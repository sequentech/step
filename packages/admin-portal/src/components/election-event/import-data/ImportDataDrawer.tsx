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
    doImport: (documentId: string, sha256: string) => Promise<void>
}

export const ImportDataDrawer: React.FC<ImportVotersTabsProps> = ({
    open,
    closeDrawer,
    title,
    subtitle,
    doImport,
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
                            <DrawerStyles.SubTitle>
                                {t("electionEventScreen.import.votersSubtitle")}
                            </DrawerStyles.SubTitle>
                            <ImportScreen
                                refresh="electionEventScreen.import.voters"
                                doCancel={closeDrawer}
                                doImport={doImport}
                                errors={null}
                            />
                        </>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>
        </>
    )
}

import React, {useState} from "react"
import {useRecordContext} from "react-admin"
import {ImportUsersMutation, Sequent_Backend_Election_Event} from "@/gql/graphql"
import {DrawerStyles} from "@/components/styles/DrawerStyles"
import ElectionHeader from "@/components/ElectionHeader"
import {useTranslation} from "react-i18next"
import {Box, Drawer} from "@mui/material"
import {ImportScreen} from "./ImportScreen"
import {useMutation} from "@apollo/client"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useNotify} from "react-admin"
import {IMPORT_USERS} from "@/queries/ImportUsers"

interface ImportVotersTabsProps {
    open: boolean
    closeDrawer: () => void
    title: string
    subtitle: string
    doRefresh: () => void
}

export const ImportDataDrawer: React.FC<ImportVotersTabsProps> = ({
    open,
    closeDrawer,
    doRefresh,
    title,
    subtitle,
}) => {
    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const notify = useNotify()

    const [loadingImport, setLoadingImport] = React.useState<boolean>(false)
    const [tenantId] = useTenantStore()

    const [importUsers] = useMutation<ImportUsersMutation>(IMPORT_USERS)

    const handleImportVoters = async (documentId: string, sha256: string) => {
        setLoadingImport(true)

        let {data, errors} = await importUsers({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEvent.id,
                documentId: documentId,
            },
        })

        setLoadingImport(false)

        doRefresh()

        if (!errors) {
            notify(t("electionEventScreen.import.importVotersSuccess"), {type: "success"})
        } else {
            notify(t("electionEventScreen.import.importVotersError"), {type: "error"})
        }
    }

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
                                doImport={handleImportVoters}
                                isLoading={loadingImport}
                                errors={null}
                            />
                        </>
                    </DrawerStyles.Wrapper>
                </Box>
            </Drawer>
        </>
    )
}

import React, {useContext} from "react"
import {useRecordContext} from "react-admin"
import {ImportUsersMutation, Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {ReactI18NextChild, useTranslation} from "react-i18next"
import {useLocation, useNavigate} from "react-router"
import {Box, Tab} from "@mui/material"
import {ImportScreen} from "./ImportScreen"
import importDrawerState from "@/atoms/import-drawer-state"
import {useAtom} from "jotai"
import {Tabs} from "@/components/Tabs"
import {useMutation} from "@apollo/client"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useNotify} from "react-admin"
import {IMPORT_USERS} from "@/queries/ImportUsers"

interface ImportVotersTabsProps {
    doRefresh: () => void
}

export const ImportVotersTabs: React.FC<ImportVotersTabsProps> = (props) => {
    const {doRefresh} = props

    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const notify = useNotify()
    const [, setOpenImport] = useAtom(importDrawerState)

    const [loadingVoters, setLoadingVoters] = React.useState<boolean>(false)
    const [tenantId] = useTenantStore()
    const [errors, setErrors] = React.useState<String | null>(null)
    const [importUsers] = useMutation<ImportUsersMutation>(IMPORT_USERS)

    const handleCancel = () => {
        console.log("handleCancel()")
        setOpenImport(false)
    }

    const handleImportVoters = async (documentId: string, sha256: string) => {
        console.log(`handleImportVoters(documentId: ${documentId}, sha256: ${sha256})`)
        setLoadingVoters(true)
        let {data, errors} = await importUsers({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEvent.id,
                documentId: documentId,
            },
        })
        setLoadingVoters(false)
        setOpenImport(false)
        doRefresh()
        if (!errors) {
            notify(t("electionEventScreen.import.importVotersSuccess"), {type: "success"})
        } else {
            notify(t("electionEventScreen.import.importVotersError"), {type: "error"})
        }
    }

    return (
        <>
            <ElectionHeader
                title={t("electionEventScreen.import.title")}
                subtitle="electionEventScreen.import.subtitle"
            />
            <Tabs
                elements={[
                    {
                        label: t("electionEventScreen.import.voters"),
                        component: () => (
                            <ImportScreen
                                refresh="electionEventScreen.import.voters"
                                doCancel={handleCancel}
                                doImport={handleImportVoters}
                                isLoading={loadingVoters}
                                errors={errors}
                            />
                        ),
                    },
                    /*{
                        label: t("electionEventScreen.import.elections"),
                        component: () => (
                            <ImportScreen
                                refresh="electionEventScreen.import.elections"
                                doCancel={handleCancel}
                                doImport={handleImportElections}
                                isLoading={loadingElections}
                                errors={errors}
                            />
                        ),
                    },
                    {
                        label: t("electionEventScreen.import.areas"),
                        component: () => (
                            <ImportScreen
                                refresh="electionEventScreen.import.areas"
                                doCancel={handleCancel}
                                doImport={handleImportAreas}
                                isLoading={loadingAreas}
                                errors={errors}
                            />
                        ),
                    },*/
                ]}
            />

            {/* <Tabs value={value}>
                <Tab label={t("electionEventScreen.import.voters")} onClick={() => tabClicked(0)} />
                <Tab
                    label={t("electionEventScreen.import.elections")}
                    onClick={() => tabClicked(1)}
                />
                <Tab label={t("electionEventScreen.import.areas")} onClick={() => tabClicked(2)} />
            </Tabs>

            <CustomTabPanel index={0} value={value}>
                <ImportScreen
                    doCancel={handleCancel}
                    doImport={handleImportVoters}
                    isLoading={loading}
                />
            </CustomTabPanel>

            <CustomTabPanel index={1} value={value}>
                <ImportScreen
                    doCancel={handleCancel}
                    doImport={handleImportElections}
                    isLoading={true}
                />
            </CustomTabPanel>

            <CustomTabPanel index={2} value={value}>
                <ImportScreen
                    doCancel={handleCancel}
                    doImport={handleImportAreas}
                    isLoading={true}
                />
            </CustomTabPanel> */}
        </>
    )
}

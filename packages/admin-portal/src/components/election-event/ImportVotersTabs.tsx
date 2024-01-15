import React, {useContext} from "react"
import {useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
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

interface ImportVotersTabsProps {
    doRefresh: () => void
}

export const ImportVotersTabs: React.FC<ImportVotersTabsProps> = (props) => {
    const {doRefresh} = props

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [, setOpenImport] = useAtom(importDrawerState)

    const [loadingVoters, setLoadingVoters] = React.useState<boolean>(false)
    const [loadingElections, setLoadingElections] = React.useState<boolean>(false)
    const [loadingAreas, setLoadingAreas] = React.useState<boolean>(false)
    const [errors, setErrors] = React.useState<String | null>(null)

    const handleCancel = () => {
        console.log("handleCancel()")
        setOpenImport(false)
    }

    const handleImportVoters = (file: FileList | null, sha: string) => {
        // TODO: call importVoters mutation
        console.log("handleImportVoters()", file, sha)
        setLoadingVoters(true)
        setTimeout(() => {
            setLoadingVoters(false)
            setOpenImport(false)
            doRefresh()
        }, 5000)
    }

    const handleImportElections = (file: FileList | null, sha: string) => {
        console.log("handleImportElections()", file, sha)
        setLoadingElections(true)
        setTimeout(() => {
            setLoadingElections(false)
            setOpenImport(false)
            doRefresh()
        }, 5000)
    }

    const handleImportAreas = (file: FileList | null, sha: string) => {
        console.log("handleImportAreas()", file, sha)
        setLoadingAreas(true)
        setTimeout(() => {
            setLoadingAreas(false)
            setOpenImport(false)
            doRefresh()
        }, 5000)
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
                    {
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
                    },
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

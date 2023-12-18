import React, {useContext} from "react"
import {useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {ReactI18NextChild, useTranslation} from "react-i18next"
import {useLocation, useNavigate} from "react-router"
import {Box, Tab, Tabs} from "@mui/material"
import {ImportScreen} from "./ImportScreen"
import importDrawerState from '@/atoms/import-drawer-state'
import { useAtom } from 'jotai'

export const ImportVotersTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [, setOpenImport] = useAtom(importDrawerState)

    const location = useLocation()
    const navigate = useNavigate()

    const [value, setValue] = React.useState<number | null>(0)

    // const showVoters = authContext.isAuthorized(true, authContext.tenantId, IPermissions.VOTER_READ)
    // const showDashboard = authContext.isAuthorized(
    //     true,
    //     authContext.tenantId,
    //     IPermissions.ADMIN_DASHBOARD_VIEW
    // )
    // const showData = authContext.isAuthorized(
    //     true,
    //     authContext.tenantId,
    //     IPermissions.ELECTION_EVENT_WRITE
    // )

    // useEffect(() => {
    //     const locArr = location.pathname.split("/").slice(0, 3).join("/")
    //     navigate(locArr)
    // }, [])

    interface TabPanelProps {
        children?: ReactI18NextChild | Iterable<ReactI18NextChild>
        index: number
        value: number | null
    }

    function CustomTabPanel(props: TabPanelProps) {
        const {children, value, index, ...other} = props

        return (
            <div role="tabpanel" hidden={value !== index} {...other}>
                {value === index && <Box>{children}</Box>}
            </div>
        )
    }

    const tabClicked = (index: number) => {
        // setElectionId(id)
        setValue(index)
    }

    const handleCancel = () => {
        console.log("handleCancel()")
        setOpenImport(false)
    }

    const handleImportVoters = (file: FileList | null, sha: string) => {
        console.log("handleImportVoters()", file, sha)
    }

    const handleImportElections = (file: FileList | null, sha: string) => {
        console.log("handleImportElections()", file, sha)
    }

    const handleImportAreas = (file: FileList | null, sha: string) => {
        console.log("handleImportAreas()", file, sha)
    }

    return (
        <>
            <ElectionHeader
                title={t("electionEventScreen.import.title")}
                subtitle="electionEventScreen.import.subtitle"
            />

            <Tabs value={value}>
                <Tab label={t("electionEventScreen.import.voters")} onClick={() => tabClicked(0)} />
                <Tab
                    label={t("electionEventScreen.import.elections")}
                    onClick={() => tabClicked(1)}
                />
                <Tab label={t("electionEventScreen.import.areas")} onClick={() => tabClicked(2)} />
            </Tabs>

            <CustomTabPanel index={0} value={value}>
                <ImportScreen doCancel={handleCancel} doImport={handleImportVoters} isLoading={false}/>
            </CustomTabPanel>

            <CustomTabPanel index={1} value={value}>
                <ImportScreen doCancel={handleCancel} doImport={handleImportElections} isLoading={true}/>
            </CustomTabPanel>

            <CustomTabPanel index={2} value={value}>
                <ImportScreen doCancel={handleCancel} doImport={handleImportAreas} isLoading={true}/>
            </CustomTabPanel>
        </>
    )
}

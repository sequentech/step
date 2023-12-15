import React, {useContext, useEffect} from "react"
import {Button, TabbedShowLayout, useRecordContext} from "react-admin"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import ElectionHeader from "@/components/ElectionHeader"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {ReactI18NextChild, useTranslation} from "react-i18next"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {useLocation, useNavigate} from "react-router"
import {Box, Tab, Tabs, TextField} from "@mui/material"
import { DropFile } from '@sequentech/ui-essentials'

export const ImportVotersTabs: React.FC = () => {
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)

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

    const [shaField, setShaField] = React.useState<string>("")

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
                <Box sx={{padding: "16px"}}>
                    <div>VOTERS</div>
                    <TextField
                        label={t("sideMenu.search")}
                        size="small"
                        value={shaField}
                        onChange={(e) => setShaField(e.target.value)}
                    />
                    <DropFile handleFiles={(files) => console.log(files)} />
                    <div>
                        <Button label="Cancel" />
                        <Button label="Import" />
                    </div>  
                </Box>
            </CustomTabPanel>

            <CustomTabPanel index={1} value={value}>
                <Box sx={{padding: "16px"}}>
                    <div>ELECTIONS</div>
                </Box>
            </CustomTabPanel>

            <CustomTabPanel index={2} value={value}>
                <Box sx={{padding: "16px"}}>
                    <div>AREAS</div>
                </Box>
            </CustomTabPanel>

            {/* <TabbedShowLayout>
                <TabbedShowLayout.Tab label={t("electionEventScreen.import.voters")}>
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionEventScreen.import.elections")}>
                </TabbedShowLayout.Tab>
                <TabbedShowLayout.Tab label={t("electionEventScreen.import.areas")}>
                </TabbedShowLayout.Tab>
            </TabbedShowLayout> */}
        </>
    )
}

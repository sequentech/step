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
import importDrawerState from "@/atoms/import-drawer-state"
import {useAtom} from "jotai"

interface ImportVotersTabsProps {
    doRefresh: () => void
}

export const ImportElectionEvent: React.FC<ImportVotersTabsProps> = (props) => {
    const {doRefresh} = props

    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)
    const [, setOpenImport] = useAtom(importDrawerState)

    const location = useLocation()
    const navigate = useNavigate()

    const [value, setValue] = React.useState<number | null>(0)
    const [loading, setLoading] = React.useState<boolean>(false)
    const [errors, setErrors] = React.useState<String | null>(null)

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

    const handleImport = async (documentId: string, sha256: string) => {
        console.log(`handleImport(documentId: ${documentId}, sha256: ${sha256})`)
        // TODO: call importVoters mutation
        // console.log("handleImportVoters()", file, sha)
        // setLoading(true)
        // setTimeout(() => {
        //     setLoading(false)
        //     setOpenImport(false)
        //     doRefresh()
        // }, 5000)
    }

    return (
        <>
            <ElectionHeader
                title={t("electionEventScreen.import.eetitle")}
                subtitle="electionEventScreen.import.eesubtitle"
            />

            <ImportScreen
                doCancel={handleCancel}
                doImport={handleImport}
                isLoading={loading}
                errors={errors}
            />
        </>
    )
}

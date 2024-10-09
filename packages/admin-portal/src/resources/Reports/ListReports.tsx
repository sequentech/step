import {Action, ActionsColumn} from "@/components/ActionButons"
import ElectionHeader from "@/components/ElectionHeader"
import {ListActions} from "@/components/ListActions"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, styled, Typography, Button, Drawer} from "@mui/material"
import React, {ReactElement, useContext, useState} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    Identifier,
    List,
    TextField,
    useGetList,
    useSidebarState,
    WrapperField,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {CreateReport} from "./CreateReport"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {IconButton} from "@sequentech/ui-essentials"
import {CustomApolloContextProvider} from "@/providers/ApolloContextProvider"
import {EditReport} from "./EditReport"
import {
    Sequent_Backend_Election,
    Sequent_Backend_Report,
    Sequent_Backend_Template,
} from "@/gql/graphql"
import EditIcon from "@mui/icons-material/Edit"

const DataGridContainerStyle = styled(DatagridConfigurable)<{isOpenSideBar?: boolean}>`
    @media (min-width: ${({theme}) => theme.breakpoints.values.md}px) {
        overflow-x: auto;
        width: 100%;
        ${({isOpenSideBar}) =>
            `max-width: ${isOpenSideBar ? "calc(100vw - 355px)" : "calc(100vw - 108px)"};`}
        &  > div:first-child {
            position: absolute;
            width: 100%;
        }
    }
`
const TemplateEmpty = styled(Box)`
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    width: 100%;
`

interface ListReportsProps {
    electionEventId: string
}

const ListReports: React.FC<ListReportsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const [openCreateReport, setOpenCreateReport] = useState<boolean>(false)
    const [isEditReport, setIsEditReport] = useState(false)
    const [isOpenSidebar] = useSidebarState()
    const [selectedReportId, setSelectedReportId] = useState<Identifier | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const canWrtieReport = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_WRITE)
    const [openEditReport, setOpenEditReport] = useState<boolean>(false)
    const handleClose = () => {
        setOpenCreateReport(false)
        setOpenEditReport(false)
        setIsEditReport(false)
        setSelectedReportId(null)
    }

    const handleEditDrawer = (id: Identifier) => {
        setSelectedReportId(id)
        setOpenEditReport(true)
    }

    const {data: templates} = useGetList<Sequent_Backend_Template>(
        "sequent_backend_template",
        {
            pagination: {page: 1, perPage: 100},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const {data: elections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 100},
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
            },
        },
        {
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
            refetchOnWindowFocus: false,
            refetchOnReconnect: false,
            refetchOnMount: false,
        }
    )

    const OMIT_FIELDS: Array<string> = []

    const Filters: Array<ReactElement> = []

    const actions: Action[] = [{icon: <EditIcon />, action: handleEditDrawer}]

    const handleCreateDrawer = () => {
        setSelectedReportId(null)
        setOpenCreateReport(true)
    }

    const CreateButton = () => (
        <Button onClick={handleCreateDrawer}>
            <IconButton icon={faPlus} fontSize="24px" />
            {t("reportsScreen.empty.button")}
        </Button>
    )

    const ReportEmpty = () => {
        return (
            <TemplateEmpty>
                <Typography variant="h5">{t("reportsScreen.empty.header")}</Typography>
                {canWrtieReport && (
                    <>
                        <Typography variant="body1" paragraph>
                            {t("reportsScreen.empty.body")}
                        </Typography>
                        <CreateButton />
                    </>
                )}
            </TemplateEmpty>
        )
    }

    const getTemplateName = (report: Sequent_Backend_Report) => {
        let templateId = report.template_id
        const template = templates?.find((template) => template.id === templateId)
        return template?.template.alias
    }

    const getElectionName = (report: Sequent_Backend_Report) => {
        let electionId = report.election_id
        if (!electionId) return "-"
        const election = elections?.find((election) => election.id === electionId)
        return election?.name
    }

    return (
        <>
            <ElectionHeader
                title={t("reportsScreen.title")}
                subtitle={t("reportsScreen.subtitle")}
            />
            <List
                resource="sequent_backend_report"
                filter={{
                    election_event_id: electionEventId || undefined,
                    tenant_id: tenantId,
                }}
                filters={Filters}
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<ReportEmpty />}
                actions={
                    <ListActions
                        custom
                        withImport={false}
                        withExport={false}
                        withFilter={false}
                        open={openCreateReport}
                        setOpen={setOpenCreateReport}
                        Component={
                            <CreateReport close={handleClose} electionEventId={electionEventId} />
                        }
                    />
                }
            >
                <DataGridContainerStyle isOpenSideBar={isOpenSidebar} omit={OMIT_FIELDS}>
                    <TextField source="report_type" label={t("reportsScreen.fields.reportType")} />
                    <FunctionField
                        label={t("reportsScreen.fields.template")}
                        source="template_id"
                        render={getTemplateName}
                    />

                    <FunctionField
                        label={t("reportsScreen.fields.electionId")}
                        source="election_id"
                        render={getElectionName}
                    />

                    <WrapperField label={t("common.label.actions")}>
                        <ActionsColumn actions={actions} />
                    </WrapperField>
                </DataGridContainerStyle>
            </List>

            <Drawer
                anchor="right"
                open={openEditReport}
                onClose={handleClose}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <CustomApolloContextProvider role={IPermissions.template_WRITE}>
                    <EditReport
                        close={handleClose}
                        electionEventId={electionEventId}
                        reportId={selectedReportId}
                    />
                </CustomApolloContextProvider>
            </Drawer>
        </>
    )
}

export default ListReports

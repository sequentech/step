// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Action} from "@/components/ActionButons"
import ElectionHeader from "@/components/ElectionHeader"
import {ListActions} from "@/components/ListActions"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {Box, styled, Typography, Button, Drawer, IconButton} from "@mui/material"
import React, {ReactElement, useContext, useMemo, useState} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    Identifier,
    List,
    TextField,
    useGetList,
    useSidebarState,
    useDataProvider,
    useNotify,
    useGetOne,
    useRefresh,
    WrapperField,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {CustomApolloContextProvider} from "@/providers/ApolloContextProvider"
import {
    GenerateReportMutation,
    Sequent_Backend_Election,
    Sequent_Backend_Report,
    Sequent_Backend_Template,
} from "@/gql/graphql"
import EditIcon from "@mui/icons-material/Edit"
import {IconButton as IconButtonSequent} from "@sequentech/ui-essentials"
import {EditReportForm} from "./EditReportForm"
import DeleteIcon from "@mui/icons-material/Delete"
import DescriptionIcon from "@mui/icons-material/Description"
import PreviewIcon from "@mui/icons-material/Preview"
import {Dialog} from "@sequentech/ui-essentials"
import {EGenerateReportMode, EReportType, ReportActions, reportTypeConfig} from "@/types/reports"
import {GENERATE_REPORT} from "@/queries/GenerateReport"
import {useMutation} from "@apollo/client"
import {DownloadDocument} from "../User/DownloadDocument"
import {ListActionsMenu} from "@/components/ListActionsMenu"
import {el} from "intl-tel-input/i18n"

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

interface ActionsColumnProps {
    actions: Action[]
    record: Sequent_Backend_Report
    canWriteReport: boolean
}

const ListReports: React.FC<ListReportsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const [openCreateReport, setOpenCreateReport] = useState<boolean>(false)
    const [isOpenSidebar] = useSidebarState()
    const [documentId, setDocumentId] = useState<string | undefined>(undefined)
    const [isGeneratingDocument, setIsGeneratingDocument] = useState<boolean>(false)
    const [selectedReportId, setSelectedReportId] = useState<Identifier | null>(null)
    const {globalSettings} = useContext(SettingsContext)
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const notify = useNotify()
    const refresh = useRefresh()
    const {data: report} = useGetOne<Sequent_Backend_Report>("sequent_backend_report", {
        id: selectedReportId,
    })

    const [generateReport] = useMutation<GenerateReportMutation>(GENERATE_REPORT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.REPORT_READ,
            },
        },
    })
    const canWriteReport = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_WRITE)
    const [openDeleteModal, setOpenDeleteModal] = useState<boolean>(false)
    const dataProvider = useDataProvider()
    const handleClose = () => {
        console.log("closing report form")
        refresh()
        setOpenCreateReport(false)
        setSelectedReportId(null)
        setOpenDeleteModal(false)
    }

    const fileName = useMemo(() => {
        if (report) {
            const electionId = report.election_id
            const electionEventId = report.election_event_id
            const reportType = report.report_type
            return `${reportType}_${electionId}_${electionEventId}`
        } else {
            return ""
        }
    }, [report])

    const handleEditDrawer = (id: Identifier) => {
        console.log("closing report form")
        setSelectedReportId(id)
        setOpenCreateReport(true)
        setOpenDeleteModal(false)
    }

    const deleteReport = (id: Identifier) => {
        setOpenDeleteModal(true)
        setOpenCreateReport(false)
        setSelectedReportId(id)
    }

    const handleGenerateReport = async (id: Identifier, mode: EGenerateReportMode) => {
        setDocumentId(undefined)
        setSelectedReportId(id)
        setIsGeneratingDocument(true)

        try {
            let documentId = await generateReport({
                variables: {
                    reportId: id,
                    tenantId: tenantId,
                    reportMode: mode,
                },
            })
            let generated_document_id = documentId.data?.generate_report?.document_id
            if (generated_document_id) {
                setDocumentId(documentId.data?.generate_report?.document_id)
            } else {
                setIsGeneratingDocument(false)
                setSelectedReportId(null)
                notify(t("reportsScreen.messages.createError"), {type: "error"})
            }
        } catch (e) {
            setIsGeneratingDocument(false)
            setSelectedReportId(null)
            setDocumentId(undefined)
            notify(t("reportsScreen.messages.createError"), {type: "error"})
        }
    }

    const {data: reports} = useGetList<Sequent_Backend_Report>(
        "sequent_backend_report",
        {
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

    const isShowGenerateAction = (id: Identifier) => {
        const supportedReportTypes = new Set([
            EReportType.INITIALIZATION.toString(),
            EReportType.MANUAL_VERIFICATION.toString(),
            EReportType.BALLOT_RECEIPT.toString(),
            EReportType.ELECTORAL_RESULTS.toString(),
        ])

        const reportType = reports?.find((report) => report.id === id)?.report_type
        console.log("reportType", reportType)
        return reportType ? !supportedReportTypes.has(reportType) : false
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

    const actions: Action[] = [
        {
            key: ReportActions.EDIT,
            icon: <EditIcon />,
            action: handleEditDrawer,
            label: t("reportsScreen.actions.edit"),
        },
        {
            key: ReportActions.DELETE,
            icon: <DeleteIcon />,
            action: deleteReport,
            label: t("reportsScreen.actions.delete"),
        },
        {
            key: ReportActions.GENERATE,
            icon: <DescriptionIcon />,
            action: (id: Identifier) => {
                handleGenerateReport(id, EGenerateReportMode.REAL)
            },
            label: t("reportsScreen.actions.generate"),
            showAction: isShowGenerateAction,
        },
        {
            key: ReportActions.PREVIEW,
            icon: <PreviewIcon />,
            action: (id: Identifier) => {
                handleGenerateReport(id, EGenerateReportMode.PREVIEW)
            },
            label: t("reportsScreen.actions.preview"),
        },
    ]

    const handleCreateDrawer = () => {
        setSelectedReportId(null)
        setOpenCreateReport(true)
    }

    const CreateButton = () => (
        <Button onClick={handleCreateDrawer}>
            <IconButtonSequent icon={faPlus} fontSize="24px" />
            {t("reportsScreen.empty.button")}
        </Button>
    )

    const ReportEmpty = () => {
        return (
            <TemplateEmpty>
                <Typography variant="h4">{t("reportsScreen.empty.header")}</Typography>
                {canWriteReport && (
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

    const confirmDeleteAction = async () => {
        if (selectedReportId) {
            await dataProvider.delete("sequent_backend_report", {
                id: selectedReportId,
            })
            handleClose()
        }
    }

    const renderDeleteModal = () => {
        return (
            <Dialog
                variant="warning"
                open={openDeleteModal}
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteAction()
                    }
                    setOpenDeleteModal(false)
                }}
            >
                {t(`reportsScreen.delete.body`)}
            </Dialog>
        )
    }

    const getTemplateName = (report: Sequent_Backend_Report) => {
        let templateId = report.template_id
        const template = templates?.find((template) => template.id === templateId)
        return template?.template.alias ?? "-"
    }

    const getElectionName = (report: Sequent_Backend_Report) => {
        let electionId = report.election_id
        if (!electionId) return "-"
        const election = elections?.find((election) => election.id === electionId)
        return election?.name
    }

    const ActionsColumn: React.FC<ActionsColumnProps> = ({actions, record, canWriteReport}) => {
        const reportConfig = reportTypeConfig[record.report_type]

        if (!reportConfig) {
            return null
        }

        const isShowAction = (action: Action) => {
            return (
                !action.key ||
                !reportConfig.actions.includes(action.key as ReportActions) ||
                ((action.key === ReportActions.EDIT || action.key === ReportActions.DELETE) &&
                    !canWriteReport)
            )
        }

        return (
            <Box>
                {actions.map((action, index) => {
                    if (isShowAction(action)) {
                        return null
                    }

                    return (
                        <IconButton
                            key={index}
                            onClick={() => action.action(record.id)}
                            ariel-label={action.label ?? ""}
                        >
                            {action.icon}
                        </IconButton>
                    )
                })}
            </Box>
        )
    }

    const renderDownloadDocumentHelper = () => {
        if (!documentId || !isGeneratingDocument) {
            return null
        }
        return (
            <DownloadDocument
                onDownload={() => {
                    setDocumentId(undefined)
                    setSelectedReportId(null)
                    setIsGeneratingDocument(false)
                }}
                fileName={fileName}
                documentId={documentId}
                electionEventId={electionEventId}
                withProgress={true}
            />
        )
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
                            <EditReportForm
                                close={handleClose}
                                electionEventId={electionEventId}
                                tenantId={tenantId}
                                isEditReport={false}
                            />
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
                    <WrapperField source="actions" label="Actions">
                        <ListActionsMenu actions={actions} />
                    </WrapperField>
                </DataGridContainerStyle>
            </List>

            <Drawer
                anchor="right"
                open={openCreateReport}
                onClose={handleClose}
                PaperProps={{
                    sx: {width: "40%"},
                }}
            >
                <CustomApolloContextProvider role={IPermissions.template_WRITE}>
                    <EditReportForm
                        close={handleClose}
                        electionEventId={electionEventId}
                        tenantId={tenantId}
                        isEditReport={!!selectedReportId}
                        reportId={selectedReportId}
                    />
                </CustomApolloContextProvider>
            </Drawer>
            {renderDeleteModal()}
            {renderDownloadDocumentHelper()}
        </>
    )
}

export default ListReports

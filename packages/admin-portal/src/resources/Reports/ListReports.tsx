// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Action} from "@/components/ActionButons"
import ElectionHeader from "@/components/ElectionHeader"
import {ListActions} from "@/components/ListActions"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {
    Box,
    styled,
    Typography,
    Button,
    Drawer,
    IconButton,
    TextField as TextInput,
    Tooltip,
    CircularProgress,
} from "@mui/material"
import React, {ReactElement, useContext, useEffect, useMemo, useState} from "react"
import {
    DatagridConfigurable,
    FunctionField,
    Identifier,
    List,
    useGetList,
    useSidebarState,
    useDataProvider,
    useNotify,
    useGetOne,
    TextField,
    TextInput as FilterTextInput,
    useRefresh,
    WrapperField,
    FilterPayload,
    SelectInput,
} from "react-admin"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {CustomApolloContextProvider} from "@/providers/ApolloContextProvider"
import {
    GenerateReportMutation,
    ReportEncryptionPolicy,
    Sequent_Backend_Election,
    Sequent_Backend_Report,
    Sequent_Backend_Template,
} from "@/gql/graphql"
import EditIcon from "@mui/icons-material/Edit"
import {IconButton as IconButtonSequent} from "@sequentech/ui-essentials"
import LockIcon from "@mui/icons-material/Lock"
import NoEncryptionGmailerrorredIcon from "@mui/icons-material/NoEncryptionGmailerrorred"
import {EditReportForm, EReportEncryption} from "./EditReportForm"
import DeleteIcon from "@mui/icons-material/Delete"
import DescriptionIcon from "@mui/icons-material/Description"
import PreviewIcon from "@mui/icons-material/Preview"
import {Dialog} from "@sequentech/ui-essentials"
import {EGenerateReportMode, EReportType, ReportActions, reportTypeConfig} from "@/types/reports"
import {GENERATE_REPORT} from "@/queries/GenerateReport"
import {useMutation} from "@apollo/client"
import {DownloadDocument} from "../User/DownloadDocument"
import {ListActionsMenu} from "@/components/ListActionsMenu"
import {WidgetProps} from "@/components/Widget"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"
import {useReportsPermissions} from "./useReportsPermissions"
import {set} from "lodash"
import {isArray} from "@sequentech/ui-core"
import {DecryptHelp} from "@/components/election-event/export-data/PasswordDialog"
import {EventProcessors} from "../ScheduledEvents/CreateScheduledEvent"

export const decryptionCommand = `openssl enc -d -aes-256-cbc -in <encrypted_file> -out <decrypted_file> -pass pass:<password>  -md md5`

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

interface ActionsPopUpProps {
    actions: Action[]
    report: Sequent_Backend_Report
    canWriteReport: boolean
}

const ActionsPopUp: React.FC<ActionsPopUpProps> = ({actions, report, canWriteReport}) => {
    const filteredActions = useMemo(() => {
        const reportConfig = reportTypeConfig[report.report_type]

        const isShowAction = (action: Action) => {
            return (
                !action.key ||
                !reportConfig?.actions.includes(action.key as ReportActions) ||
                ((action.key === ReportActions.EDIT || action.key === ReportActions.DELETE) &&
                    !canWriteReport)
            )
        }
        return actions.filter((action) => !isShowAction(action))
    }, [report])

    return <ListActionsMenu actions={filteredActions} />
}

// filter by permission-labels

const ListReports: React.FC<ListReportsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const [openCreateReport, setOpenCreateReport] = useState<boolean>(false)
    const [isOpenSidebar] = useSidebarState()
    const [documentId, setDocumentId] = useState<string | undefined>(undefined)
    const [electionList, setElectionList] = useState<string[]>([])
    const [selectedReportId, setSelectedReportId] = useState<Identifier | null>(null)
    const [isDecryptModalOpen, setIsDecryptModalOpen] = useState<boolean>(false)
    const {globalSettings} = useContext(SettingsContext)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [tenantId] = useTenantStore()
    const authContext = useContext(AuthContext)
    const notify = useNotify()
    const refresh = useRefresh()
    const aliasRenderer = useAliasRenderer()

    const {data: report} = useGetOne<Sequent_Backend_Report>(
        "sequent_backend_report",
        {
            id: selectedReportId,
        },
        {
            enabled: !!selectedReportId,
            refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
        }
    )

    const {
        canReadReports,
        canWriteReports,
        canCreateReports,
        canDeleteReports,
        canGenerateReports,
        canPreviewReports,
        showReportsColumns,
    } = useReportsPermissions()

    const [generateReport] = useMutation<GenerateReportMutation>(GENERATE_REPORT, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.REPORT_READ,
            },
        },
    })
    // const canWriteReport = authContext.isAuthorized(true, tenantId, IPermissions.REPORT_WRITE)

    const [openDeleteModal, setOpenDeleteModal] = useState<boolean>(false)
    const dataProvider = useDataProvider()
    const handleClose = () => {
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
        setIsDecryptModalOpen(false)
        const currWidget: WidgetProps = addWidget(ETasksExecution.GENERATE_REPORT, undefined)
        try {
            let generateReportResponse = await generateReport({
                variables: {
                    reportId: id,
                    tenantId: tenantId,
                    reportMode: mode,
                    electionEventId: electionEventId,
                },
            })
            let response = generateReportResponse.data?.generate_report
            let taskId = response?.task_execution?.id
            let generatedDocumentId = response?.document_id
            let isEncrypted =
                response?.encryption_policy == ReportEncryptionPolicy.ConfiguredPassword

            if (!generatedDocumentId) {
                updateWidgetFail(currWidget.identifier)
                setSelectedReportId(null)
                setDocumentId(undefined)
                return
            }
            setDocumentId(generatedDocumentId)
            setWidgetTaskId(currWidget.identifier, taskId, () => setIsDecryptModalOpen(isEncrypted))
        } catch (e) {
            updateWidgetFail(currWidget.identifier)
            setSelectedReportId(null)
            setDocumentId(undefined)
            setIsDecryptModalOpen(false)
        }
    }

    const {data: templates} = useGetList<Sequent_Backend_Template>(
        "sequent_backend_template",
        {
            pagination: {page: 1, perPage: 1000},
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
            pagination: {page: 1, perPage: 1000},
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

    const listFilter: FilterPayload = useMemo(() => {
        const ids = elections?.map((election) => election.id)
        if (undefined !== ids && electionList !== ids) {
            setElectionList(ids)
        }
        const filter: Record<string, any> = {
            election_event_id: electionEventId,
            tenant_id: tenantId,
            _or: {
                format: "hasura-raw-query",
                value: [
                    {
                        election_id: {_in: ids ?? []},
                    },
                    {
                        election_id: {_is_null: true},
                    },
                ],
            },
        }

        return filter
    }, [electionEventId, tenantId, elections])

    const OMIT_FIELDS: Array<string> = ["id"]

    const Filters: Array<ReactElement> = [
        <SelectInput
            source="report_type"
            key="event_processor_filter"
            label={t("reportsScreen.fields.reportType")}
            choices={Object.values(EReportType).map((eventType) => ({
                id: eventType,
                name: t(`template.type.${eventType}`),
            }))}
        />,
        <SelectInput
            source="election_id"
            key="election_id_filter"
            label={t("reportsScreen.fields.electionId")}
            choices={elections?.map((election) => ({
                id: election.id,
                name: election.alias || election.name || "-",
            }))}
        />,
        <FilterTextInput label="Template" source="template_alias" key={0} />,
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
                {canCreateReports && (
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
        let templateAlias = report.template_alias
        const template = templates?.find((template) => template.alias === templateAlias)
        return template?.template.name ?? "-"
    }

    const getElectionName = (report: Sequent_Backend_Report) => {
        let electionId = report.election_id
        if (!electionId) return "-"
        const foundElection = elections?.find((election) => election.id === electionId)
        return (foundElection && aliasRenderer(foundElection)) || "-"
    }

    const getEncryptionPolicy = (report: Sequent_Backend_Report) => {
        return report.encryption_policy === EReportEncryption.CONFIGURED_PASSWORD ? (
            <LockIcon />
        ) : (
            <NoEncryptionGmailerrorredIcon />
        )
    }

    const actions: Action[] = [
        {
            key: ReportActions.EDIT,
            icon: <EditIcon />,
            action: handleEditDrawer,
            showAction: () => canWriteReports,
            label: t("reportsScreen.actions.edit"),
        },
        {
            key: ReportActions.DELETE,
            icon: <DeleteIcon />,
            action: deleteReport,
            showAction: () => canDeleteReports,
            label: t("reportsScreen.actions.delete"),
        },
        {
            key: ReportActions.GENERATE,
            icon: <DescriptionIcon />,
            action: (id: Identifier) => {
                handleGenerateReport(id, EGenerateReportMode.REAL)
            },
            showAction: () => canGenerateReports,
            label: t("reportsScreen.actions.generate"),
        },
        {
            key: ReportActions.PREVIEW,
            icon: <PreviewIcon />,
            action: (id: Identifier) => {
                handleGenerateReport(id, EGenerateReportMode.PREVIEW)
            },
            showAction: () => canPreviewReports,
            label: t("reportsScreen.actions.preview"),
        },
    ]

    const renderDownloadDocumentHelper = () => {
        if (!documentId) {
            return null
        }
        return (
            <DownloadDocument
                onDownload={() => {
                    setDocumentId(undefined)
                    setSelectedReportId(null)
                }}
                fileName={fileName}
                documentId={documentId}
                electionEventId={electionEventId}
            />
        )
    }

    if (!elections) {
        return <CircularProgress />
    }

    return (
        <>
            <ElectionHeader
                title={t("reportsScreen.title")}
                subtitle={t("reportsScreen.subtitle")}
            />
            <List
                resource="sequent_backend_report"
                filter={listFilter}
                filters={Filters}
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<ReportEmpty />}
                actions={
                    <ListActions
                        custom
                        withColumns={showReportsColumns}
                        withImport={false}
                        withExport={false}
                        withFilter={true}
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
                        withComponent={canCreateReports}
                    />
                }
                disableSyncWithLocation
            >
                <DataGridContainerStyle isOpenSideBar={isOpenSidebar} omit={OMIT_FIELDS}>
                    <TextField source="id" />
                    <FunctionField
                        label={t("reportsScreen.fields.reportType")}
                        source="report_type"
                        render={(record: {report_type: keyof typeof EReportType}) =>
                            t("template.type." + record.report_type)
                        }
                    />
                    <FunctionField
                        label={t("reportsScreen.fields.template")}
                        source="template_alias"
                        render={getTemplateName}
                    />

                    <FunctionField
                        label={t("reportsScreen.fields.electionId")}
                        source="election_id"
                        render={getElectionName}
                    />

                    <FunctionField
                        label={"encryption"}
                        source="encryption_policy"
                        render={getEncryptionPolicy}
                    />
                    {!canWriteReports &&
                    !canDeleteReports &&
                    !canGenerateReports &&
                    !canPreviewReports ? null : (
                        <WrapperField label="Actions">
                            <FunctionField
                                render={(record: Sequent_Backend_Report) => (
                                    <ActionsPopUp
                                        actions={actions}
                                        report={record}
                                        canWriteReport={canWriteReports}
                                    />
                                )}
                            />
                        </WrapperField>
                    )}
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
            <Dialog
                variant="info"
                open={isDecryptModalOpen}
                handleClose={(results) => {
                    if (results) {
                        setIsDecryptModalOpen(false)
                    }
                    setIsDecryptModalOpen(false)
                }}
                aria-labelledby="password-dialog-title"
                title={t("reportsScreen.messages.decryptFileTitle")}
                ok={"Ok"}
            >
                <DecryptHelp decryptionCommand={decryptionCommand} />
            </Dialog>
        </>
    )
}

export default ListReports

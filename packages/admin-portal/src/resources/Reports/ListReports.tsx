// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {Action} from "@/components/ActionButons"
import ElectionHeader from "@/components/ElectionHeader"
import {ListActions} from "@/components/ListActions"
import {SettingsContext} from "@/providers/SettingsContextProvider"
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
} from "@mui/material"
import React, {ReactElement, useCallback, useContext, useMemo, useState} from "react"
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
import {el} from "intl-tel-input/i18n"
import {WidgetProps} from "@/components/Widget"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ETasksExecution} from "@/types/tasksExecution"
import ContentCopyIcon from "@mui/icons-material/ContentCopy"

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
        console.log("ActionsPopUp", {report})
        const reportConfig = reportTypeConfig[report.report_type]

        const isShowAction = (action: Action) => {
            return (
                !action.key ||
                !reportConfig.actions.includes(action.key as ReportActions) ||
                ((action.key === ReportActions.EDIT || action.key === ReportActions.DELETE) &&
                    !canWriteReport)
            )
        }
        return actions.filter((action) => !isShowAction(action))
    }, [report])

    return <ListActionsMenu actions={filteredActions} />
}

const ListReports: React.FC<ListReportsProps> = ({electionEventId}) => {
    const {t} = useTranslation()
    const [openCreateReport, setOpenCreateReport] = useState<boolean>(false)
    const [isOpenSidebar] = useSidebarState()
    const [documentId, setDocumentId] = useState<string | undefined>(undefined)
    const [selectedReportId, setSelectedReportId] = useState<Identifier | null>(null)
    const [isDecryptModalOpen, setIsDecryptModalOpen] = useState<boolean>(false)
    const {globalSettings} = useContext(SettingsContext)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
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
        setIsDecryptModalOpen(false)
        const currWidget: WidgetProps = addWidget(ETasksExecution.GENERATE_REPORT)
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

            console.log(`response?.encryption_policy = ${response?.encryption_policy}`)
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

    const OMIT_FIELDS: Array<string> = ["id"]

    const Filters: Array<ReactElement> = []

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

    let decryptionCommend = `openssl enc -d -aes-256-cbc -in <encrypted_file> -out <decrypted_file> -pass pass:<password>  -md md5`
    const handleCopyPassword = () => {
        navigator.clipboard
            .writeText(decryptionCommend)
            .then(() => {
                notify(t("electionEventScreen.export.copiedSuccess"), {
                    type: "success",
                })
            })
            .catch((err) => {
                notify(t("electionEventScreen.export.copiedError"), {
                    type: "error",
                })
            })
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
                        withComponent={canWriteReport}
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
                disableSyncWithLocation
            >
                <DataGridContainerStyle isOpenSideBar={isOpenSidebar} omit={OMIT_FIELDS}>
                    <TextField source="id" />
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

                    <FunctionField
                        label={"encryption"}
                        source="encryption_policy"
                        render={getEncryptionPolicy}
                    />
                    <WrapperField label="Actions">
                        <FunctionField
                            render={(record: Sequent_Backend_Report) => (
                                <ActionsPopUp
                                    actions={actions}
                                    report={record}
                                    canWriteReport={canWriteReport}
                                />
                            )}
                        />
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
            <Dialog
                variant="info"
                open={isDecryptModalOpen}
                handleClose={(results) => {
                    if (results) {
                        console.log("results", results)
                        setIsDecryptModalOpen(false)
                    }
                    setIsDecryptModalOpen(false)
                }}
                aria-labelledby="password-dialog-title"
                title={t("reportsScreen.messages.decryptFileTitle")}
                ok={"Ok"}
            >
                <Typography sx={{whiteSpace: "pre-wrap"}}>
                    {t("reportsScreen.messages.decryptInstructions")}
                </Typography>
                <TextInput
                    fullWidth
                    value={decryptionCommend}
                    InputProps={{
                        readOnly: true,
                        endAdornment: (
                            <Tooltip
                                title={t("electionEventScreen.import.passwordDialog.copyPassword")}
                            >
                                <IconButton onClick={handleCopyPassword}>
                                    <ContentCopyIcon />
                                </IconButton>
                            </Tooltip>
                        ),
                    }}
                />
            </Dialog>
        </>
    )
}

export default ListReports

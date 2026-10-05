// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useMemo, useState} from "react"
import {
    List,
    DateField,
    FunctionField,
    TextField,
    DatagridConfigurable,
    Identifier,
    RecordContextProvider,
    SelectInput,
    TextInput,
    useListContext,
    DatagridConfigurableProps,
    useNotify,
    useRefresh,
    useReference,
    useSidebarState,
    useGetOne,
} from "react-admin"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTranslation} from "react-i18next"
import {FactCheck, Rule as RuleIcon, Visibility} from "@mui/icons-material"
import {ListActions} from "@/components/ListActions"
import {ListActionsMenu} from "@/components/ListActionsMenu"
import ElectionHeader from "@/components/ElectionHeader"
import {
    ExportApplicationMutation,
    ImportApplicationMutation,
    Sequent_Backend_Election_Event,
    GetUserProfileAttributesQuery,
    Sequent_Backend_Applications,
    UserProfileAttribute,
    Sequent_Backend_Election,
} from "@/gql/graphql"
import {Dialog} from "@sequentech/ui-essentials"
import {FormStyles} from "@/components/styles/FormStyles"
import {DownloadDocument} from "../User/DownloadDocument"
import {useMutation} from "@apollo/client"
import {EXPORT_APPLICATION} from "@/queries/ExportApplication"
import {IPermissions} from "@/types/keycloak"
import {WidgetProps} from "@/components/Widget"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {IMPORT_APPLICATION} from "@/queries/ImportApplication"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {useQuery} from "@apollo/client"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {styled} from "@mui/material/styles"
import {Box, Button, Chip, CircularProgress, Typography} from "@mui/material"
import {convertToCamelCase} from "./UtilsApprovals"
import {getAttributeLabel, getTranslationLabel} from "@/services/UserService"
import {useLocation} from "react-router-dom"
import {getPreferenceKey} from "@/lib/helpers"
import CustomDateField from "../User/CustomDateField"
import {IApplicationsStatus} from "@/types/applications"
import {ApprovalStatusChip} from "./ApprovalChips"
import {profileFieldLabel} from "./approvalMatrix"
import {
    applicantData,
    applicantName,
    decisionDetails,
    enrollmentSummary,
    waitingTime,
} from "./approvalReview"
import {Muted} from "./approvalStyles"

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = styled("div")`
    display: block;
    padding-left: 18px;
`

const CellTitle = styled("div")({
    fontWeight: 600,
    lineHeight: "22px",
})

const QueueHeader = styled(Box)({
    padding: "8px 16px 0",
})

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    onViewMatrix?: () => void
    /** Opens the matrix on the rule that decided this application. */
    onViewRule?: (id: Identifier) => void
    electionEventRecord?: Sequent_Backend_Election_Event
}

interface ApprovalsListProps extends Omit<DatagridConfigurableProps, "children"> {
    onViewApproval: (id: Identifier) => void
    onViewRule?: (id: Identifier) => void
    userAttributes: GetUserProfileAttributesQuery | undefined
    defaultFilters: string | null
}

// Storage key for the status filter
const STATUS_FILTER_KEY = "approvals_status_filter"
const NO_RECORD = {}
// The queue opens on the enrollments that wait for a person.
const DEFAULT_STATUS = "pending"
// The columns of the queue changed: column choices saved for the old list don't apply.
const PREFERENCE_KEY = "approvals_queue"

/** The Post the enrollment belongs to: its area, or the one the applicant named. */
const PostCell: React.FC<{record: Sequent_Backend_Applications}> = ({record}) => {
    const {referenceRecord} = useReference({
        reference: "sequent_backend_area",
        id: record.area_id ?? "",
        options: {enabled: !!record.area_id},
    })
    const data = applicantData(record)
    const post = referenceRecord?.name || data.embassy || "-"
    return (
        <>
            <div>{post}</div>
            {data.country && data.country !== post && <Muted>{data.country}</Muted>}
        </>
    )
}

const ApprovalsList = ({
    onViewApproval,
    onViewRule,
    userAttributes,
    defaultFilters,
    ...props
}: ApprovalsListProps) => {
    const {filterValues, setFilters} = useListContext()
    const location = useLocation()

    const {t, i18n} = useTranslation()
    const [isOpenSidebar] = useSidebarState()
    const profileAttributes = userAttributes?.get_user_profile_attributes
    const fieldLabel = useMemo(
        () => profileFieldLabel(profileAttributes ?? [], t, getAttributeLabel),
        [profileAttributes, t]
    )
    const now = useMemo(() => new Date(), [])
    const listFields = useMemo(() => {
        const omitFields: string[] = [
            "verification_type",
            "annotations.verified_by",
            "created_at",
            "id",
            "applicant_id",
        ]
        profileAttributes?.forEach((attr) => {
            omitFields.push(
                `applicant_data.${convertToCamelCase(getAttributeLabel(attr.name ?? ""))}`
            )
        })
        return {attributesFields: profileAttributes ?? [], omitFields}
    }, [profileAttributes])

    useEffect(() => {
        if (defaultFilters) {
            setFilters({...filterValues, status: defaultFilters}, {})
        }
    }, [defaultFilters])

    const renderUserFields = (fields: UserProfileAttribute[]) =>
        fields.map((attr) => {
            const attrMappedName = convertToCamelCase(getAttributeLabel(attr.name ?? ""))
            const label = getTranslationLabel(attr.name, attr.display_name, t)
            if (!attr.name) {
                return null
            }
            if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data.${attrMappedName}`}
                        label={label}
                        sortable={false}
                        render={() => (
                            <CustomDateField
                                key={attr.name}
                                base="applicant_data"
                                source={`${attrMappedName}`}
                                label={label}
                                emptyText="-"
                            />
                        )}
                    />
                )
            }
            if (attr.multivalued) {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data.${attrMappedName}`}
                        label={label}
                        sortable={false}
                        render={(record: Sequent_Backend_Applications) => {
                            const value = record?.applicant_data?.[attrMappedName]
                            const values: string[] = value ? String(value).split(";") : []
                            return values.length > 0 ? (
                                <>
                                    {values.map((item, index) => (
                                        <StyledChip key={index} label={item} />
                                    ))}
                                </>
                            ) : (
                                <StyledNull>-</StyledNull>
                            )
                        }}
                    />
                )
            }
            return (
                <FunctionField
                    key={attr.name}
                    source={`applicant_data.${attrMappedName}`}
                    label={label}
                    sortable={false}
                    render={(record: Sequent_Backend_Applications) => (
                        <span>{record?.applicant_data?.[attrMappedName] || "-"}</span>
                    )}
                />
            )
        })

    const sx = {
        "@media (min-width: 960px)": {
            overflowX: "auto",
            width: "100%",
            maxWidth: isOpenSidebar ? "calc(100vw - 355px)" : "calc(100vw - 108px)",
        },
        "& .RaDatagrid-headerCell": {fontWeight: 600},
        "& .RaDatagrid-rowCell": {verticalAlign: "middle"},
    }

    const restFields = useMemo(() => {
        localStorage.removeItem(
            `RaStore.preferences.${getPreferenceKey(
                location.pathname,
                PREFERENCE_KEY
            )}.datagrid.availableColumns`
        )
        return renderUserFields(listFields.attributesFields)
    }, [listFields.attributesFields, location.pathname])

    // Monitor and save filter changes
    useEffect(() => {
        if (filterValues?.status) {
            localStorage.setItem(STATUS_FILTER_KEY, filterValues.status)
        }
    }, [filterValues?.status])

    const isPending = (record: Sequent_Backend_Applications) =>
        String(record.status).toUpperCase() === IApplicationsStatus.PENDING
    const date = (value: string) =>
        new Date(value).toLocaleDateString(i18n.language, {
            day: "numeric",
            month: "short",
            year: "numeric",
        })

    return (
        <DatagridConfigurable
            preferenceKey={getPreferenceKey(location.pathname, PREFERENCE_KEY)}
            sx={sx}
            {...props}
            omit={listFields.omitFields}
            bulkActionButtons={false}
            rowClick={(id) => {
                onViewApproval(id)
                return false
            }}
            empty={
                <Box sx={{padding: "48px 16px", textAlign: "center"}}>
                    <Typography variant="h6">{t("approvalsScreen.list.empty.title")}</Typography>
                    <Muted>{t("approvalsScreen.list.empty.text")}</Muted>
                </Box>
            }
        >
            <FunctionField
                source="applicant_data"
                label={String(t("approvalsScreen.column.voter"))}
                sortable={false}
                render={(record: Sequent_Backend_Applications) => (
                    <>
                        <CellTitle>
                            {applicantName(record) || t("approvalsScreen.list.unnamed")}
                        </CellTitle>
                        <Muted>{applicantData(record).email}</Muted>
                    </>
                )}
            />
            <FunctionField
                source="annotations"
                label={String(t("approvalsScreen.column.what"))}
                sortable={false}
                render={(record: Sequent_Backend_Applications) => {
                    const summary = enrollmentSummary(record, t, fieldLabel)
                    return (
                        <>
                            <CellTitle>{summary.headline}</CellTitle>
                            <Muted>{summary.detail}</Muted>
                        </>
                    )
                }}
            />
            <FunctionField
                source="area_id"
                label={String(t("approvalsScreen.column.post"))}
                sortable={false}
                render={(record: Sequent_Backend_Applications) => <PostCell record={record} />}
            />
            <FunctionField
                source="updated_at"
                sortBy="created_at"
                label={String(t("approvalsScreen.column.when"))}
                render={(record: Sequent_Backend_Applications) => (
                    <>
                        {isPending(record) && (
                            <CellTitle>
                                {t("approvalsScreen.list.waiting", {
                                    time: waitingTime(record.created_at, now, t),
                                })}
                            </CellTitle>
                        )}
                        <Muted>
                            {t("approvalsScreen.list.applied", {date: date(record.created_at)})}
                        </Muted>
                    </>
                )}
            />
            <FunctionField
                source="status"
                label={String(t("approvalsScreen.column.status"))}
                sortable={false}
                render={(record: Sequent_Backend_Applications) => (
                    <ApprovalStatusChip status={record.status} />
                )}
            />
            <FunctionField
                source="verification_type"
                label={String(t("approvalsScreen.column.verificationType"))}
                render={(record: Sequent_Backend_Applications) =>
                    record.verification_type
                        ? t(`approvalsScreen.verification.${record.verification_type}`, {
                              defaultValue: record.verification_type,
                          })
                        : "-"
                }
            />
            <TextField
                source="annotations.verified_by"
                label={String(t("approvalsScreen.column.verified_by"))}
                emptyText="-"
                sortable={false}
            />
            <DateField
                showTime
                source="created_at"
                label={String(t("approvalsScreen.column.createdAt"))}
            />
            <TextField source="id" label={String(t("approvalsScreen.column.id"))} />
            <FunctionField
                source="applicant_id"
                label={String(t("approvalsScreen.column.applicantId"))}
                render={(record: Sequent_Backend_Applications) =>
                    record.applicant_id && record.applicant_id !== "null"
                        ? record.applicant_id
                        : "-"
                }
            />
            {restFields}
            <FunctionField
                source="actions"
                label={String(t("common.label.actions"))}
                sortable={false}
                render={(record: Sequent_Backend_Applications) => (
                    <span onClick={(event) => event.stopPropagation()}>
                        <ListActionsMenu
                            actions={[
                                {
                                    icon: isPending(record) ? <FactCheck /> : <Visibility />,
                                    action: onViewApproval,
                                    label: String(
                                        t(
                                            isPending(record)
                                                ? "approvalsScreen.list.review"
                                                : "approvalsScreen.list.openRecord"
                                        )
                                    ),
                                },
                                ...(onViewRule && decisionDetails(record)
                                    ? [
                                          {
                                              icon: <RuleIcon />,
                                              action: onViewRule,
                                              label: String(t("approvalsScreen.list.seeRule")),
                                          },
                                      ]
                                    : []),
                            ]}
                        />
                    </span>
                )}
            />
        </DatagridConfigurable>
    )
}

const generateFilters = (
    fields: UserProfileAttribute[],
    t: ReturnType<typeof useTranslation>["t"]
) => {
    return fields.map((attr) => {
        const source = `applicant_data.${convertToCamelCase(getAttributeLabel(attr.name ?? ""))}`
        const label = getTranslationLabel(attr.name, attr.display_name, t)

        if (attr.annotations?.inputType === "html5-date") {
            return <TextInput key={source} source={source} label={label} type="date" />
        } else if (attr.multivalued) {
            return <TextInput key={source} source={source} label={label} />
        }
        return <TextInput key={source} source={`${source}._ilike`} label={label} />
    })
}

const CustomFilters = (t: any, changeFilters: any, fields: UserProfileAttribute[]) => {
    return [
        <TextInput
            source="q"
            key="search_filter"
            label={String(t("approvalsScreen.list.search"))}
            alwaysOn
            resettable
        />,
        <SelectInput
            source="status"
            key="status_filter"
            label={String(t("approvalsScreen.column.status"))}
            choices={[
                {id: "pending", name: String(t("approvalsScreen.status.PENDING"))},
                {id: "accepted", name: String(t("approvalsScreen.status.ACCEPTED"))},
                {id: "rejected", name: String(t("approvalsScreen.status.REJECTED"))},
            ]}
            defaultValue={localStorage.getItem(STATUS_FILTER_KEY) || DEFAULT_STATUS}
            onChange={(e) => {
                if (e.target.value) {
                    localStorage.setItem(STATUS_FILTER_KEY, e.target.value)
                    changeFilters(e.target.value)
                }
            }}
        />,
        <SelectInput
            source="verification_type"
            key="verification_type_filter"
            label={String(t("approvalsScreen.column.verificationType"))}
            choices={[
                {id: "MANUAL", name: String(t("approvalsScreen.verification.MANUAL"))},
                {id: "AUTOMATIC", name: String(t("approvalsScreen.verification.AUTOMATIC"))},
            ]}
        />,
        <TextInput
            key={"applicant_id_filter"}
            source="applicant_id"
            label={String(t("approvalsScreen.column.applicantId"))}
        />,
        <TextInput key={"id_filter"} source="id" label={String(t("approvalsScreen.column.id"))} />,
        ...generateFilters(fields, t),
    ]
}

export const ListApprovals: React.FC<ListApprovalsProps> = ({
    electionEventId,
    electionId,
    onViewApproval,
    onViewMatrix,
    onViewRule,
    electionEventRecord,
}) => {
    const {t} = useTranslation()
    const location = useLocation()

    const [openExport, setOpenExport] = useState(false)
    const [exporting, setExporting] = useState(false)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const notify = useNotify()
    const [openImportDrawer, setOpenImportDrawer] = useState<boolean>(false)
    const refresh = useRefresh()
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [exportApplication] = useMutation<ExportApplicationMutation>(EXPORT_APPLICATION, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.APPLICATION_EXPORT,
            },
        },
    })
    const [importApplications] = useMutation<ImportApplicationMutation>(IMPORT_APPLICATION, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.APPLICATION_IMPORT,
            },
        },
    })

    useEffect(() => {
        localStorage.removeItem(
            `RaStore.preferences.${getPreferenceKey(
                location.pathname,
                PREFERENCE_KEY
            )}.datagrid.availableColumns`
        )
    }, [])

    // Move the useGetOne hook here and handle the undefined case
    const {data: election} = useGetOne<Sequent_Backend_Election>(
        "sequent_backend_election",
        {id: electionId || ""},
        {enabled: !!electionId} // Only fetch when electionId exists
    )

    const handleExport = () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const handleImport = () => {
        setOpenImportDrawer(true)
    }

    const handleImportApplications = async (documentId: string, sha256: string) => {
        setOpenImportDrawer(false)
        let currWidget: WidgetProps | undefined
        try {
            currWidget = addWidget(ETasksExecution.IMPORT_APPLICATION, undefined)
            let {data, errors} = await importApplications({
                variables: {
                    tenantId: electionEventRecord?.tenant_id,
                    electionEventId: electionEventRecord?.id,
                    electionId: electionId,
                    documentId,
                    sha256,
                },
            })
            const task_id = data?.import_application?.task_execution?.id
            setWidgetTaskId(currWidget.identifier, task_id)
            refresh()
        } catch (err) {
            console.log(err)
            currWidget && updateWidgetFail(currWidget.identifier)
            notify("application.import.messages.error", {type: "error"})
        }
    }

    const confirmExportAction = async () => {
        if (!electionEventRecord) {
            notify(t("approvalsScreen.export.error"))
            setOpenExport(false)
            return
        }
        let currWidget: WidgetProps | undefined
        try {
            currWidget = addWidget(ETasksExecution.EXPORT_APPLICATION, undefined)
            const {data: exportApplicationData, errors} = await exportApplication({
                variables: {
                    tenantId: electionEventRecord.tenant_id,
                    electionEventId: electionEventRecord.id,
                    electionId: electionId,
                },
            })
            setExporting(true)

            if (errors || !exportApplicationData) {
                setExporting(false)
                updateWidgetFail(currWidget.identifier)
                notify(t("approvalsScreen.export.error"))
                return
            }
            let documentId = exportApplicationData.export_application?.document_id
            const task_id = exportApplicationData?.export_application?.task_execution?.id
            setExportDocumentId(documentId)
            task_id
                ? setWidgetTaskId(currWidget.identifier, task_id)
                : updateWidgetFail(currWidget.identifier)
        } catch (err) {
            setExporting(false)
            currWidget && updateWidgetFail(currWidget.identifier)
        }
    }

    const [tenantId] = useTenantStore()
    const {data: userAttributes, loading: userAttributesLoading} =
        useQuery<GetUserProfileAttributesQuery>(USER_PROFILE_ATTRIBUTES, {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        })

    // Get initial status from localStorage or use "pending" as default
    // const defaultFilters = localStorage.getItem(STATUS_FILTER_KEY) // || "pending"
    const [defaultFilters, setDefaultFilters] = useState<string | null>(
        localStorage.getItem(STATUS_FILTER_KEY) || DEFAULT_STATUS
    )

    const listFilter = useMemo(() => {
        const filter: Record<string, any> = {
            election_event_id: electionEventId || undefined,
        }

        if (election?.permission_label) {
            filter.permission_label = election.permission_label
        }

        return filter
    }, [electionEventId, election?.permission_label, defaultFilters])

    const authContext = useContext(AuthContext)
    const canExport = authContext.isAuthorized(true, tenantId, IPermissions.APPLICATION_EXPORT)
    const canImport = authContext.isAuthorized(true, tenantId, IPermissions.APPLICATION_IMPORT)

    // add election level
    if (userAttributesLoading) {
        return null
    }

    if (!electionEventRecord) {
        return <CircularProgress aria-label={String(t("approvalsScreen.list.title"))} />
    }

    return (
        <>
            <QueueHeader>
                <ElectionHeader
                    title="approvalsScreen.list.title"
                    subtitle="approvalsScreen.list.subtitle"
                />
            </QueueHeader>
            {/* The tab sits inside the election event's record: without this, the
                filters would start from the event's own "status" and "id". */}
            <RecordContextProvider value={NO_RECORD}>
                <List
                    actions={
                        <ListActions
                            preferenceKey={getPreferenceKey(location.pathname, PREFERENCE_KEY)}
                            withImport={canImport}
                            withExport={canExport}
                            doImport={handleImport}
                            doExport={handleExport}
                            extraActions={
                                onViewMatrix
                                    ? [
                                          <Button key="approval-matrix" onClick={onViewMatrix}>
                                              <RuleIcon sx={{mr: 1}} />
                                              {t("approvalsScreen.matrix.button")}
                                          </Button>,
                                      ]
                                    : []
                            }
                        />
                    }
                    empty={false}
                    resource="sequent_backend_applications"
                    filters={CustomFilters(
                        t,
                        setDefaultFilters,
                        userAttributes?.get_user_profile_attributes || []
                    )}
                    filter={listFilter}
                    sort={{field: "created_at", order: "DESC"}}
                    perPage={10}
                    filterDefaultValues={{status: defaultFilters}}
                    disableSyncWithLocation
                >
                    <ApprovalsList
                        onViewApproval={onViewApproval}
                        onViewRule={onViewRule}
                        userAttributes={userAttributes}
                        defaultFilters={defaultFilters}
                    />
                </List>
            </RecordContextProvider>

            <Dialog
                variant="info"
                open={openExport}
                ok={String(t("application.export.button"))}
                okEnabled={() => !exporting}
                cancel={String(t("common.label.cancel"))}
                title={String(t("application.export.title"))}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                        setExporting(false)
                        setOpenExport(false)
                    } else {
                        setExportDocumentId(undefined)
                        setExporting(false)
                        setOpenExport(false)
                    }
                }}
            >
                {t("common.export")}
            </Dialog>

            <FormStyles.ReservedProgressSpace>
                {exporting && exportDocumentId ? (
                    <DownloadDocument
                        documentId={exportDocumentId}
                        electionEventId={electionEventRecord?.id || ""}
                        fileName={`export-applications.csv`}
                        onDownload={() => {
                            setExportDocumentId(undefined)
                            setExporting(false)
                            setOpenExport(false)
                            notify(t("approvalsScreen.export.success"), {
                                type: "success",
                            })
                        }}
                    />
                ) : null}
            </FormStyles.ReservedProgressSpace>

            <ImportDataDrawer
                open={openImportDrawer}
                closeDrawer={() => setOpenImportDrawer(false)}
                title="application.import.title"
                subtitle="application.import.subtitle"
                paragraph="application.import.paragraph"
                doImport={handleImportApplications}
                errors={null}
            />
        </>
    )
}

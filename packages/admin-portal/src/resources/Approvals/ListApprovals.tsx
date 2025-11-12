// SPDX-FileCopyrightText: 2024 Sequent Tech <legal@sequentech.io>
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
    SelectInput,
    TextInput,
    useListContext,
    DatagridConfigurableProps,
    useNotify,
    useRefresh,
    useSidebarState,
    useGetOne,
    useRemoveFromStore,
} from "react-admin"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTranslation} from "react-i18next"
import {Visibility} from "@mui/icons-material"
import {Action, ActionsColumn} from "@/components/ActionButons"
import {ListActions} from "@/components/ListActions"
import {
    ExportApplicationMutation,
    ImportApplicationMutation,
    Sequent_Backend_Election_Event,
    GetUserProfileAttributesQuery,
    Sequent_Backend_Applications,
    UserProfileAttribute,
    Sequent_Backend_Election,
} from "@/gql/graphql"
import {StatusApplicationChip} from "@/components/StatusApplicationChip"
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
import {Chip, CircularProgress} from "@mui/material"
import {convertToCamelCase} from "./UtilsApprovals"
import {getAttributeLabel, getTranslationLabel} from "@/services/UserService"
import {useLocation} from "react-router"
import {getPreferenceKey} from "@/lib/helpers"
import CustomDateField from "../User/CustomDateField"
import {IUser} from "@sequentech/ui-core"

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = styled("div")`
    display: block;
    padding-left: 18px;
`

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    electionEventRecord?: Sequent_Backend_Election_Event
}

interface ApprovalsListProps extends Omit<DatagridConfigurableProps, "children"> {
    omit: string[]
    actions: Action[]
    t: ReturnType<typeof useTranslation>["t"]
    userAttributes: GetUserProfileAttributesQuery | undefined
    defaultFilters: string | null
}

// Storage key for the status filter
const STATUS_FILTER_KEY = "approvals_status_filter"

const ApprovalsList = (props: ApprovalsListProps) => {
    const {filterValues, setFilters, data} = useListContext()
    const location = useLocation()

    const {t} = useTranslation()
    const [isOpenSidebar] = useSidebarState()
    const userBasicInfo = ["first_name", "last_name", "email", "username", "dateOfBirth"]
    const listFields = useMemo(() => {
        const basicInfoFields: UserProfileAttribute[] = []
        const attributesFields: UserProfileAttribute[] = []
        const omitFields: string[] = []

        props.userAttributes?.get_user_profile_attributes.forEach((attr) => {
            if (attr.name && userBasicInfo.includes(attr.name)) {
                basicInfoFields.push(attr)
            } else {
                omitFields.push(
                    `applicant_data[${convertToCamelCase(getAttributeLabel(attr.name ?? ""))}]`
                )
                attributesFields.push(attr)
            }
        })

        return {basicInfoFields, attributesFields, omitFields}
    }, [props.userAttributes?.get_user_profile_attributes])

    useEffect(() => {
        if (props.defaultFilters) {
            setFilters({...filterValues, status: localStorage.getItem(STATUS_FILTER_KEY)}, {})
        }
    }, [props.defaultFilters])

    const renderUserFields = (fields: UserProfileAttribute[]) => {
        const allFields = fields.map((attr) => {
            const attrMappedName = convertToCamelCase(getAttributeLabel(attr.name ?? ""))
            if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data.${attr.name}`}
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        render={(
                            record: Sequent_Backend_Applications,
                            source: string | undefined
                        ) => {
                            return (
                                <CustomDateField
                                    key={attr.name}
                                    base="applicant_data"
                                    source={`${attrMappedName}`}
                                    label={getTranslationLabel(attr.name, attr.display_name, t)}
                                    emptyText="-"
                                />
                            )
                        }}
                    />
                )
            } else if (attr.multivalued) {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data.${attrMappedName}` as any}
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        render={(record: Sequent_Backend_Applications) => {
                            let value = record?.applicant_data[attrMappedName]
                            let values = value ? value.split(";") : []
                            return (
                                <>
                                    {values ? (
                                        values.map((item: any, index: number) => (
                                            <StyledChip key={index} label={item} />
                                        ))
                                    ) : (
                                        <StyledNull>-</StyledNull>
                                    )}
                                </>
                            )
                        }}
                    />
                )
            }
            if (attr.name) {
                return (
                    <FunctionField
                        key={attr.name}
                        // source={`applicant_data[${attrMappedName}]` as any}
                        source={`applicant_data.${attrMappedName}` as any}
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        render={(record: Sequent_Backend_Applications) => {
                            const attributeValue = record?.applicant_data[attrMappedName]
                            if (attributeValue) {
                                return <span>{attributeValue}</span>
                            }
                            return <span>-</span>
                        }}
                    />
                )
            } else {
                return null
            }
        })

        return allFields
    }

    const sx = {
        "@media (min-width: 960px)": {
            overflowX: "auto",
            width: "100%",
            maxWidth: isOpenSidebar ? "calc(100vw - 355px)" : "calc(100vw - 108px)",
        },
    }

    const restFields = useMemo(() => {
        const theFields = []
        theFields.push(...renderUserFields(listFields?.basicInfoFields ?? []))
        theFields.push(...renderUserFields(listFields?.attributesFields ?? []))

        localStorage.removeItem(
            `RaStore.preferences.${getPreferenceKey(
                location.pathname,
                "approvals"
            )}.datagrid.availableColumns`
        )

        return theFields
    }, [listFields?.basicInfoFields, listFields?.attributesFields, location.pathname])

    // Monitor and save filter changes
    useEffect(() => {
        if (filterValues?.status) {
            localStorage.setItem(STATUS_FILTER_KEY, filterValues.status)
        }
    }, [filterValues?.status])

    return (
        <DatagridConfigurable
            preferenceKey={getPreferenceKey(location.pathname, "approvals")}
            sx={sx}
            {...props}
            omit={listFields?.omitFields}
            bulkActionButtons={false}
        >
            <TextField source="id" />
            <DateField showTime source="created_at" />
            <DateField showTime source="updated_at" />
            <FunctionField
                source="applicant_id"
                render={(record: Sequent_Backend_Applications) => {
                    if (record.applicant_id && record.applicant_id !== "null") {
                        return record.applicant_id
                    } else {
                        return "-"
                    }
                }}
            />
            <TextField source="verification_type" />
            <FunctionField
                label={String(props.t("approvalsScreen.column.status"))}
                render={(record: any) => (
                    <StatusApplicationChip status={record.status.toUpperCase()} />
                )}
            />
            <TextField
                source="annotations.verified_by"
                label={String(props.t("approvalsScreen.column.verified_by"))}
                emptyText="-"
            />
            {restFields}
            <ActionsColumn
                actions={props.actions}
                label={String(props.t("common.label.actions"))}
            />
        </DatagridConfigurable>
    )
}

const generateFilters = (
    fields: UserProfileAttribute[],
    t: ReturnType<typeof useTranslation>["t"]
) => {
    return fields.map((attr) => {
        // const source = `applicant_data[${convertToCamelCase(getAttributeLabel(attr.name ?? ""))}]`
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
        <SelectInput
            source="status"
            key="status_filter"
            label={String(t("approvalsScreen.column.status"))}
            choices={[
                {id: "pending", name: "Pending"},
                {id: "accepted", name: "Accepted"},
                {id: "rejected", name: "Rejected"},
            ]}
            defaultValue={localStorage.getItem(STATUS_FILTER_KEY)}
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
                {id: "MANUAL", name: "Manual"},
                {id: "AUTOMATIC", name: "Automatic"},
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
    electionEventRecord,
}) => {
    const {t} = useTranslation()
    const location = useLocation()

    const OMIT_FIELDS: string[] = []
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

    const {filterValues} = useListContext()

    // ✨ Admin Portal > Approvals: Add Approved By row #5050

    useEffect(() => {
        localStorage.removeItem(
            `RaStore.preferences.${getPreferenceKey(
                location.pathname,
                "approvals"
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

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]

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
        localStorage.getItem(STATUS_FILTER_KEY) || "pending"
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
        return <CircularProgress />
    }

    return (
        <>
            <List
                actions={
                    <ListActions
                        preferenceKey={getPreferenceKey(location.pathname, "approvals")}
                        withImport={canImport}
                        withExport={canExport}
                        doImport={handleImport}
                        doExport={handleExport}
                    />
                }
                // empty={false}
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
                    omit={OMIT_FIELDS}
                    actions={actions}
                    t={t}
                    userAttributes={userAttributes}
                    defaultFilters={defaultFilters}
                />
            </List>

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

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
    useGetList,
} from "react-admin"
import {AuthContext} from "@/providers/AuthContextProvider"
import {TFunction, useTranslation} from "react-i18next"
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
import eStyled from "@emotion/styled"
import {Chip, Typography} from "@mui/material"
import {convertToCamelCase} from "./UtilsApprovals"
import {getAttributeLabel} from "@/services/UserService"

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = eStyled.div`
    display: block;
    padding-left: 18px;
`

export interface ListApprovalsProps {
    electionEventId: string
    electionId?: string
    onViewApproval: (id: Identifier) => void
    electionEventRecord: Sequent_Backend_Election_Event
}

interface ApprovalsListProps extends Omit<DatagridConfigurableProps, "children"> {
    omit: string[]
    actions: Action[]
    t: TFunction
    userAttributes: GetUserProfileAttributesQuery | undefined
}

// Storage key for the status filter
const STATUS_FILTER_KEY = "approvals_status_filter"

const ApprovalsList = (props: ApprovalsListProps) => {
    const {filterValues, data, isLoading} = useListContext()

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

    const renderUserFields = (fields: UserProfileAttribute[]) =>
        fields.map((attr) => {
            const attrMappedName = convertToCamelCase(getAttributeLabel(attr.name ?? ""))
            if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data['${attr.name}']`}
                        label={getAttributeLabel(attr.display_name ?? "")}
                        render={(
                            record: Sequent_Backend_Applications,
                            source: string | undefined
                        ) => {
                            const dateValue = record?.applicant_data[attrMappedName]
                            try {
                                const date = new Date(dateValue)
                                if (isNaN(date.getTime())) {
                                    throw new Error("Invalid date")
                                }
                                return <span>{date.toLocaleDateString()}</span>
                            } catch {
                                return <span>-</span>
                            }
                        }}
                    />
                )
            } else if (attr.multivalued) {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`applicant_data[${attrMappedName}]` as any}
                        label={getAttributeLabel(attr.display_name ?? "")}
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
                        source={`applicant_data[${attrMappedName}]` as any}
                        label={getAttributeLabel(attr.display_name ?? "")}
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

    const sx = {
        "@media (min-width: 960px)": {
            "overflowX": "auto",
            "width": "100%",
            "maxWidth": isOpenSidebar ? "calc(100vw - 355px)" : "calc(100vw - 108px)",
            "& > div:first-of-type": {
                position: "absolute",
                width: "100%",
            },
        },
    }

    // Monitor and save filter changes
    useEffect(() => {
        if (filterValues?.status) {
            localStorage.setItem(STATUS_FILTER_KEY, filterValues.status)
        }
    }, [filterValues?.status])

    return (
        <div>
            <DatagridConfigurable
                sx={sx}
                {...props}
                omit={listFields.omitFields}
                bulkActionButtons={<></>}
            >
                <TextField source="id" />
                <DateField showTime source="created_at" />
                <DateField showTime source="updated_at" />
                <FunctionField
                    source="applicant_id"
                    render={(record: Sequent_Backend_Applications) => {
                        if (record.applicant_id && record.applicant_id != "null") {
                            return record.applicant_id
                        } else {
                            return "-"
                        }
                    }}
                />
                <TextField source="verification_type" />
                <FunctionField
                    label={props.t("approvalsScreen.column.status")}
                    render={(record: any) => (
                        <StatusApplicationChip status={record.status.toUpperCase()} />
                    )}
                />
                {renderUserFields(listFields.basicInfoFields)}
                {renderUserFields(listFields.attributesFields)}
                <ActionsColumn actions={props.actions} label={props.t("common.label.actions")} />
            </DatagridConfigurable>
        </div>
    )
}

const CustomFilters = () => {
    const {t} = useTranslation()

    return [
        <SelectInput
            source="status"
            key="status_filter"
            label={t("approvalsScreen.column.status")}
            choices={[
                {id: "pending", name: "Pending"},
                {id: "accepted", name: "Accepted"},
                {id: "rejected", name: "Rejected"},
            ]}
        />,
        <SelectInput
            source="verification_type"
            key="verification_type_filter"
            label={t("approvalsScreen.column.verificationType")}
            choices={[
                {id: "MANUAL", name: "Manual"},
                {id: "AUTOMATIC", name: "Automatic"},
            ]}
        />,
        <TextInput
            key={"applicant_id_filter"}
            source="applicant_id"
            label={t("approvalsScreen.column.applicantId")}
        />,
        <TextInput key={"id_filter"} source="id" label={t("approvalsScreen.column.id")} />,
    ]
}

export const ListApprovals: React.FC<ListApprovalsProps> = ({
    electionEventId,
    electionId,
    onViewApproval,
    electionEventRecord,
}) => {
    const {t} = useTranslation()
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
        try {
            await importApplications({
                variables: {
                    tenantId: electionEventRecord.tenant_id,
                    electionEventId: electionEventRecord.id,
                    electionId: electionId,
                    documentId,
                },
            })
            notify(t("application.import.messages.success"), {type: "success"})
            refresh()
        } catch (err) {
            console.log(err)
            notify("application.import.messages.error", {type: "error"})
        }
    }

    const confirmExportAction = async () => {
        if (!electionEventRecord) {
            notify(t("tasksScreen.exportApplication.error"))
            setOpenExport(false)
            return
        }
        let currWidget: WidgetProps | undefined
        try {
            currWidget = addWidget(ETasksExecution.EXPORT_APPLICATION)
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
                notify(t("tasksScreen.exportTasksExecution.error"))
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
            console.log(err)
        }
    }

    const actions: Action[] = [
        {
            icon: <Visibility />,
            action: onViewApproval,
        },
    ]

    const [tenantId] = useTenantStore()
    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        }
    )

    const {data: applicationData} = useGetList<Sequent_Backend_Applications>(
        "sequent_backend_applications",
        {
            filter: {tenant_id: tenantId},
            pagination: {page: 1, perPage: 1},
        }
    )

    // Get initial status from localStorage or use "pending" as default
    const initialStatus = localStorage.getItem(STATUS_FILTER_KEY) || "pending"

    const authContext = useContext(AuthContext)
    const canExport =
        authContext.isAuthorized(true, tenantId, IPermissions.APPLICATION_EXPORT) &&
        applicationData &&
        applicationData.length > 0
    const canImport = authContext.isAuthorized(true, tenantId, IPermissions.APPLICATION_IMPORT)

    return (
        <>
            <List
                actions={
                    <ListActions
                        withImport={canImport}
                        withExport={canExport}
                        doImport={handleImport}
                        doExport={handleExport}
                    />
                }
                resource="sequent_backend_applications"
                filters={CustomFilters()}
                filter={{election_event_id: electionEventId || undefined}}
                sort={{field: "created_at", order: "DESC"}}
                perPage={10}
                filterDefaultValues={{status: initialStatus}}
                disableSyncWithLocation
                storeKey="approvals-list"
            >
                <ApprovalsList
                    omit={OMIT_FIELDS}
                    actions={actions}
                    t={t}
                    userAttributes={userAttributes}
                />
            </List>
            <Dialog
                variant="info"
                open={openExport}
                ok={t("application.export.button")}
                okEnabled={() => !exporting}
                cancel={t("common.label.cancel")}
                title={t("application.export.title")}
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
                            console.log("onDownload called")
                            setExportDocumentId(undefined)
                            setExporting(false)
                            setOpenExport(false)
                            notify(t("tasksScreen.exportTasksExecution.success"), {
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

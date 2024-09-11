// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext, useMemo} from "react"
import {
    DatagridConfigurable,
    List,
    TextField,
    TextInput,
    BooleanField,
    Identifier,
    WrapperField,
    useRefresh,
    useNotify,
    useGetList,
    FunctionField,
    Button as ReactAdminButton,
    useRecordContext,
    BooleanInput,
    SelectInput,
    DateInput,
    useSidebarState,
} from "react-admin"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import UploadIcon from "@mui/icons-material/Upload"
import {ListActions} from "@/components/ListActions"
import {Box, Button, Chip, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {Action, ActionsColumn} from "@/components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import MailIcon from "@mui/icons-material/Mail"
import CreditScoreIcon from "@mui/icons-material/CreditScore"
import PasswordIcon from "@mui/icons-material/Password"
import DeleteIcon from "@mui/icons-material/Delete"
import {EditUser} from "./EditUser"
import {AudienceSelection, SendCommunication} from "./SendCommunication"
import {CreateUser} from "./CreateUser"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    DeleteUserMutation,
    DeleteUsersMutation,
    ExportTenantUsersMutation,
    ExportUsersMutation,
    GetDocumentQuery,
    GetUserProfileAttributesQuery,
    ImportUsersMutation,
    ManualVerificationMutation,
    Sequent_Backend_Area,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {DELETE_USER} from "@/queries/DeleteUser"
import {GET_DOCUMENT} from "@/queries/GetDocument"
import {MANUAL_VERIFICATION} from "@/queries/ManualVerification"
import {useLazyQuery, useMutation, useQuery} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {IRole, IUser} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {FormStyles} from "@/components/styles/FormStyles"
import {EXPORT_USERS} from "@/queries/ExportUsers"
import {EXPORT_TENANT_USERS} from "@/queries/ExportTenantUsers"
import {DownloadDocument} from "./DownloadDocument"
import {IMPORT_USERS} from "@/queries/ImportUsers"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {getAttributeLabel, userBasicInfo} from "@/services/UserService"
import CustomDateField from "./CustomDateField"
import {ActionsMenu} from "@/components/ActionsMenu"
import EditPassword from "./EditPassword"
import {styled} from "@mui/material/styles"
import {DELETE_USERS} from "@/queries/DeleteUsers"

const OMIT_FIELDS: Array<string> = ["email_verified"]

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
export interface ListUsersProps {
    aside?: ReactElement
    electionEventId?: string
    electionId?: string
}

function useGetPublicDocumentUrl() {
    const [tenantId] = useTenantStore()
    const {globalSettings} = React.useContext(SettingsContext)

    function getDocumentUrl(documentId: string, documentName: string): string {
        return encodeURI(
            `${globalSettings.PUBLIC_BUCKET_URL}tenant-${tenantId}/document-${documentId}/${documentName}`
        )
    }

    return {
        getDocumentUrl,
    }
}

export const ListUsers: React.FC<ListUsersProps> = ({aside, electionEventId, electionId}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const [isOpenSidebar] = useSidebarState()

    const [open, setOpen] = React.useState(false)
    const [openExport, setOpenExport] = React.useState(false)
    const [exporting, setExporting] = React.useState(false)
    const [exportDocumentId, setExportDocumentId] = React.useState<string | undefined>()
    const [openNew, setOpenNew] = React.useState(false)
    const [audienceSelection, setAudienceSelection] = React.useState<AudienceSelection>(
        AudienceSelection.SELECTED
    )
    const [polling, setPolling] = React.useState<NodeJS.Timer | null>(null)
    const [documentId, setDocumentId] = React.useState<string | null>(null)
    const [documentOpened, setDocumentOpened] = React.useState<boolean>(false)
    const [documentUrl, setDocumentUrl] = React.useState<string | null>(null)
    const [getDocument, {data: documentData}] = useLazyQuery<GetDocumentQuery>(GET_DOCUMENT)
    const documentUrlRef = React.useRef(documentUrl)
    const {getDocumentUrl} = useGetPublicDocumentUrl()

    const [openSendCommunication, setOpenSendCommunication] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [openManualVerificationModal, setOpenManualVerificationModal] = React.useState(false)
    const [openDeleteBulkModal, setOpenDeleteBulkModal] = React.useState(false)
    const [openEditPassword, setOpenEditPassword] = React.useState(false)
    const [selectedIds, setSelectedIds] = React.useState<Identifier[]>([])
    const [deleteId, setDeleteId] = React.useState<string | undefined>()
    const [openDrawer, setOpenDrawer] = React.useState<boolean>(false)
    const [openImportDrawer, setOpenImportDrawer] = React.useState<boolean>(false)
    const [recordIds, setRecordIds] = React.useState<Array<Identifier>>([])
    const authContext = useContext(AuthContext)
    const refresh = useRefresh()
    const [deleteUser] = useMutation<DeleteUserMutation>(DELETE_USER)
    const [getManualVerificationPdf] = useMutation<ManualVerificationMutation>(MANUAL_VERIFICATION)
    const [deleteUsers] = useMutation<DeleteUsersMutation>(DELETE_USERS)
    const [exportUsers] = useMutation<ExportUsersMutation>(EXPORT_USERS)
    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        }
    )

    const {data: areas} = useGetList<Sequent_Backend_Area>("sequent_backend_area", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId, tenant_id: tenantId},
    })

    const Filters = useMemo(() => {
        let filters: ReactElement[] = []
        if (userAttributes?.get_user_profile_attributes) {
            filters = userAttributes.get_user_profile_attributes.map((attr) => {
                //covert to valid source string (if attr name is for example sequent.read-only.otp-method)
                const source = attr.name?.replaceAll(".", "%")
                if (attr.annotations?.inputType === "html5-date") {
                    return (
                        <DateInput
                            key={attr.name}
                            source={`attributes.${attr.name}`}
                            label={attr.display_name ?? ""}
                        />
                    )
                }
                return (
                    <TextInput
                        key={attr.name}
                        source={
                            userBasicInfo.includes(`${attr.name}`)
                                ? `${attr.name}`
                                : `attributes.${source}`
                        }
                        label={getAttributeLabel(attr.display_name ?? "")}
                    />
                )
            })
            filters.push(<BooleanInput key="enabled" source={"enabled"} />)
            filters.push(<BooleanInput key="email_verified" source={"email_verified"} />)
            if (electionEventId && areas) {
                filters.push(
                    <SelectInput
                        source="attributes.area-id"
                        choices={areas?.map((area) => {
                            return {id: area.id, name: area.name}
                        })}
                        label={t("usersAndRolesScreen.users.fields.area")}
                    />
                )
            }
            if (electionEventId && !electionId) {
                filters.push(
                    <BooleanInput
                        key="has_voted"
                        source={"has_voted"}
                        label={t("usersAndRolesScreen.users.fields.has_voted")}
                    />
                )
            }
        }
        return filters
    }, [userAttributes?.get_user_profile_attributes])

    console.log("userAttributes ", userAttributes)

    const [exportTenantUsers] = useMutation<ExportTenantUsersMutation>(EXPORT_TENANT_USERS, {
        context: {
            headers: {
                "x-hasura-role": IPermissions.USER_READ,
            },
        },
    })

    const notify = useNotify()
    const {data: rolesList} = useGetList<IRole & {id: string}>("role", {
        pagination: {page: 1, perPage: 9999},
        sort: {field: "last_updated_at", order: "DESC"},
        filter: {
            tenant_id: tenantId,
        },
    })
    const canEditUsers = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_WRITE)
    const canSendCommunications = authContext.isAuthorized(
        true,
        tenantId,
        IPermissions.NOTIFICATION_SEND
    )

    const fetchData = async (documentId: string) => {
        let {data, error} = await getDocument({
            variables: {
                id: documentId,
                tenantId,
            },
            fetchPolicy: "network-only",
        })
    }

    function startPolling(documentId: string) {
        if (!polling) {
            fetchData(documentId)

            const intervalId = setInterval(() => {
                fetchData(documentId)
            }, 1000)

            setPolling(intervalId)

            setTimeout(() => {
                clearInterval(intervalId)
                setPolling(null)
                if (!documentUrlRef.current) {
                    notify(t("usersAndRolesScreen.voters.notifications.manualVerificationError"), {
                        type: "error",
                    })
                }
            }, 5 * globalSettings.QUERY_POLL_INTERVAL_MS)
        }
    }

    React.useEffect(() => {
        documentUrlRef.current = documentUrl
    }, [documentUrl])

    React.useEffect(() => {
        function stopPolling() {
            if (polling) {
                clearInterval(polling)
                setPolling(null)
            }
        }

        if (documentData && documentData?.sequent_backend_document?.length > 0) {
            let name = documentData?.sequent_backend_document[0]?.name
            stopPolling()

            if (name && !documentOpened) {
                const newDocumentUrl = getDocumentUrl(documentId!, name)

                setDocumentUrl(newDocumentUrl)
                setDocumentOpened(true)

                setTimeout(() => {
                    // We use a setTimeout as a work around due to this issue in React:
                    // https://stackoverflow.com/questions/76944918/should-not-already-be-working-on-window-open-in-simple-react-app
                    // https://github.com/facebook/react/issues/17355
                    window.open(newDocumentUrl, "_blank")
                }, 0)
            }
        }
    }, [
        electionEventId,
        documentUrl,
        documentOpened,
        polling,
        documentData,
        documentId,
        getDocumentUrl,
    ])

    React.useEffect(() => {
        return () => {
            if (polling) {
                clearInterval(polling)
            }
        }
    }, [polling])

    const handleClose = () => {
        setRecordIds([])
        setOpenSendCommunication(false)
        setOpenDeleteModal(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenDrawer(false)
        setOpenNew(false)
        setOpen(false)
    }

    const editAction = (id: Identifier) => {
        setOpen(true)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenSendCommunication(false)
        setRecordIds([id as string])
    }

    const sendCommunicationForIdAction = (id: Identifier) => {
        sendCommunicationAction([id])
    }

    const sendCommunicationAction = (
        ids: Array<Identifier>,
        audienceSelection = AudienceSelection.SELECTED
    ) => {
        setOpen(false)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenSendCommunication(true)

        setAudienceSelection(audienceSelection)
        setRecordIds(ids)
    }

    const deleteAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        setOpen(false)
        setOpenNew(false)
        setOpenSendCommunication(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenDeleteModal(true)
        setDeleteId(id as string)
    }

    const manualVerificationAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        setOpen(false)
        setOpenNew(false)
        setOpenSendCommunication(false)
        setOpenManualVerificationModal(true)
        setOpenDeleteBulkModal(false)
        setOpenDeleteModal(false)
        setRecordIds([id])
    }

    const editPasswordAction = (id: Identifier) => {
        setOpen(false)
        setOpenNew(false)
        setOpenSendCommunication(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenDeleteModal(false)
        setOpenEditPassword(true)
        setRecordIds([id as string])
    }

    const confirmManualVerificationAction = async () => {
        const {errors, data} = await getManualVerificationPdf({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
                voterId: recordIds[0],
            },
        })

        if (errors) {
            console.log(`Error manually verifying user: ${errors}`)
            notify(t("usersAndRolesScreen.voters.notifications.manualVerificationError"), {
                type: "error",
            })
            setRecordIds([])
            refresh()
            return
        }
        console.log(`fetchData success: ${data}`)
        notify(t("usersAndRolesScreen.voters.notifications.manualVerificationSuccess"), {
            type: "success",
        })
        setRecordIds([])
        refresh()

        let docId = data?.get_manual_verification_pdf?.document_id
        if (docId) {
            setDocumentId(docId)
            startPolling(docId)
            setDocumentOpened(false)
        }
    }

    const confirmDeleteAction = async () => {
        const {errors} = await deleteUser({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
                userId: deleteId,
            },
        })
        if (errors) {
            notify(
                t(
                    `usersAndRolesScreen.${
                        electionEventId ? "voters" : "users"
                    }.notifications.deleteError`
                ),
                {type: "error"}
            )
            console.log(`Error deleting user: ${errors}`)
            return
        }
        notify(
            t(
                `usersAndRolesScreen.${
                    electionEventId ? "voters" : "users"
                }.notifications.deleteSuccess`
            ),
            {type: "success"}
        )
        setDeleteId(undefined)
        refresh()
    }

    const actions: Action[] = [
        {
            icon: <MailIcon />,
            action: sendCommunicationForIdAction,
            showAction: () => canSendCommunications,
            label: t(`sendCommunication.send`),
        },
        {
            icon: <EditIcon />,
            action: editAction,
            showAction: () => canEditUsers,
            label: t(`common.label.edit`),
        },
        {
            icon: <DeleteIcon />,
            action: deleteAction,
            showAction: () => canEditUsers,
            label: t(`common.label.delete`),
        },
        {
            icon: <CreditScoreIcon />,
            action: manualVerificationAction,
            showAction: () => canEditUsers,
            label: t(`usersAndRolesScreen.voters.manualVerification.label`),
        },
        {
            icon: <PasswordIcon />,
            action: editPasswordAction,
            showAction: () => canEditUsers,
            label: t(`usersAndRolesScreen.editPassword.label`),
        },
    ]

    async function confirmDeleteBulkAction() {
        console.log(selectedIds)

        const {errors} = await deleteUsers({
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
                usersId: selectedIds,
            },
        })

        if (errors) {
            notify(
                t(
                    `usersAndRolesScreen.${
                        electionEventId ? "voters" : "users"
                    }.notifications.deleteError`
                ),
                {type: "error"}
            )
            return
        }

        notify(
            t(
                `usersAndRolesScreen.${
                    electionEventId ? "voters" : "users"
                }.notifications.multipleDeleteSuccess`
            ),
            {type: "success"}
        )

        refresh()
    }

    // @ts-ignore
    function BulkActions(props) {
        return (
            <>
                {canSendCommunications && (
                    <Button
                        variant="actionbar"
                        key="send-notification"
                        onClick={() => {
                            sendCommunicationAction(
                                props.selectedIds ?? [],
                                AudienceSelection.SELECTED
                            )
                        }}
                    >
                        <ResourceListStyles.MailIcon />
                        {t(`sendCommunication.send`)}
                    </Button>
                )}

                {canEditUsers && (
                    <Button
                        variant="actionbar"
                        onClick={() => {
                            setSelectedIds(props.selectedIds)
                            setOpenDeleteBulkModal(true)
                        }}
                    >
                        <ResourceListStyles.DeleteIcon />
                        {t("common.label.delete")}
                    </Button>
                )}
            </>
        )
    }

    const handleImport = () => {
        setOpenImportDrawer(true)
    }

    const handleExport = () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const confirmExportAction = async () => {
        try {
            setExportDocumentId(undefined)
            setExporting(true)

            if (electionEventId) {
                const {data: exportUsersData, errors} = await exportUsers({
                    variables: {tenantId, electionEventId, electionId},
                })
                if (errors || !exportUsersData) {
                    setExporting(false)
                    setOpenExport(false)
                    notify(t(`usersAndRolesScreen.${"voters"}.notifications.exportError`), {
                        type: "error",
                    })
                    return
                }
                let documentId = exportUsersData.export_users?.document_id
                setExportDocumentId(documentId)
            } else {
                const {data: exportUsersData, errors} = await exportTenantUsers({
                    variables: {tenantId},
                })

                if (errors || !exportUsersData) {
                    setExporting(false)
                    setOpenExport(false)
                    notify(t(`usersAndRolesScreen.${"users"}.notifications.exportError`), {
                        type: "error",
                    })
                    return
                }
                let documentId = exportUsersData.export_tenant_users?.document_id
                setExportDocumentId(documentId)
            }
        } catch (err) {
            console.log(err)
        }
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.emptyHeader`)}
            </Typography>
            {canEditUsers ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.askCreate`)}
                    </Typography>
                    <ResourceListStyles.EmptyButtonList>
                        <Button onClick={() => setOpenNew(true)}>
                            <ResourceListStyles.CreateIcon icon={faPlus} />
                            {t(
                                `usersAndRolesScreen.${
                                    electionEventId ? "voters" : "users"
                                }.create.subtitle`
                            )}
                        </Button>
                        <ReactAdminButton onClick={handleImport} label={t("common.label.import")}>
                            <UploadIcon />
                        </ReactAdminButton>
                    </ResourceListStyles.EmptyButtonList>
                </>
            ) : null}
        </ResourceListStyles.EmptyBox>
    )

    const electionEvent = useRecordContext<Sequent_Backend_Election_Event>()
    const [importUsers] = useMutation<ImportUsersMutation>(IMPORT_USERS)

    const handleImportVoters = async (documentId: string, sha256: string) => {
        let {data, errors} = await importUsers({
            variables: {
                tenantId,
                documentId,
                electionEventId: electionEvent.id,
            },
        })

        refresh()

        if (!errors) {
            notify(t("electionEventScreen.import.importVotersSuccess"), {type: "success"})
        } else {
            notify(t("electionEventScreen.import.importVotersError"), {type: "error"})
        }
    }

    return (
        <>
            <List
                resource="user"
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={
                    <ListActions
                        withImport
                        doImport={handleImport}
                        withExport
                        doExport={handleExport}
                        open={openDrawer}
                        setOpen={setOpenDrawer}
                        Component={
                            <CreateUser
                                electionEventId={electionEventId}
                                close={handleClose}
                                rolesList={rolesList || []}
                                userAttributes={userAttributes?.get_user_profile_attributes || []}
                                areas={areas}
                            />
                        }
                        extraActions={[
                            <Button
                                key="send-notification"
                                onClick={() => {
                                    sendCommunicationAction([], AudienceSelection.ALL_USERS)
                                }}
                            >
                                <ResourceListStyles.MailIcon />
                                {t("sendCommunication.send")}
                            </Button>,
                        ]}
                    />
                }
                filter={{
                    tenant_id: tenantId,
                    election_event_id: electionEventId,
                    election_id: electionId,
                }}
                aside={aside}
                filters={Filters}
            >
                {userAttributes?.get_user_profile_attributes && (
                    <DataGridContainerStyle
                        isOpenSideBar={isOpenSidebar}
                        bulkActionButtons={<BulkActions />}
                    >
                        {/* <DatagridConfigurable> */}
                        <TextField source="id" sx={{display: "block", width: "280px"}} />
                        <BooleanField source="email_verified" />
                        <BooleanField source="enabled" />
                        {userAttributes.get_user_profile_attributes.map((attr) => {
                            if (attr.annotations?.inputType === "html5-date") {
                                return (
                                    <CustomDateField
                                        key={attr.name}
                                        source={`${attr.name}`}
                                        label={getAttributeLabel(attr.display_name ?? "")}
                                        emptyText=""
                                    />
                                )
                            }
                            return (
                                <TextField
                                    key={attr.name}
                                    source={
                                        attr.name && userBasicInfo.includes(attr.name)
                                            ? attr.name
                                            : `attributes['${attr.name}']`
                                    }
                                    label={getAttributeLabel(attr.display_name ?? "")}
                                />
                            )
                        })}
                        {electionEventId && (
                            <FunctionField
                                label={t("usersAndRolesScreen.users.fields.area")}
                                render={(record: IUser) =>
                                    record?.area?.name ? (
                                        <Chip label={record?.area?.name ?? ""} />
                                    ) : (
                                        "-"
                                    )
                                }
                            />
                        )}
                        {electionEventId && (
                            <FunctionField
                                source="has_voted"
                                label={t("usersAndRolesScreen.users.fields.has_voted")}
                                render={(record: IUser, source: string | undefined) => {
                                    let newRecord = {
                                        has_voted: (record?.votes_info?.length ?? 0) > 0,
                                        ...record,
                                    }
                                    return <BooleanField record={newRecord} source={source} />
                                }}
                            />
                        )}
                        <WrapperField source="actions" label="Actions">
                            <ActionsMenu actions={actions} />
                        </WrapperField>
                        {/* </DatagridConfigurable> */}
                    </DataGridContainerStyle>
                )}
            </List>
            <ResourceListStyles.Drawer anchor="right" open={open} onClose={handleClose}>
                <EditUser
                    id={recordIds[0] as string}
                    electionEventId={electionEventId}
                    close={handleClose}
                    rolesList={rolesList || []}
                    userAttributes={userAttributes?.get_user_profile_attributes || []}
                    areas={areas}
                />
            </ResourceListStyles.Drawer>
            <ResourceListStyles.Drawer
                anchor="right"
                open={openSendCommunication}
                onClose={handleClose}
            >
                <SendCommunication
                    ids={recordIds}
                    audienceSelection={audienceSelection}
                    electionEventId={electionEventId}
                    close={handleClose}
                />
            </ResourceListStyles.Drawer>
            <ResourceListStyles.Drawer anchor="right" open={openNew} onClose={handleClose}>
                <CreateUser
                    electionEventId={electionEventId}
                    close={handleClose}
                    rolesList={rolesList || []}
                    userAttributes={userAttributes?.get_user_profile_attributes || []}
                    areas={areas}
                />
            </ResourceListStyles.Drawer>
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
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.delete.body`)}
            </Dialog>
            <Dialog
                variant="warning"
                open={openManualVerificationModal}
                ok={t("usersAndRolesScreen.voters.manualVerification.verify")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmManualVerificationAction()
                    }
                    setOpenManualVerificationModal(false)
                }}
            >
                {t(`usersAndRolesScreen.voters.manualVerification.body`)}
            </Dialog>

            <ImportDataDrawer
                open={openImportDrawer}
                closeDrawer={() => setOpenImportDrawer(false)}
                title="electionEventScreen.import.title"
                subtitle="electionEventScreen.import.subtitle"
                paragraph="electionEventScreen.import.votersParagraph"
                doImport={handleImportVoters}
                errors={null}
            />

            <Dialog
                variant="warning"
                open={openDeleteBulkModal}
                ok={t("common.label.delete")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmDeleteBulkAction()
                    }
                    setOpenDeleteBulkModal(false)
                }}
            >
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.delete.bulkBody`)}
            </Dialog>

            <Dialog
                variant="info"
                open={openExport}
                ok={t("common.label.export")}
                okEnabled={() => !exporting}
                cancel={t("common.label.cancel")}
                title={t("common.label.export")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmExportAction()
                    } else {
                        setExportDocumentId(undefined)
                        setExporting(false)
                        setOpenExport(false)
                    }
                }}
            >
                {t("common.export")}
                <FormStyles.ReservedProgressSpace>
                    {exporting ? <FormStyles.ShowProgress /> : null}
                    {exporting && exportDocumentId ? (
                        <DownloadDocument
                            documentId={exportDocumentId}
                            electionEventId={electionEventId ?? ""}
                            fileName={`users-export.csv`}
                            onDownload={() => {
                                console.log("onDownload called")
                                setExportDocumentId(undefined)
                                setExporting(false)
                                setOpenExport(false)
                            }}
                        />
                    ) : null}
                </FormStyles.ReservedProgressSpace>
            </Dialog>
            {openEditPassword && (
                <EditPassword
                    open={openEditPassword}
                    handleClose={() => setOpenEditPassword(false)}
                    id={recordIds[0] as string}
                    electionEventId={electionEventId}
                />
            )}
        </>
    )
}

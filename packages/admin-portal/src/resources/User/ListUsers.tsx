// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext, useEffect, useMemo, useRef, useState} from "react"
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
    DateInput,
    useSidebarState,
    useUnselectAll,
    RaRecord,
    PreferencesEditorContext,
    useListContext,
} from "react-admin"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import UploadIcon from "@mui/icons-material/Upload"
import {ListActions} from "@/components/ListActions"
import {Box, Button, Chip, Menu, MenuItem, Skeleton, Stack, Typography} from "@mui/material"
import {Dialog, theme} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {Action} from "@/components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import MailIcon from "@mui/icons-material/Mail"
import CreditScoreIcon from "@mui/icons-material/CreditScore"
import PasswordIcon from "@mui/icons-material/Password"
import DeleteIcon from "@mui/icons-material/Delete"
import VisibilityIcon from "@mui/icons-material/Visibility"
import FilterAlt from "@mui/icons-material/FilterAlt"
import {EditUser} from "./EditUser"
import {AudienceSelection, SendTemplate} from "./SendTemplate"
import {CreateUser} from "./CreateUser"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    DeleteUserMutation,
    DeleteUsersMutation,
    ExportTenantUsersMutation,
    ExportUsersMutation,
    GetUserProfileAttributesQuery,
    ImportUsersMutation,
    ImportVotersDelegationMutation,
    ManualVerificationMutation,
    Sequent_Backend_Election_Event,
    UserProfileAttribute,
} from "@/gql/graphql"
import {DELETE_USER} from "@/queries/DeleteUser"
import {MANUAL_VERIFICATION} from "@/queries/ManualVerification"
import {useMutation, useQuery} from "@apollo/client"
import {IPermissions} from "@/types/keycloak"
import {ResourceListStyles} from "@/components/styles/ResourceListStyles"
import {IRole, IUser, translate} from "@sequentech/ui-core"
import {SettingsContext} from "@/providers/SettingsContextProvider"
import {ImportDataDrawer} from "@/components/election-event/import-data/ImportDataDrawer"
import {FormStyles} from "@/components/styles/FormStyles"
import {EXPORT_USERS} from "@/queries/ExportUsers"
import {EXPORT_TENANT_USERS} from "@/queries/ExportTenantUsers"
import {DownloadDocument} from "./DownloadDocument"
import {IMPORT_USERS} from "@/queries/ImportUsers"
import {ElectoralLogFilters, ElectoralLogList} from "@/components/ElectoralLogList"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {getAttributeLabel, getTranslationLabel, userBasicInfo} from "@/services/UserService"
import CustomDateField from "./CustomDateField"
import {ListActionsMenu} from "@/components/ListActionsMenu"
import EditPassword from "./EditPassword"
import {styled} from "@mui/material/styles"
import eStyled from "@emotion/styled"
import {DELETE_USERS} from "@/queries/DeleteUsers"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import SelectArea from "@/components/area/SelectArea"
import {WidgetProps} from "@/components/Widget"
import {ResetFilters} from "@/components/ResetFilters"
import {useElectionEventTallyStore} from "@/providers/ElectionEventTallyProvider"
import {UserActionTypes} from "@/components/types"
import {useUsersPermissions} from "./useUsersPermissions"
import {Check, FilterAltOff} from "@mui/icons-material"
import {useLocation} from "react-router"
import {getPreferenceKey} from "@/lib/helpers"
import {isEqual} from "lodash"
import {IMPORT_VOTERS_DELEGATION} from "@/queries/ImportVotersDelegation"

const DataGridContainerStyle = styled(DatagridConfigurable, {
    shouldForwardProp: (prop) => prop !== "isOpenSideBar", // Prevent `isOpenSideBar` from being passed to the DOM
})<{isOpenSideBar?: boolean}>`
    @media (min-width: ${({theme}) => theme.breakpoints.values.md}px) {
        overflowx: auto;
        width: 100%;
        ${({isOpenSideBar}) =>
            `maxWidth: ${isOpenSideBar ? "calc(100vw - 355px)" : "calc(100vw - 108px)"};`}
        &  > div:first-of-type {
            position: absolute;
            width: 100%;
        }
    }
`

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = eStyled.div`
    display: block;
    padding-left: 18px;
`

export interface ListUsersProps {
    aside?: ReactElement
    electionEventId?: string
    electionId?: string
}

export const ListUsers: React.FC<ListUsersProps> = ({aside, electionEventId, electionId}) => {
    const {t, i18n} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const [isOpenSidebar] = useSidebarState()
    const location = useLocation()

    const [open, setOpen] = useState(false)
    const [openExport, setOpenExport] = useState(false)
    const [exporting, setExporting] = useState(false)
    const [userType, setUserType] = useState<string | null>(null)
    const [exportDocumentId, setExportDocumentId] = useState<string | undefined>()
    const [openNew, setOpenNew] = useState(false)
    const [audienceSelection, setAudienceSelection] = useState<AudienceSelection>(
        AudienceSelection.SELECTED
    )
    const [documentId, setDocumentId] = useState<string | null>(null)
    const [addWidget, setWidgetTaskId, updateWidgetFail] = useWidgetStore()
    const [openUsersLogsModal, setOpenUsersLogsModal] = React.useState(false)
    const [openSendTemplate, setOpenSendTemplate] = React.useState(false)
    const [openDeleteModal, setOpenDeleteModal] = React.useState(false)
    const [openManualVerificationModal, setOpenManualVerificationModal] = React.useState(false)
    const [openDeleteBulkModal, setOpenDeleteBulkModal] = React.useState(false)
    const [openEditPassword, setOpenEditPassword] = React.useState(false)
    const [selectedIds, setSelectedIds] = useState<Identifier[]>([])
    const [deleteId, setDeleteId] = useState<string | undefined>()
    const [openDrawer, setOpenDrawer] = useState<boolean>(false)
    const [openImportDrawer, setOpenImportDrawer] = useState<boolean>(false)
    const [openImportDelegationsDrawer, setOpenImportDelegationsDrawer] = useState<boolean>(false)
    const [recordIds, setRecordIds] = useState<Array<Identifier>>([])
    const [userRecord, setUserRecord] = useState<RaRecord<Identifier> | undefined>()
    const authContext = useContext(AuthContext)
    const refresh = useRefresh()
    const unselectAll = useUnselectAll("user")
    const [deleteUser] = useMutation<DeleteUserMutation>(DELETE_USER)
    const [getManualVerificationPdf] = useMutation<ManualVerificationMutation>(MANUAL_VERIFICATION)
    const [deleteUsers] = useMutation<DeleteUsersMutation>(DELETE_USERS)
    const [exportUsers] = useMutation<ExportUsersMutation>(EXPORT_USERS)
    const PHONE_NUMBER_USER_ATTRIBUTE = "sequent.read-only.mobile-number"

    const {data: userAttributes} = useQuery<GetUserProfileAttributesQuery>(
        USER_PROFILE_ATTRIBUTES,
        {
            variables: {
                tenantId: tenantId,
                electionEventId: electionEventId,
            },
        }
    )

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
                            label={getTranslationLabel(attr.name, attr.display_name, t)}
                        />
                    )
                }
                return (
                    <TextInput
                        key={attr.name}
                        source={
                            userBasicInfo.includes(`${attr.name}`)
                                ? `${attr.name}.IsLike`
                                : `attributes.${source}`
                        }
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                    />
                )
            })
            filters.push(<BooleanInput key="enabled" source={"enabled"} />)
            filters.push(<BooleanInput key="email_verified" source={"email_verified"} />)
            if (electionEventId) {
                filters.push(
                    <SelectArea
                        tenantId={tenantId}
                        electionEventId={electionEventId}
                        source="attributes.area-id"
                        label={t("usersAndRolesScreen.users.fields.area")}
                    />
                )
            }
            if (electionEventId) {
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

    /**
     * Permissions
     */
    const {
        canImportUsers,
        canCreateVoters,
        canEditVoters,
        canEditVotersEmailTlf,
        canDeleteVoters,
        canImportVoters,
        canExportVoters,
        canManuallyVerify,
        canChangePassword,
        showVotersColumns,
        showVotersFilters,
        showVotersLogs,
        canSendTemplates,
        canImportVotersDelegations,
    } = useUsersPermissions()
    /**
     * Permissions
     */

    const handleClose = () => {
        setRecordIds([])
        setOpenUsersLogsModal(false)
        setOpenSendTemplate(false)
        setOpenDeleteModal(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenDrawer(false)
        setOpenNew(false)
        setOpen(false)
        unselectAll()
    }

    const editAction = (id: Identifier) => {
        setOpen(true)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenSendTemplate(false)
        setRecordIds([id as string])
    }

    const sendTemplateForIdAction = (id: Identifier) => {
        sendTemplateAction([id])
    }

    const sendTemplateAction = (
        ids: Array<Identifier>,
        audienceSelection = AudienceSelection.SELECTED
    ) => {
        setOpen(false)
        setOpenNew(false)
        setOpenDeleteModal(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenSendTemplate(true)
        setAudienceSelection(audienceSelection)
        setRecordIds(ids)
    }

    const deleteAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        setOpen(false)
        setOpenNew(false)
        setOpenSendTemplate(false)
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
        setOpenSendTemplate(false)
        setOpenManualVerificationModal(true)
        setOpenDeleteBulkModal(false)
        setOpenDeleteModal(false)
        setRecordIds([id])
    }

    const editPasswordAction = (id: Identifier) => {
        setOpen(false)
        setOpenNew(false)
        setOpenSendTemplate(false)
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
            setOpenManualVerificationModal(false)
            refresh()
            return
        }
        console.log(`fetchData success: ${data}`)
        setRecordIds([])
        refresh()

        let docId = data?.get_manual_verification_pdf?.document_id
        if (docId) {
            setDocumentId(docId)
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

    const showUsersLogsModal = (id: Identifier) => {
        if (!electionEventId) {
            return
        }
        setOpen(false)
        setOpenNew(false)
        setOpenSendTemplate(false)
        setOpenManualVerificationModal(false)
        setOpenDeleteBulkModal(false)
        setOpenDeleteModal(false)
        setOpenUsersLogsModal(true)
        setRecordIds([id])
    }

    const actionsConfig: Record<string, Action> = {
        [UserActionTypes.COMMUNICATION]: {
            icon: <MailIcon />,
            action: sendTemplateForIdAction,
            showAction: () => canSendTemplates,
            label: t(`sendCommunication.send`),
        },
        [UserActionTypes.EDIT]: {
            icon: <EditIcon className="edit-voter-icon" />,
            action: editAction,
            showAction: () => canEditVoters || canEditVotersEmailTlf,
            label: t(`common.label.edit`),
            saveRecordAction: setUserRecord,
        },
        [UserActionTypes.DELETE]: {
            icon: <DeleteIcon className="delete-voter-icon" />,
            action: deleteAction,
            showAction: () => canDeleteVoters,
            label: t(`common.label.delete`),
            className: "delete-voter-icon",
        },
        [UserActionTypes.MANUAL_VERIFICATION]: {
            icon: <CreditScoreIcon />,
            action: manualVerificationAction,
            showAction: () => canManuallyVerify,
            label: t(`usersAndRolesScreen.voters.manualVerification.label`),
            saveRecordAction: setUserRecord,
        },
        [UserActionTypes.PASSWORD]: {
            icon: <PasswordIcon />,
            action: editPasswordAction,
            showAction: () => canChangePassword,
            label: t(`usersAndRolesScreen.editPassword.label`),
        },
        [UserActionTypes.LOGS]: {
            icon: <VisibilityIcon />,
            action: showUsersLogsModal,
            showAction: () => showVotersLogs,
            label: t(`usersAndRolesScreen.voters.logs.label`),
        },
    }
    const actionByUserType: Record<string, any[]> = {
        user: [
            UserActionTypes.COMMUNICATION,
            UserActionTypes.EDIT,
            UserActionTypes.DELETE,
            UserActionTypes.PASSWORD,
        ],
        voter: [
            UserActionTypes.COMMUNICATION,
            UserActionTypes.EDIT,
            UserActionTypes.DELETE,
            UserActionTypes.MANUAL_VERIFICATION,
            UserActionTypes.PASSWORD,
            UserActionTypes.LOGS,
        ],
    }

    function getActionsForUserType(userType: string): Action[] {
        const actionsByUserType: string[] = actionByUserType[userType] || []
        const actions: Action[] = actionsByUserType.map((actionType) => actionsConfig[actionType])
        return actions
    }

    const actions = useMemo(() => {
        const userType = electionEventId ? "voter" : "user"
        return getActionsForUserType(userType)
    }, [electionEventId])

    async function confirmDeleteBulkAction() {
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
                {canSendTemplates && (
                    <Button
                        variant="actionbar"
                        key="send-notification"
                        onClick={() => {
                            sendTemplateAction(props.selectedIds ?? [], AudienceSelection.SELECTED)
                        }}
                    >
                        <ResourceListStyles.MailIcon />
                        {t(`sendCommunication.send`)}
                    </Button>
                )}

                {canDeleteVoters && (
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

    const handleClickImportDelegations = () => {
        setOpenImportDelegationsDrawer(true)
    }

    const handleExport = () => {
        setExporting(false)
        setExportDocumentId(undefined)
        setOpenExport(true)
    }

    const confirmExportAction = async () => {
        let currWidget: WidgetProps | undefined
        try {
            setExportDocumentId(undefined)
            setExporting(true)

            if (electionEventId) {
                currWidget = addWidget(ETasksExecution.EXPORT_VOTERS, undefined)
                const {data: exportUsersData, errors} = await exportUsers({
                    variables: {tenantId, electionEventId, electionId},
                })
                if (errors || !exportUsersData) {
                    setExporting(false)
                    setOpenExport(false)
                    updateWidgetFail(currWidget.identifier)
                    return
                }
                let documentId = exportUsersData.export_users?.document_id
                const task_id = exportUsersData?.export_users?.task_execution?.id
                setExportDocumentId(documentId)
                task_id
                    ? setWidgetTaskId(currWidget.identifier, task_id)
                    : updateWidgetFail(currWidget.identifier)
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
            currWidget && updateWidgetFail(currWidget.identifier)
        }
    }

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.emptyHeader`)}
            </Typography>
            {canCreateVoters ? (
                <>
                    <Typography variant="body1" paragraph>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.askCreate`)}
                    </Typography>
                    <ResourceListStyles.EmptyButtonList className="voter-add-button">
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
    const [importVotersDelegation] =
        useMutation<ImportVotersDelegationMutation>(IMPORT_VOTERS_DELEGATION)

    const enabledVotersDelegationsPolicy =
        electionEvent?.presentation?.delegations_policy === "enabled"

    /**
     * added custom filter actions menu
     */
    const buttonRef = useRef<HTMLButtonElement>(null)

    // state
    const [listActions, setListActions] = useState<ReactElement[]>(() => [
        ...(canSendTemplates
            ? [
                  <Button
                      key="send-notification"
                      onClick={() => sendTemplateAction([], AudienceSelection.ALL_USERS)}
                  >
                      <ResourceListStyles.MailIcon />
                      {t("sendCommunication.send")}
                  </Button>,
              ]
            : []),
        ...(canImportVotersDelegations && electionEventId && enabledVotersDelegationsPolicy
            ? [
                  <Button
                      key="import"
                      onClick={handleClickImportDelegations}
                      className="voter-import-button"
                  >
                      {/** TODO: add translations */}
                      Import Voters Delegations
                  </Button>,
              ]
            : []),
    ])
    const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null)
    const [openCustomMenu, setOpenCustomMenu] = useState(false)
    const {customFilter} = useElectionEventTallyStore()
    const [myFilters, setMyFilters] = useState({})
    const [hasCustomFilter, setHasCustomFilter] = useState<boolean>(false)
    const [selectedCustomItemMenu, setSelectedCustomItemMenu] = useState<number | null>(null)

    // functions
    const handleClickCustomMenu = (event: React.MouseEvent<HTMLElement>) => {
        setAnchorEl(event.currentTarget)
        setOpenCustomMenu(true)
    }

    const handleCloseCustomMenu = () => {
        setAnchorEl(null)
        setOpenCustomMenu(false)
    }

    const handleApplyCustomMenu = (filter: object | null | undefined, index: number | null) => {
        if (filter) {
            setMyFilters((prev: any) => ({...filter}))
            setHasCustomFilter(true)
        } else {
            setMyFilters({})
            setHasCustomFilter(false)
        }
        setSelectedCustomItemMenu(index)

        setAnchorEl(null)
        setOpenCustomMenu(false)
    }

    // effect

    useEffect(() => {
        setMyFilters(customFilter)
        setHasCustomFilter(false)
    }, [customFilter])

    const permanentFilters = {
        tenant_id: tenantId,
        election_event_id: electionEventId,
        election_id: electionId,
    }

    useEffect(() => {
        if (electionEvent) {
            let customFilters = electionEvent?.presentation?.custom_filters || []
            if (customFilters.length > 0) {
                customFilters = [...customFilters]
                setListActions((prev: ReactElement[]) => {
                    // prevent double adding the button
                    const customFilterExists = prev.some(
                        (action) => action.key === "custom-filters"
                    )
                    // if it's not in the list of actions, add it
                    if (!customFilterExists) {
                        return [
                            ...prev,
                            <Button
                                ref={buttonRef}
                                disableElevation
                                key="custom-filters"
                                aria-controls={openCustomMenu ? "basic-menu" : undefined}
                                aria-haspopup="true"
                                aria-expanded={openCustomMenu ? "true" : undefined}
                                variant="contained"
                                onClick={(e) => {
                                    handleClickCustomMenu(e)
                                    buttonRef.current?.blur()
                                }}
                                style={{
                                    backgroundColor: hasCustomFilter ? "#0F054C" : "#FFFFFF",
                                    color: hasCustomFilter ? "#FFFFFF" : "#0F054C",
                                }}
                            >
                                {hasCustomFilter ? (
                                    <FilterAlt sx={{mr: 1}} />
                                ) : (
                                    <FilterAltOff sx={{mr: 1}} />
                                )}
                                {t("common.label.filter")}
                            </Button>,
                        ]
                    }
                    return prev
                })
            }
        }
    }, [electionEvent])

    // Force update when hasCustomFilter changes
    useEffect(() => {
        setListActions((prev: ReactElement[]) => {
            return prev.map((action) => {
                if (action.key === "custom-filters") {
                    return (
                        <Button
                            ref={buttonRef}
                            disableElevation
                            key="custom-filters"
                            aria-controls={openCustomMenu ? "basic-menu" : undefined}
                            aria-haspopup="true"
                            aria-expanded={openCustomMenu ? "true" : undefined}
                            variant="contained"
                            onClick={(e) => {
                                handleClickCustomMenu(e)
                                buttonRef.current?.blur()
                            }}
                            style={{
                                backgroundColor: hasCustomFilter ? "#0F054C" : "#FFFFFF",
                                color: hasCustomFilter ? "#FFFFFF" : "#0F054C",
                            }}
                        >
                            {hasCustomFilter ? (
                                <FilterAlt sx={{mr: 1}} />
                            ) : (
                                <FilterAltOff sx={{mr: 1}} />
                            )}
                            {t("common.label.filter")}
                        </Button>
                    )
                }
                return action
            })
        })
    }, [hasCustomFilter])

    // Direct DOM manipulation for background color
    useEffect(() => {
        if (buttonRef.current) {
            buttonRef.current.style.backgroundColor = hasCustomFilter ? "#0F054C" : "#FFFFFF"
            buttonRef.current.style.color = hasCustomFilter ? "#FFFFFF" : "#0F054C"
        }
    }, [hasCustomFilter, theme.palette.primary])

    const resetMenuItem = () => {
        // renders the reset custom menu item
        return (
            <MenuItem key="reset-menu-item" onClick={() => handleApplyCustomMenu(null, null)}>
                <Stack direction="row" alignItems="center">
                    <span style={{width: "32px"}} />
                    <span>{t("electionEventScreen.common.reset")} </span>
                </Stack>
            </MenuItem>
        )
    }
    /**
     * END added custom filter actions menu
     */

    const renderMenuItems = () => {
        let customFiltersList = []
        let customFilters = electionEvent?.presentation?.custom_filters || []
        if (customFilters.length > 0) {
            // build the list of available filters
            customFiltersList = customFilters.map((item: any, index: number) => {
                const {label, filter} = item
                return (
                    <MenuItem
                        key={`custom-filter-${index}`}
                        onClick={() => handleApplyCustomMenu(filter, index + 1)}
                    >
                        <Stack direction="row" alignItems="center">
                            <span style={{width: "32px"}}>
                                {selectedCustomItemMenu && selectedCustomItemMenu === index + 1 ? (
                                    <Check sx={{mr: 1}} />
                                ) : null}
                            </span>
                            <span>
                                {translate(
                                    label.i18n,
                                    i18n.language.split("-")[0],
                                    i18n.language.split("-")[0]
                                ) || t(label.name)}
                            </span>
                        </Stack>
                    </MenuItem>
                )
            })
        }
        return [resetMenuItem(), ...customFiltersList]
    }

    const handleImportVoters = async (documentId: string, sha256: string) => {
        setOpenImportDrawer(false)
        const currWidget = addWidget(ETasksExecution.IMPORT_USERS, undefined)
        try {
            let {data, errors} = await importUsers({
                variables: {
                    tenantId,
                    documentId,
                    electionEventId: electionEventId || undefined,
                    sha256,
                },
            })
            const task_id = data?.import_users?.task_execution.id
            setWidgetTaskId(currWidget.identifier, task_id)

            refresh()

            if (errors) {
                updateWidgetFail(currWidget.identifier)
                notify(t("electionEventScreen.import.importVotersError"), {type: "error"})
            }
        } catch (err) {
            console.log(`Tenant ID: ${tenantId}`)
            console.log(`Document ID: ${documentId}`)
            console.log(`Election Event ID: ${electionEvent.id}`)
            console.log(``)
            console.log(`Error importing voters: ${err}`)
            updateWidgetFail(currWidget.identifier)
        }
    }

    const handleImportDelegations = async (documentId: string) => {
        setOpenImportDelegationsDrawer(false)
        if (electionEventId) {
            const currWidget = addWidget(ETasksExecution.IMPORT_VOTERS_DELEGATIONS)
            try {
                let {data, errors} = await importVotersDelegation({
                    variables: {
                        tenantId,
                        documentId,
                        electionEventId: electionEventId,
                    },
                })
                const task_id = data?.import_voters_delegation?.task_execution.id
                setWidgetTaskId(currWidget.identifier, task_id)

                refresh()

                if (errors) {
                    // TODO: Change text
                    updateWidgetFail(currWidget.identifier)
                    notify(t("electionEventScreen.import.importVotersError"), {type: "error"})
                }
            } catch (err) {
                updateWidgetFail(currWidget.identifier)
            }
        }
    }

    const listFields = useMemo(() => {
        const basicInfoFields: UserProfileAttribute[] = []
        const attributesFields: UserProfileAttribute[] = []
        const omitFields = ["id", "email_verified", "email"]

        userAttributes?.get_user_profile_attributes.forEach((attr) => {
            if (attr.name && userBasicInfo.includes(attr.name)) {
                basicInfoFields.push(attr)
            } else {
                omitFields.push(`attributes['${attr.name}']`)
                attributesFields.push(attr)
            }
        })
        return {basicInfoFields, attributesFields, omitFields}
    }, [userAttributes?.get_user_profile_attributes])

    const renderFields = (fields: UserProfileAttribute[]) => {
        const allFields = fields.map((attr) => {
            if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`attributes['${attr.name}']`}
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        render={(record: IUser, source: string | undefined) => {
                            return (
                                <CustomDateField
                                    key={attr.name}
                                    base="attributes"
                                    source={`${attr.name}`}
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
                        label={getTranslationLabel(attr.name, attr.display_name, t)}
                        render={(record: IUser, source: string | undefined) => {
                            let value: any =
                                attr.name && userBasicInfo.includes(attr.name)
                                    ? (record as any)[attr.name]
                                    : attr?.name
                                    ? (record as any).attributes[attr?.name]
                                    : "-"

                            return (
                                <>
                                    {value ? (
                                        value.map((item: any, index: number) => (
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
            return (
                <TextField
                    key={attr.name}
                    source={
                        attr.name && userBasicInfo.includes(attr.name)
                            ? attr.name
                            : `attributes['${attr.name}']`
                    }
                    label={getTranslationLabel(attr.name, attr.display_name, t)}
                    emptyText="-"
                />
            )
        })

        localStorage.removeItem(
            `RaStore.preferences.${getPreferenceKey(
                location.pathname,
                "voters"
            )}.datagrid.availableColumns`
        )

        return allFields
    }

    useEffect(() => {
        if (location.pathname.includes("user-roles")) {
            setUserType("user")
        } else {
            setUserType("voter")
        }
    }, [location.pathname])

    const checkUserEmailAndPhoneNumber = () => {
        if (userRecord) {
            const email = userRecord?.email as string
            const phoneNumber = userRecord?.attributes[PHONE_NUMBER_USER_ATTRIBUTE] as string
            if (email || phoneNumber) {
                return true
            }
        }
        return false
    }

    const checkIsVoted = (record: IUser) => {
        return record?.votes_info?.length
            ? !electionId || record.votes_info.some((vote) => vote.election_id === electionId)
            : false
    }

    const CustomListBody = () => {
        // `isLoading` => initial load, `isFetching` => subsequent loads (e.g. paging, filtering)
        const {isLoading, isFetching, perPage, filterValues, page, data} = useListContext()
        // Keep track of the "previous" page/filters in a ref and whether they changed
        const prevStateRef = useRef({perPage, page, filterValues})
        const [filtersChanged, setFiltersChanged] = useState(false)

        // 1. Compare current page and filters with previous
        //    Detect when filters/page change, and set filtersChanged=true
        useEffect(() => {
            const prev = prevStateRef.current
            const changed =
                prev.perPage !== perPage ||
                prev.page !== page ||
                !isEqual(prev.filterValues, filterValues)

            if (changed) {
                setFiltersChanged(true)
            }

            prevStateRef.current = {perPage, page, filterValues}
        }, [perPage, page, filterValues])

        // 2. Detect when the fetch is complete so we can reset filtersChanged if it was set
        useEffect(() => {
            if (!isFetching && filtersChanged) {
                setFiltersChanged(false)
            }
        }, [isFetching, filtersChanged])

        if (isLoading || (isFetching && filtersChanged)) {
            return <TableSkeleton rowCount={perPage} />
        }

        return (
            <>
                {userAttributes?.get_user_profile_attributes && (
                    <DataGridContainerStyle
                        preferenceKey={getPreferenceKey(location.pathname, "voters")}
                        omit={listFields.omitFields}
                        isOpenSideBar={isOpenSidebar}
                        bulkActionButtons={<BulkActions />}
                    >
                        <TextField source="id" sx={{display: "block", width: "280px"}} />
                        <BooleanField
                            source="email_verified"
                            label={t("usersAndRolesScreen.users.fields.emailVerified")}
                        />
                        <BooleanField
                            source="enabled"
                            label={t("usersAndRolesScreen.users.fields.enabled")}
                        />
                        {renderFields(listFields.basicInfoFields)}
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
                        {renderFields(listFields.attributesFields)}
                        {electionEventId && (
                            <FunctionField
                                source="has_voted"
                                label={t("usersAndRolesScreen.users.fields.has_voted")}
                                render={(record: IUser, source: string | undefined) => {
                                    let newRecord = {
                                        has_voted: checkIsVoted(record),
                                        ...record,
                                    }
                                    return <BooleanField record={newRecord} source={source} />
                                }}
                            />
                        )}
                        {!canEditVoters &&
                        !canDeleteVoters &&
                        !canSendTemplates &&
                        !canManuallyVerify &&
                        !canChangePassword &&
                        !showVotersLogs ? null : (
                            <WrapperField source="actions" label="Actions">
                                <ListActionsMenu actions={actions} />
                            </WrapperField>
                        )}
                    </DataGridContainerStyle>
                )}
                {/* Custom filters menu */}
                {showVotersFilters && (
                    <Menu
                        id="custom-filters-menu"
                        anchorEl={anchorEl}
                        open={openCustomMenu}
                        onClose={handleCloseCustomMenu}
                        MenuListProps={{
                            "aria-labelledby": "basic-button",
                        }}
                    >
                        {/* {customFiltersList} */}
                        {renderMenuItems()}
                    </Menu>
                )}
                {/* Custom filters menu */}
            </>
        )
    }

    return (
        <>
            {
                <List
                    resource="user"
                    storeKey={`${getPreferenceKey(location.pathname, "voters")}`}
                    queryOptions={{
                        refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                    }}
                    empty={hasCustomFilter ? false : <Empty />}
                    actions={
                        <ListActions
                            withColumns={showVotersColumns}
                            preferenceKey={getPreferenceKey(location.pathname, "voters")}
                            withFilter={showVotersFilters}
                            withImport={
                                userType
                                    ? userType === "voter"
                                        ? canImportVoters
                                        : canImportUsers
                                    : false
                            }
                            doImport={handleImport}
                            withExport={canExportVoters}
                            doExport={handleExport}
                            isExportDisabled={openExport}
                            open={openDrawer}
                            setOpen={setOpenDrawer}
                            Component={
                                <CreateUser
                                    electionEventId={electionEventId}
                                    close={handleClose}
                                    rolesList={rolesList || []}
                                    userAttributes={
                                        userAttributes?.get_user_profile_attributes || []
                                    }
                                />
                            }
                            withComponent={canCreateVoters}
                            extraActions={[...listActions]}
                        />
                    }
                    filter={{
                        ...myFilters,
                        ...permanentFilters,
                    }}
                    aside={aside}
                    filters={Filters}
                    disableSyncWithLocation
                >
                    <CustomListBody />
                </List>
            }

            <ResourceListStyles.Drawer anchor="right" open={open} onClose={handleClose}>
                <EditUser
                    id={recordIds[0] as string}
                    electionEventId={electionEventId}
                    electionId={electionId}
                    close={handleClose}
                    rolesList={rolesList || []}
                    userAttributes={userAttributes?.get_user_profile_attributes || []}
                    record={userRecord}
                />
            </ResourceListStyles.Drawer>
            <ResourceListStyles.Drawer anchor="right" open={openSendTemplate} onClose={handleClose}>
                <SendTemplate
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
                okEnabled={() => {
                    return checkUserEmailAndPhoneNumber() && !documentId
                }}
                ok={t("usersAndRolesScreen.voters.manualVerification.verify")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                errorMessage={
                    checkUserEmailAndPhoneNumber()
                        ? undefined
                        : t(`usersAndRolesScreen.voters.manualVerification.noEmailOrPhone`)
                }
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmManualVerificationAction()
                        return
                    }
                    setOpenManualVerificationModal(false)
                }}
            >
                {t(`usersAndRolesScreen.voters.manualVerification.body`)}
                <FormStyles.ReservedProgressSpace>
                    {documentId ? <FormStyles.ShowProgress /> : null}
                    {documentId ? (
                        <DownloadDocument
                            documentId={documentId}
                            electionEventId={electionEventId ?? ""}
                            fileName={`manual-verify-${electionEventId}-${documentId}.pdf`}
                            onDownload={() => {
                                console.log("onDownload called")
                                notify(
                                    t(
                                        "usersAndRolesScreen.voters.notifications.manualVerificationSuccess"
                                    ),
                                    {
                                        type: "success",
                                    }
                                )
                                setOpenManualVerificationModal(false)
                                setDocumentId(null)
                            }}
                        />
                    ) : null}
                </FormStyles.ReservedProgressSpace>
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
                    unselectAll()
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
            <Dialog
                fullWidth={true}
                variant="info"
                maxWidth={"xl"}
                title={t("usersAndRolesScreen.voters.logs.label")}
                ok={t("common.label.close")}
                open={openUsersLogsModal}
                handleClose={handleClose}
            >
                {/* The conditional below prevents re-rendering and data
                refetching when closing the dialog with no id */}
                {recordIds && recordIds.length > 0 && (
                    <ElectoralLogList
                        electionEventId={electionEventId}
                        showActions={false}
                        filterToShow={ElectoralLogFilters.USER_ID}
                        filterValue={recordIds[0]?.toString()}
                    />
                )}
            </Dialog>
            {openEditPassword && (
                <EditPassword
                    open={openEditPassword}
                    handleClose={() => setOpenEditPassword(false)}
                    id={recordIds[0] as string}
                    electionEventId={electionEventId}
                />
            )}
            {/** TODO: Change text */}
            <ImportDataDrawer
                open={openImportDelegationsDrawer}
                closeDrawer={() => setOpenImportDelegationsDrawer(false)}
                title="electionEventScreen.import.title"
                subtitle="electionEventScreen.import.subtitle"
                paragraph=""
                doImport={handleImportDelegations}
                errors={null}
                enableSha={false}
            />
        </>
    )
}

interface TableSkeletonProps {
    rowCount?: number
    columnWidths?: string[]
}

export const TableSkeleton: React.FC<TableSkeletonProps> = ({
    rowCount = 10,
    columnWidths = ["10%", "20%", "20%", "10%", "15%", "25%"],
}) => {
    return (
        <Box sx={{width: "100%", p: 2}}>
            {Array.from({length: rowCount}).map((_, rowIndex) => (
                <Box key={rowIndex} sx={{display: "flex", gap: 2, mb: 1, alignItems: "center"}}>
                    {columnWidths.map((width, colIndex) => (
                        <Skeleton
                            key={`${rowIndex}-${colIndex}`}
                            variant="text"
                            sx={{width, height: 24}}
                        />
                    ))}
                </Box>
            ))}
        </Box>
    )
}

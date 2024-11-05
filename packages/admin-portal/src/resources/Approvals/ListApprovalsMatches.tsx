// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
// SPDX-FileCopyrightText: 2023, 2024 Eduardo Robles <edu@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {ReactElement, useContext, useEffect, useMemo, useState} from "react"
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
    Datagrid,
} from "react-admin"
import {faPlus} from "@fortawesome/free-solid-svg-icons"
import {useTenantStore} from "@/providers/TenantContextProvider"
import UploadIcon from "@mui/icons-material/Upload"
import DescriptionIcon from "@mui/icons-material/Description"
import {ListActions} from "@/components/ListActions"
import {Button, Chip, Typography} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useTranslation} from "react-i18next"
import {Action, ActionsColumn} from "@/components/ActionButons"
import EditIcon from "@mui/icons-material/Edit"
import MailIcon from "@mui/icons-material/Mail"
import CreditScoreIcon from "@mui/icons-material/CreditScore"
import PasswordIcon from "@mui/icons-material/Password"
import DeleteIcon from "@mui/icons-material/Delete"
import VisibilityIcon from "@mui/icons-material/Visibility"
import {EditUser} from "../User/EditUser"
import {AudienceSelection, SendTemplate} from "../User/SendTemplate"
import {CreateUser} from "../User/CreateUser"
import {AuthContext} from "@/providers/AuthContextProvider"
import {
    ApplicationConfirmationBody,
    DeleteUserMutation,
    DeleteUsersMutation,
    ExportTenantUsersMutation,
    ExportUsersMutation,
    GetDocumentQuery,
    GetUserProfileAttributesQuery,
    ImportUsersMutation,
    ManualVerificationMutation,
    Sequent_Backend_Applications,
    Sequent_Backend_Election_Event,
    UserProfileAttribute,
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
import {DownloadDocument} from "../User/DownloadDocument"
import {IMPORT_USERS} from "@/queries/ImportUsers"
import {ElectoralLogFilters, ElectoralLogList} from "@/components/ElectoralLogList"
import {USER_PROFILE_ATTRIBUTES} from "@/queries/GetUserProfileAttributes"
import {getAttributeLabel, userBasicInfo} from "@/services/UserService"
import CustomDateField from "../User/CustomDateField"
import {ListActionsMenu} from "@/components/ListActionsMenu"
import EditPassword from "../User/EditPassword"
import {styled} from "@mui/material/styles"
import eStyled from "@emotion/styled"
import {DELETE_USERS} from "@/queries/DeleteUsers"
import {ETasksExecution} from "@/types/tasksExecution"
import {useWidgetStore} from "@/providers/WidgetsContextProvider"
import SelectArea from "@/components/area/SelectArea"
import {WidgetProps} from "@/components/Widget"
import {ResetFilters} from "@/components/ResetFilters"
import ElectionHeader from "@/components/ElectionHeader"
import { APPLICATION_CONFIRM } from "@/queries/ApplicationConfirm"
import { taskCancelled } from "@reduxjs/toolkit/dist/listenerMiddleware/exceptions"

const StyledChip = styled(Chip)`
    margin: 4px;
`

const StyledNull = eStyled.div`
    display: block;
    padding-left: 18px;
`

export interface ListUsersProps {
    electionEventId?: string
    electionId?: string
    task: Sequent_Backend_Applications 
}

export const ListApprovalsMatches: React.FC<ListUsersProps> = ({electionEventId, electionId, task}) => {
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()
    const {globalSettings} = useContext(SettingsContext)
    const notify = useNotify()

    const [openApproveModal, setOpenApproveModal] = React.useState(false)
    const [userId, setUserId] = useState<string | undefined>()
    const authContext = useContext(AuthContext)
    const refresh = useRefresh()

    const canEditUsers = authContext.isAuthorized(true, tenantId, IPermissions.VOTER_WRITE)
    const [approveVoter] = useMutation<ApplicationConfirmationBody>(APPLICATION_CONFIRM)

    // const userApprovalInfo = ["first_name", "last_name", "email", "username", "date_of_birth"]
    const userApprovalInfo = ["username"]

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
                console.log("aa Filters attr", attr)

                //covert to valid source string (if attr name is for example sequent.read-only.otp-method)
                const source = attr.name?.replaceAll(".", "%")
                if (attr.annotations?.inputType === "html5-date") {
                    return (
                        <DateInput
                            key={attr.name}
                            source={`attributes.${attr.name}`}
                            label={getAttributeLabel(attr.display_name ?? "")}
                        />
                    )
                }
                return (
                    <TextInput
                        key={attr.name}
                        source={
                            userApprovalInfo.includes(`${attr.name}`)
                                ? `${attr.name}`
                                : `attributes.${source}`
                        }
                        label={getAttributeLabel(attr.display_name ?? "")}
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

    const approveAction = (id: Identifier) => {
        if (!electionEventId && authContext.userId === id) {
            return
        }
        setOpenApproveModal(true)
        setUserId(id as string)
    }

    const confirmApproveAction = async () => {
        const {errors} = await approveVoter({
            variables: {
                tenant_id: tenantId,
                id: task?.id,
                user_id: userId,
                area_id: task?.area_id,
                election_event_id: electionEventId,
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
        setUserId(undefined)
        refresh()
    }

    const actions: Action[] = [
        {
            icon: <DescriptionIcon className="delete-voter-icon" />,
            action: approveAction,
            showAction: () => canEditUsers,
            label: t(`common.label.delete`),
            className: "delete-voter-icon",
        },
    ]

    const Empty = () => (
        <ResourceListStyles.EmptyBox>
            <Typography variant="h4" paragraph>
                {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.emptyHeader`)}
            </Typography>
        </ResourceListStyles.EmptyBox>
    )

    const listFields = useMemo(() => {
        const basicInfoFields: UserProfileAttribute[] = []
        const attributesFields: UserProfileAttribute[] = []
        const omitFields = ["id", "email_verified", "username"]

        console.log(
            "aa userAttributes?.get_user_profile_attributes :>> ",
            userAttributes?.get_user_profile_attributes
        )

        userAttributes?.get_user_profile_attributes.forEach((attr) => {
            if (attr.name && userApprovalInfo.includes(attr.name)) {
                basicInfoFields.push(attr)
            } else {
                omitFields.push(`attributes['${attr.name}']`)
                attributesFields.push(attr)
            }
        })

        return {basicInfoFields, attributesFields, omitFields}
    }, [userAttributes?.get_user_profile_attributes])

    const renderFields = (fields: UserProfileAttribute[]) =>
        fields.map((attr) => {
            if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FunctionField
                        key={attr.name}
                        source={`attributes['${attr.name}']`}
                        label={getAttributeLabel(attr.display_name ?? "")}
                        render={(record: IUser, source: string | undefined) => {
                            return (
                                <CustomDateField
                                    key={attr.name}
                                    source={`${attr.name}`}
                                    label={getAttributeLabel(attr.display_name ?? "")}
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
                        label={getAttributeLabel(attr.display_name ?? "")}
                        render={(record: IUser, source: string | undefined) => {
                            let value: any =
                                attr.name && userApprovalInfo.includes(attr.name)
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
                        attr.name && userApprovalInfo.includes(attr.name)
                            ? attr.name
                            : `attributes['${attr.name}']`
                    }
                    label={getAttributeLabel(attr.display_name ?? "")}
                    emptyText="-"
                />
            )
        })

    return (
        <>
            <ElectionHeader
                title={t("approvalsScreen.title")}
                subtitle="approvalsScreen.subtitle"
            />

            <List
                resource="user"
                queryOptions={{
                    refetchInterval: globalSettings.QUERY_POLL_INTERVAL_MS,
                }}
                empty={<Empty />}
                actions={<ListActions withImport={false} withExport={false} />}
                filter={{
                    tenant_id: tenantId,
                    election_event_id: electionEventId,
                    election_id: electionId,
                }}
                storeKey={false}
                filters={Filters}
                filterDefaultValues={{}}
            >
                <ResetFilters />
                {userAttributes?.get_user_profile_attributes && (
                    <DatagridConfigurable omit={listFields.omitFields} bulkActionButtons={false}>
                        <TextField source="id" sx={{display: "block", width: "280px"}} />
                        {/* <BooleanField source="email_verified" /> */}
                        <BooleanField source="enabled" />
                        {renderFields(listFields?.basicInfoFields)}
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
                        {/* {renderFields(listFields.attributesFields)} */}
                        {/* {electionEventId && (
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
                        )} */}
                        <ActionsColumn actions={actions} label={t("common.label.actions")} />
                    </DatagridConfigurable>
                )}
            </List>

            <Dialog
                variant="info"
                open={openApproveModal}
                ok={t("common.label.approve")}
                cancel={t("common.label.cancel")}
                title={t("common.label.warning")}
                handleClose={(result: boolean) => {
                    if (result) {
                        confirmApproveAction()
                    }
                    setOpenApproveModal(false)
                }}
            >
                {t(`approvalsScreen.approve.body`)}
            </Dialog>
        </>
    )
}

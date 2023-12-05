// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useState} from "react"
import {
    SaveButton,
    SimpleForm,
    useGetList,
    useListContext,
    useNotify,
    useRefresh,
} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IRole, IUser} from "sequent-core"
import {FormControl, MenuItem, Select, SelectChangeEvent, TextField} from "@mui/material"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {
    DeleteUserRoleMutation,
    EditUsersInput,
    ListUserRolesQuery,
    Sequent_Backend_Area,
    SetUserRoleMutation,
} from "@/gql/graphql"
import {EDIT_USER} from "@/queries/EditUser"
import {LIST_USER_ROLES} from "@/queries/ListUserRoles"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {isUndefined} from "@sequentech/ui-essentials"
import Checkbox from "@mui/material/Checkbox"
import {DELETE_USER_ROLE} from "@/queries/DeleteUserRole"
import {SET_USER_ROLE} from "@/queries/SetUserRole"

interface ListUserRolesProps {
    userId: string
    userRoles: ListUserRolesQuery
    rolesList: Array<IRole>
    refetch: () => void
}

const ListUserRoles: React.FC<ListUserRolesProps> = ({userRoles, rolesList, userId, refetch}) => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const [deleteUserRole] = useMutation<DeleteUserRoleMutation>(DELETE_USER_ROLE)
    const [setUserRole] = useMutation<SetUserRoleMutation>(SET_USER_ROLE)
    const refresh = useRefresh()
    const notify = useNotify()

    const activeRoleIds = userRoles.list_user_roles.map((role) => role.id || "")

    let rows: Array<IRole & {id: string; active: boolean}> = rolesList.map((role) => ({
        ...role,
        id: role.id || "",
        active: activeRoleIds.includes(role.id || ""),
    }))

    const editRolePermission = (props: GridRenderCellParams<any, boolean>) => async () => {
        const role = (rolesList || []).find((el) => el.id === props.row.id)
        if (!role?.name) {
            return
        }

        // remove/add permission to role
        const {errors} = await (props.value ? deleteUserRole : setUserRole)({
            variables: {
                tenantId: tenantId,
                roleId: role.id,
                userId: userId,
            },
        })
        if (errors) {
            notify(t(`usersAndRolesScreen.roles.notifications.permissionEditError`), {
                type: "error",
            })
            console.log(`Error editing permission: ${errors}`)
            return
        }
        notify(t(`usersAndRolesScreen.roles.notifications.permissionEditSuccess`), {
            type: "success",
        })
        refresh()
        refetch()
    }

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Role",
            width: 350,
            editable: false,
        },
        {
            field: "active",
            headerName: "Active",
            width: 70,
            editable: false,
            renderCell: (props: GridRenderCellParams<any, boolean>) => (
                <Checkbox checked={props.value} onClick={editRolePermission(props)} />
            ),
        },
    ]

    return (
        <>
            <DataGrid
                rows={rows}
                columns={columns}
                initialState={{
                    pagination: {
                        paginationModel: {
                            pageSize: 10,
                        },
                    },
                }}
                pageSizeOptions={[10, 20, 50, 100]}
            />
        </>
    )
}

interface EditUserFormProps {
    id?: string
    electionEventId?: string
    close?: () => void
    rolesList: Array<IRole>
}

export const EditUserForm: React.FC<EditUserFormProps> = ({
    id,
    close,
    electionEventId,
    rolesList,
}) => {
    const {data, isLoading} = useListContext<IUser & {id: string}>()
    let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
    const [user, setUser] = useState<IUser | undefined>(userOriginal)
    const notify = useNotify()
    const {t} = useTranslation()
    const [tenantId] = useTenantStore()

    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)
    const {data: userRoles, refetch} = useQuery<ListUserRolesQuery>(LIST_USER_ROLES, {
        variables: {
            tenantId: tenantId,
            electionEventId: electionEventId,
            userId: id || tenantId,
        },
    })

    const {data: areas} = useGetList<Sequent_Backend_Area>("sequent_backend_area", {
        pagination: {page: 1, perPage: 9999},
        filter: {election_event_id: electionEventId, tenant_id: tenantId},
    })

    useEffect(() => {
        if (!isLoading && data) {
            let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
            setUser(userOriginal)
        }
    }, [isLoading, data, id])

    const onSubmit = async () => {
        try {
            let {data, errors} = await edit_user({
                variables: {
                    body: {
                        user_id: user?.id,
                        tenant_id: tenantId,
                        election_event_id: electionEventId,
                        first_name: user?.first_name,
                        email: user?.email,
                        attributes: {
                            "area-id": [user?.attributes?.["area-id"]?.[0]],
                        },
                    },
                },
            })
            notify(t("usersAndRolesScreen.voters.errors.editSuccess"), {type: "success"})
            close?.()
        } catch (error) {
            notify(t("usersAndRolesScreen.voters.errors.editError"), {type: "error"})
            close?.()
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setUser({...user, [name]: value})
    }

    if (!user) {
        return null
    }

    let areaIdAttribute = user?.attributes?.["area-id"] as Array<string> | undefined
    let defaultAreaId = areaIdAttribute?.[0] ?? undefined

    const handleSelectArea = async (e: SelectChangeEvent) => {
        if (!electionEventId) {
            return
        }

        setUser({
            ...user,
            attributes: {
                ...user.attributes,
                "area-id": [e.target.value],
            },
        })
    }

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={<SaveButton alwaysEnable />}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <>
                    <PageHeaderStyles.Title>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.title`)}
                    </PageHeaderStyles.Title>
                    <PageHeaderStyles.SubTitle>
                        {t(`usersAndRolesScreen.${electionEventId ? "voters" : "users"}.subtitle`)}
                    </PageHeaderStyles.SubTitle>

                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.first_name")}
                        value={user.first_name || ""}
                        name={"first_name"}
                        onChange={handleChange}
                    />
                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.last_name")}
                        value={user.last_name || ""}
                        name={"last_name"}
                        onChange={handleChange}
                    />
                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.email")}
                        value={user.email || ""}
                        name={"email"}
                        onChange={handleChange}
                    />
                    <TextField
                        variant="outlined"
                        label={t("usersAndRolesScreen.users.fields.username")}
                        value={user.username || ""}
                        name={"username"}
                        onChange={handleChange}
                    />
                    {electionEventId ? (
                        <FormControl fullWidth>
                            <ElectionHeaderStyles.Title>
                                {t("usersAndRolesScreen.users.fields.area")}
                            </ElectionHeaderStyles.Title>

                            <Select
                                name="area"
                                defaultValue={defaultAreaId}
                                value={defaultAreaId}
                                onChange={handleSelectArea}
                            >
                                {areas?.map((area: Sequent_Backend_Area) => (
                                    <MenuItem key={area.id} value={area.id}>
                                        {area.name}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    ) : null}
                    {isUndefined(electionEventId) && !isUndefined(userRoles) && !isUndefined(id) ? (
                        <ListUserRoles
                            userRoles={userRoles}
                            rolesList={rolesList}
                            userId={id}
                            refetch={() => refetch()}
                        />
                    ) : null}
                </>
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}

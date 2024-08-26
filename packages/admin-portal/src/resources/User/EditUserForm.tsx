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
import {IRole, IUser} from "@sequentech/ui-core"
import {
    FormControl,
    MenuItem,
    Select,
    SelectChangeEvent,
    FormControlLabel,
    Checkbox,
} from "@mui/material"
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
import {isUndefined} from "@sequentech/ui-core"
import {DELETE_USER_ROLE} from "@/queries/DeleteUserRole"
import {SET_USER_ROLE} from "@/queries/SetUserRole"
import {FormStyles} from "@/components/styles/FormStyles"

interface ListUserRolesProps {
    userId: string
    userRoles: ListUserRolesQuery
    rolesList: Array<IRole>
    refetch: () => void
}

export const ListUserRoles: React.FC<ListUserRolesProps> = ({
    userRoles,
    rolesList,
    userId,
    refetch,
}) => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const [deleteUserRole] = useMutation<DeleteUserRoleMutation>(DELETE_USER_ROLE)
    const [setUserRole] = useMutation<SetUserRoleMutation>(SET_USER_ROLE)
    const refresh = useRefresh()
    const notify = useNotify()

    const activeRoleIds = userRoles?.list_user_roles.map((role) => role.id || "")

    let rows: Array<IRole & {id: string; active: boolean}> = rolesList.map((role) => ({
        ...role,
        id: role.id || "",
        active: activeRoleIds?.includes(role.id || "") || false,
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
        if (refetch) {
            refetch()
        }
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
    const refresh = useRefresh()

    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)
    const {data: userRoles, refetch} = useQuery<ListUserRolesQuery>(LIST_USER_ROLES, {
        variables: {
            tenantId: tenantId,
            userId: id!,
            electionEventId: electionEventId,
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
            let {data} = await edit_user({
                variables: {
                    body: {
                        user_id: user?.id,
                        tenant_id: tenantId,
                        election_event_id: electionEventId,
                        first_name: user?.first_name,
                        last_name: user?.last_name,
                        enabled: user?.enabled,
                        password:
                            user?.password && user?.password.length > 0 ? user.password : undefined,
                        email: user?.email,
                        attributes: {
                            "area-id": user?.attributes?.["area-id"],
                            "sequent.read-only.mobile-number":
                                user?.attributes?.["sequent.read-only.mobile-number"],
                        },
                    },
                },
            })
            notify(t("usersAndRolesScreen.voters.errors.editSuccess"), {type: "success"})
            refresh()
            close?.()
        } catch (error) {
            notify(t("usersAndRolesScreen.voters.errors.editError"), {type: "error"})
            close?.()
        }
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        let newUser = {...user, [name]: value}
        console.log(`newUser = `)
        console.log(newUser)
        setUser(newUser)
    }

    const handleAttrChange =
        (attrName: string) => async (e: React.ChangeEvent<HTMLInputElement>) => {
            const {value} = e.target
            let newUser = {
                ...user,
                attributes: {
                    ...(user?.attributes ?? {}),
                    [attrName]: [value],
                },
            }
            console.log(`newUser = `)
            console.log(newUser)
            setUser(newUser)
        }

    if (!user) {
        return null
    }

    let areaIdAttribute = user?.attributes?.["area-id"] as string | undefined
    let defaultAreaId = areaIdAttribute ?? undefined

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

    const validatePassword = (value: any) => {
        /*TODO: we should validate only to the extent that these policies are 
        in place in keycloak
        if (!value || value.length == 0) {
            return
        }

        const hasEnoughChars = value.length < 8
        const hasUpperCase = /[A-Z]/.test(value)
        const hasLowerCase = /[a-z]/.test(value)
        const hasDigit = /\d/.test(value)
        const hasSpecialChar = /[^a-zA-Z\d]/.test(value)

        if (hasEnoughChars) {
            return t("usersAndRolesScreen.users.fields.passwordLengthValidate")
        }

        if (!hasUpperCase) {
            return t("usersAndRolesScreen.users.fields.passwordUppercaseValidate")
        }

        if (!hasLowerCase) {
            return t("usersAndRolesScreen.users.fields.passwordLowercaseValidate")
        }

        if (!hasDigit) {
            return t("usersAndRolesScreen.users.fields.passwordDigitValidate")
        }

        if (!hasSpecialChar) {
            return t("usersAndRolesScreen.users.fields.passwordSpecialCharValidate")
        }*/
    }

    const equalToPassword = (value: any, allValues: any) => {
        if (!allValues.password || allValues.password.length == 0) {
            return
        }
        if (value !== allValues.password) {
            return t("usersAndRolesScreen.users.fields.passwordMismatch")
        }
    }

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={<SaveButton alwaysEnable />}
                record={user}
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

                    <FormStyles.TextInput
                        label={t("usersAndRolesScreen.users.fields.first_name")}
                        source="first_name"
                        onChange={handleChange}
                    />
                    <FormStyles.TextInput
                        label={t("usersAndRolesScreen.users.fields.last_name")}
                        source="last_name"
                        onChange={handleChange}
                    />
                    <FormStyles.TextInput
                        label={t("usersAndRolesScreen.users.fields.email")}
                        source="email"
                        onChange={handleChange}
                    />
                    <FormStyles.TextInput
                        label={t("usersAndRolesScreen.users.fields.username")}
                        source="username"
                        onChange={handleChange}
                    />
                    <FormStyles.TextField
                        label={t("usersAndRolesScreen.common.mobileNumber")}
                        value={user?.attributes?.["sequent.read-only.mobile-number"]}
                        onChange={handleAttrChange("sequent.read-only.mobile-number")}
                    />
                    <FormStyles.PasswordInput
                        label={t("usersAndRolesScreen.users.fields.password")}
                        source="password"
                        onChange={handleChange}
                    />
                    <FormStyles.PasswordInput
                        label={t("usersAndRolesScreen.users.fields.repeatPassword")}
                        source="repeat_password"
                        validate={equalToPassword}
                        onChange={handleChange}
                    />
                    <FormStyles.CheckboxControlLabel
                        label={t("usersAndRolesScreen.users.fields.enabled")}
                        control={
                            <Checkbox
                                checked={user.enabled || false}
                                onChange={(event: any) => {
                                    setUser({...user, enabled: event.target.checked})
                                }}
                            />
                        }
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

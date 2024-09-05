// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useMemo, useState} from "react"
import {
    DateTimeInput,
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
    InputLabel,
    FormGroup,
    FormLabel,
} from "@mui/material"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {
    CreateUserMutationVariables,
    DeleteUserRoleMutation,
    EditUsersInput,
    ListUserRolesQuery,
    Sequent_Backend_Area,
    SetUserRoleMutation,
    UserProfileAttribute,
} from "@/gql/graphql"
import {EDIT_USER} from "@/queries/EditUser"
import {LIST_USER_ROLES} from "@/queries/ListUserRoles"
import {DataGrid, GridColDef, GridRenderCellParams} from "@mui/x-data-grid"
import {isUndefined} from "@sequentech/ui-core"
import {DELETE_USER_ROLE} from "@/queries/DeleteUserRole"
import {SET_USER_ROLE} from "@/queries/SetUserRole"
import {FormStyles} from "@/components/styles/FormStyles"
import {CREATE_USER} from "@/queries/CreateUser"
import {formatUserAtributes, getAttributeLabel, userBasicInfo} from "@/services/UserService"

interface ListUserRolesProps {
    userId?: string
    userRoles?: ListUserRolesQuery
    rolesList: Array<IRole>
    refetch: () => void
    createMode?: boolean
    setUserRoles?: (id: string) => void
    selectedRolesOnCreate?: string[]
}

export const ListUserRoles: React.FC<ListUserRolesProps> = ({
    userRoles,
    rolesList,
    userId,
    refetch,
    createMode,
    setUserRoles,
    selectedRolesOnCreate,
}) => {
    const [tenantId] = useTenantStore()
    const {t} = useTranslation()
    const [deleteUserRole] = useMutation<DeleteUserRoleMutation>(DELETE_USER_ROLE)
    const [setUserRole] = useMutation<SetUserRoleMutation>(SET_USER_ROLE)
    const refresh = useRefresh()
    const notify = useNotify()

    const activeRoleIds = createMode
        ? selectedRolesOnCreate
        : userRoles?.list_user_roles.map((role) => role.id || "")

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
        if (createMode && setUserRoles && role.id) {
            setUserRoles(role.id)
        }

        // remove/add permission to role
        if (!createMode && userId) {
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
    }

    const columns: GridColDef[] = [
        {
            field: "name",
            headerName: "Role",
            width: 250,
            editable: false,
        },
        {
            field: "active",
            headerName: "Active",
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
                style={{width: "100%"}}
            />
        </>
    )
}

interface EditUserFormProps {
    id?: string
    electionEventId?: string
    close?: () => void
    rolesList: Array<IRole>
    userAttributes: UserProfileAttribute[]
    createMode?: boolean
}

export const EditUserForm: React.FC<EditUserFormProps> = ({
    id,
    close,
    electionEventId,
    rolesList,
    userAttributes,
    createMode = false,
}) => {
    const {t} = useTranslation()
    const {data, isLoading} = useListContext<IUser & {id: string}>()
    let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
    const [user, setUser] = useState<IUser | undefined>(createMode ? {enabled: true} : userOriginal)
    const [selectedRolesOnCreate, setSelectedRolesOnCreate] = useState<string[]>([])
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()
    const [createUser] = useMutation<CreateUserMutationVariables>(CREATE_USER)
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
        if (!createMode && !isLoading && data) {
            let userOriginal: IUser | undefined = data?.find((element) => element.id === id)
            setUser(userOriginal)
        }
    }, [isLoading, data, id])

    const handleSelectedRolesOnCreate = useCallback(
        (id: string) => {
            const existId = selectedRolesOnCreate.find((roleId) => id === roleId)
            if (existId) {
                setSelectedRolesOnCreate((prev) => prev.filter((roleId) => id !== roleId))
            } else {
                setSelectedRolesOnCreate((prev) => [...prev, id])
            }
        },
        [setSelectedRolesOnCreate, selectedRolesOnCreate]
    )

    const onSubmitCreateUser = async () => {
        try {
            let {errors} = await createUser({
                variables: {
                    tenantId,
                    electionEventId,
                    user: {
                        id: user?.id,
                        first_name: user?.first_name,
                        last_name: user?.last_name,
                        enabled: user?.enabled,
                        email: user?.email,
                        username: user?.username,
                        attributes: formatUserAtributes(user?.attributes),
                    },
                    userRolesIds: selectedRolesOnCreate,
                },
            })
            close?.()
            if (errors) {
                notify(t("usersAndRolesScreen.voters.errors.createError"), {type: "error"})
                console.log(`Error creating user: ${errors}`)
            } else {
                notify(t("usersAndRolesScreen.voters.errors.createSuccess"), {type: "success"})
                refresh()
            }
        } catch (error) {
            close?.()
            notify(t("usersAndRolesScreen.voters.errors.createError"), {type: "error"})
            console.log(`Error creating user: ${error}`)
        }
    }

    const onSubmit = async () => {
        if (createMode) {
            onSubmitCreateUser()
        } else {
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
                                user?.password && user?.password.length > 0
                                    ? user.password
                                    : undefined,
                            email: user?.email,
                            attributes: formatUserAtributes(user?.attributes),
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
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        let newUser = {...user, [name]: value}
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
            setUser(newUser)
        }

    if (!user && !createMode) {
        return null
    }

    let areaIdAttribute = user?.attributes?.["area-id"] as string | undefined
    let defaultAreaId = areaIdAttribute ?? undefined

    const handleSelectChange = (attrName: string) => async (e: SelectChangeEvent) => {
        setUser((prev) => {
            return {
                ...prev,
                attributes: {
                    ...prev?.attributes,
                    [attrName]: [e.target.value],
                },
            }
        })
    }

    const handleCheckboxChange = (attrName: string) => (choiseId: string) => {
        let checkedItems = [choiseId]
        if (user && user?.attributes && user?.attributes[attrName]) {
            const currentChecked = user.attributes[attrName]
            checkedItems = currentChecked.includes(choiseId)
                ? currentChecked.filter((ab: any) => ab !== choiseId)
                : [...currentChecked, choiseId]
        }
        setUser((prev) => {
            return {
                ...prev,
                attributes: {
                    ...prev?.attributes,
                    [attrName]: checkedItems,
                },
            }
        })
    }

    //TODO: move when handle paswword
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

    // const equalToPassword = (value: any, allValues: any) => {
    //     if (!allValues.password || allValues.password.length == 0) {
    //         return
    //     }
    //     if (value !== allValues.password) {
    //         return t("usersAndRolesScreen.users.fields.passwordMismatch")
    //     }
    // }

    const renderFormField = (attr: UserProfileAttribute) => {
        if (attr.name) {
            const isCustomAttribute = !userBasicInfo.includes(attr.name)
            const value = isCustomAttribute
                ? user?.attributes?.[attr.name]
                : user && user[attr.name as keyof IUser]
            const displayName = attr.display_name ?? ""
            if (attr.annotations?.inputType === "select") {
                return (
                    <FormControl fullWidth>
                        <InputLabel id="demo-simple-select-label">{attr.display_name}</InputLabel>
                        <Select
                            name={displayName}
                            defaultValue={value}
                            labelId="demo-simple-select-label"
                            label={attr.display_name}
                            value={value}
                            onChange={handleSelectChange(attr.name)}
                        >
                            {attr.validations.options.options?.map((area: string) => (
                                <MenuItem key={area} value={area}>
                                    {area}
                                </MenuItem>
                            ))}
                        </Select>
                    </FormControl>
                )
            } else if (
                attr.annotations?.inputType === "multiselect-checkboxes" &&
                attr.annotations?.inputOptionLabels
            ) {
                const choices = Object.entries(attr.annotations?.inputOptionLabels).map(
                    ([key, value]) => {
                        return {id: key, name: getAttributeLabel(value as string)}
                    }
                )
                return (
                    <FormControl component="fieldset">
                        <FormLabel component="legend" style={{margin: 0}}>
                            {getAttributeLabel(displayName)}
                        </FormLabel>
                        <FormGroup row>
                            {choices.map((choice) => {
                                return (
                                    <FormControlLabel
                                        key={choice.id}
                                        control={
                                            <Checkbox
                                                checked={value && value.includes(choice.id)}
                                                onChange={() =>
                                                    handleCheckboxChange(attr.name ?? "")(choice.id)
                                                }
                                            />
                                        }
                                        label={choice.name}
                                    />
                                )
                            })}
                        </FormGroup>
                    </FormControl>
                )
            } else if (attr.annotations?.inputType === "html5-date") {
                return (
                    <FormStyles.DateInput
                        source={`attributes.${attr.name}`}
                        onChange={handleAttrChange(attr.name)}
                        label={attr.name}
                    />
                )
            } else if (attr.name.toLowerCase().includes("area")) {
                return
            }
            return (
                <>
                    {isCustomAttribute ? (
                        <FormStyles.TextField
                            label={attr.display_name}
                            value={value}
                            onChange={handleAttrChange(attr.name)}
                        />
                    ) : (
                        <FormStyles.TextInput
                            key={attr.display_name}
                            label={getAttributeLabel(attr.display_name ?? "")}
                            onChange={handleChange}
                            source={attr.name}
                        />
                    )}
                </>
            )
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
                    {userAttributes?.map((attr) => attr.name && renderFormField(attr))}
                    <FormStyles.CheckboxControlLabel
                        label={t("usersAndRolesScreen.users.fields.enabled")}
                        control={
                            <Checkbox
                                checked={user?.enabled || false}
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
                                onChange={handleSelectChange("area-id")}
                            >
                                {areas?.map((area: Sequent_Backend_Area) => (
                                    <MenuItem key={area.id} value={area.id}>
                                        {area.name}
                                    </MenuItem>
                                ))}
                            </Select>
                        </FormControl>
                    ) : null}
                    {isUndefined(electionEventId) ? (
                        <ListUserRoles
                            userRoles={userRoles}
                            rolesList={rolesList}
                            userId={id}
                            refetch={() => refetch()}
                            createMode={createMode}
                            setUserRoles={createMode ? handleSelectedRolesOnCreate : undefined}
                            selectedRolesOnCreate={selectedRolesOnCreate}
                        />
                    ) : null}
                </>
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}

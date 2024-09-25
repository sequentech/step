// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useEffect, useMemo, useState} from "react"
import {SaveButton, SimpleForm, useListContext, useNotify, useRefresh} from "react-admin"
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
import PhoneInput from "@/components/PhoneInput"
import SelectArea from "@/components/area/SelectArea"
import SelectActedTrustee from "./SelectActedTrustee"
import {GET_TRUSTEES_NAMES} from "@/queries/GetTrusteesNames"

interface ListUserRolesProps {
    userId?: string
    userRoles?: ListUserRolesQuery
    rolesList: Array<IRole>
    refetch: () => void
    createMode?: boolean
    setUserRoles?: (id: string) => void
    selectedRolesOnCreate?: string[]
}

export interface Trustee {
    id: string
    name: string
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
    const [selectedArea, setSelectedArea] = useState<string>("")
    const [selectedActedTrustee, setSelectedActedTrustee] = useState<string>("")
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
                        attributes: {
                            ...formatUserAtributes(user?.attributes),
                            ...(selectedArea && {"area-id": [selectedArea]}),
                            ...(selectedActedTrustee && {trustee: [selectedActedTrustee]}),
                        },
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
                await edit_user({
                    variables: {
                        body: {
                            user_id: user?.id,
                            tenant_id: tenantId,
                            election_event_id: electionEventId,
                            first_name: user?.first_name,
                            last_name: user?.last_name,
                            enabled: user?.enabled,
                            email: user?.email,
                            attributes: {
                                ...formatUserAtributes(user?.attributes),
                                ...(selectedArea && {"area-id": [selectedArea]}),
                                ...(selectedActedTrustee && {trustee: [selectedActedTrustee]}),
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
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target
        setUser((prev) => {
            return {
                ...prev,
                [name]: value,
            }
        })
    }

    const handleAttrChange =
        (attrName: string) => async (e: React.ChangeEvent<HTMLInputElement>) => {
            const {value} = e.target
            setUser((prev) => {
                return {
                    ...prev,
                    attributes: {
                        ...(prev?.attributes ?? {}),
                        [attrName]: [value],
                    },
                }
            })
        }

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

    const handleAttrStringValueChange = (attrName: string) => async (value: string) => {
        setUser((prev) => {
            return {
                ...prev,
                attributes: {
                    ...prev?.attributes,
                    [attrName]: [value],
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
                        <InputLabel id="select-label">{getAttributeLabel(displayName)}</InputLabel>
                        <Select
                            name={displayName}
                            defaultValue={value}
                            labelId="select-label"
                            label={getAttributeLabel(displayName)}
                            value={value}
                            onChange={handleSelectChange(attr.name)}
                        >
                            {attr.validations.options?.options?.map((area: string) => (
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
                        label={getAttributeLabel(displayName)}
                    />
                )
            } else if (attr.name.toLowerCase().includes("area")) {
                return
            } else if (attr.name.toLowerCase().includes("mobile-number")) {
                return (
                    <PhoneInput
                        handlePhoneNumberChange={handleAttrStringValueChange(attr.name ?? "")}
                        label={getAttributeLabel(displayName)}
                        fullWidth
                    />
                )
            } else if (attr.name.toLowerCase().includes("trustee")) {
                return (
                    <FormControl fullWidth>
                        <SelectActedTrustee
                            label={t("usersAndRolesScreen.users.fields.trustee")}
                            source={createMode ? "attributes.trustee" : "trustee"}
                            defaultValue={value}
                            tenantId={tenantId}
                            onSelectTrustee={(trustee: string) => {
                                setSelectedActedTrustee(trustee)
                            }}
                        />
                    </FormControl>
                )
            }
            return (
                <>
                    {isCustomAttribute ? (
                        <FormStyles.TextField
                            label={getAttributeLabel(displayName)}
                            value={value}
                            onChange={handleAttrChange(attr.name)}
                        />
                    ) : (
                        <FormStyles.TextInput
                            key={attr.display_name}
                            label={getAttributeLabel(displayName)}
                            onChange={handleChange}
                            source={attr.name}
                            disabled={attr.name === "username" && !createMode}
                        />
                    )}
                </>
            )
        }
    }

    const formFields = useMemo(() => {
        return userAttributes?.map((attr) => renderFormField(attr))
    }, [userAttributes])

    if (!user && !createMode) {
        return null
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
                    {formFields}
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
                    {electionEventId && (
                        <FormControl fullWidth>
                            <ElectionHeaderStyles.Title>
                                {t("usersAndRolesScreen.users.fields.area")}
                            </ElectionHeaderStyles.Title>
                            <SelectArea
                                tenantId={tenantId}
                                electionEventId={electionEventId}
                                source={createMode ? "attributes.area-id" : "area.id"}
                                onSelectArea={setSelectedArea}
                                label=""
                                customStyle={{
                                    "& legend": {
                                        display: "none",
                                    },
                                }}
                            />
                        </FormControl>
                    )}
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

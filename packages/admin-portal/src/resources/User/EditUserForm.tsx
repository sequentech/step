// SPDX-FileCopyrightText: 2023 Félix Robles <felix@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useContext, useEffect, useMemo, useRef, useState} from "react"
import {
    Identifier,
    RaRecord,
    SaveButton,
    SimpleForm,
    useNotify,
    useRefresh,
    AutocompleteArrayInput,
    BooleanInput,
    useGetList,
} from "react-admin"
import {useMutation, useQuery} from "@apollo/client"
import {PageHeaderStyles} from "../../components/styles/PageHeaderStyles"
import {useTranslation} from "react-i18next"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IRole, IUser} from "@sequentech/ui-core"
import {
    FormControl,
    FormControlLabel,
    Checkbox,
    FormGroup,
    FormLabel,
    Box,
    Autocomplete,
    Grid,
    TextField,
} from "@mui/material"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {
    CreateUserMutation,
    DeleteUserRoleMutation,
    EditUsersInput,
    ListUserRolesQuery,
    Sequent_Backend_Cast_Vote,
    Sequent_Backend_Election,
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
import {
    formatUserAtributes,
    getAttributeLabel,
    getTranslationLabel,
    userBasicInfo,
} from "@/services/UserService"
import PhoneInput from "@/components/PhoneInput"
import SelectArea from "@/components/area/SelectArea"
import SelectActedTrustee from "./SelectActedTrustee"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {InputContainerStyle, InputLabelStyle, PasswordInputStyle} from "./EditPassword"
import IconTooltip from "@/components/IconTooltip"
import {faInfoCircle} from "@fortawesome/free-solid-svg-icons"
import {useUsersPermissions} from "./useUsersPermissions"
import debounce from "lodash/debounce"
import type {ChangeEvent} from "react"

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

const convertRecordToUser = (record: RaRecord<Identifier>): IUser => {
    const user: IUser = {
        id: record.id ? String(record.id) : undefined,
        attributes: record.attributes || {},
        email: record.email,
        email_verified: record.email_verified,
        enabled: record.enabled,
        first_name: record.first_name,
        last_name: record.last_name,
        username: record.username,
        area: record.area,
        votes_info: record.votes_info || [],
    }
    return user
}

interface EditUserFormProps {
    id?: string
    electionEventId?: string
    electionId?: string
    close?: () => void
    rolesList: Array<IRole>
    userAttributes: UserProfileAttribute[]
    createMode?: boolean
    record?: RaRecord<Identifier>
}

export const EditUserForm: React.FC<EditUserFormProps> = ({
    id,
    close,
    electionEventId,
    electionId,
    rolesList,
    userAttributes,
    createMode = false,
    record,
}) => {
    const {t} = useTranslation()

    const [user, setUser] = useState<IUser | undefined>(
        createMode
            ? {
                  enabled: true,
                  attributes: {}, // Initialize attributes object for new users
              }
            : (record && convertRecordToUser(record)) || {attributes: {}}
    )

    const [selectedArea, setSelectedArea] = useState<string>("")
    const [selectedActedTrustee, setSelectedActedTrustee] = useState<string>("")
    const [selectedRolesOnCreate, setSelectedRolesOnCreate] = useState<string[]>([])
    const [phoneInputs, setPhoneInputs] = useState<{[key: string]: string[]}>({})
    const {canEditVoters, canEditVotersWhoVoted, canEditVotersEmailTlf} = useUsersPermissions()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()
    const authContext = useContext(AuthContext)
    const [createUser] = useMutation<CreateUserMutation>(CREATE_USER)
    const [edit_user] = useMutation<EditUsersInput>(EDIT_USER)
    const [permissionLabels, setPermissionLabels] = useState<string[]>(
        (user?.attributes?.permission_labels as string[]) || []
    )
    const [temporary, setTemportay] = useState<boolean>(true)
    const [choices, setChoices] = useState<any[]>(
        (user?.attributes?.permission_labels as string[])?.map((label) => ({
            id: label,
            name: label,
        })) || []
    )
    const [errorText, setErrorText] = useState("")

    const equalToPassword = (allValues: any) => {
        if (!allValues.password || allValues.password.length == 0) {
            return
        }
        if (allValues.confirm_password !== allValues.password) {
            setErrorText(t("usersAndRolesScreen.users.fields.passwordMismatch"))
        }

        if (errorText && allValues.confirm_password === allValues.password) {
            setErrorText("")
        }
    }

    useEffect(() => {
        const userPermissionLabels = user?.attributes?.permission_labels as string[] | undefined
        if (userPermissionLabels?.length) {
            setPermissionLabels([...userPermissionLabels])
            const transformedChoices = userPermissionLabels?.map((label) => ({
                id: label,
                name: label,
            }))
            setChoices([...transformedChoices])
        }
    }, [user])

    const {data: userRoles, refetch} = useQuery<ListUserRolesQuery>(LIST_USER_ROLES, {
        variables: {
            tenantId: tenantId,
            userId: id!,
            electionEventId: electionEventId,
        },
        skip: !id || !tenantId,
    })

    const {data: voterCastVotes} = useGetList<Sequent_Backend_Cast_Vote>(
        "sequent_backend_cast_vote",
        {
            pagination: {page: 1, perPage: 10},
            sort: {field: "last_updated_at", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
                voter_id_string: id,
            },
        },
        {
            enabled: !!electionEventId,
        }
    )

    const {data: electionsList} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 300},
            sort: {field: "name", order: "DESC"},
            filter: {
                tenant_id: tenantId,
                election_event_id: electionEventId,
            },
        },
        {
            enabled: !!electionEventId,
        }
    )

    // true if not affected by canEditVotersWhoVoted
    // which happens if the voter has not voted
    // or if current admin user has the permission canEditVotersWhoVoted

    const hasVoted = useMemo(() => {
        return voterCastVotes ? voterCastVotes?.length > 0 : false
    }, [voterCastVotes])

    const enabledByVoteNum = useMemo(() => {
        return canEditVotersWhoVoted || (canEditVoters && !hasVoted)
    }, [canEditVotersWhoVoted, hasVoted])

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
            let {errors, data} = await createUser({
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
                            ...(phoneInputs && phoneInputs),
                            ...(selectedActedTrustee && {trustee: [selectedActedTrustee]}),
                        },
                    },
                    userRolesIds: selectedRolesOnCreate,
                },
            })
            //update user password after creating user
            //@ts-ignore because data returns create_user property but not recognized
            close?.()
            if (errors) {
                notify(t("usersAndRolesScreen.voters.errors.createError"), {type: "error"})
                console.log(`Error creating user: ${errors}`)
            } else {
                if ((user?.password?.length ?? 0) > 0 && data?.create_user.id) {
                    await handleUpdateUserPassword(data?.create_user.id)
                }
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
                await handleEditUser()
                if (authContext.userId === user?.id) {
                    authContext.updateTokenAndPermissionLabels()
                }
                notify(t("usersAndRolesScreen.voters.errors.editSuccess"), {type: "success"})
                refresh()
                close?.()
            } catch (error) {
                notify(t("usersAndRolesScreen.voters.errors.editError"), {type: "error"})
                close?.()
            }
        }
    }

    const handleUpdateUserPassword = async (id: string) => {
        return edit_user({
            variables: {
                body: {
                    user_id: id,
                    tenant_id: tenantId,
                    election_event_id: electionEventId,
                    password:
                        user?.password && user?.password.length > 0 ? user.password : undefined,
                    temporary: temporary,
                },
            },
        })
    }

    const handleEditUser = async () => {
        return edit_user({
            variables: {
                body: {
                    user_id: user?.id,
                    tenant_id: tenantId,
                    election_event_id: electionEventId,
                    first_name: user?.first_name,
                    last_name: user?.last_name,
                    enabled: user?.enabled,
                    email: user?.email,
                    password:
                        user?.password && user?.password.length > 0 ? user.password : undefined,
                    temporary: temporary,
                    attributes: {
                        ...formatUserAtributes(user?.attributes),
                        ...(selectedArea && {"area-id": [selectedArea]}),
                        ...(phoneInputs && phoneInputs),
                        ...(selectedActedTrustee && {trustee: [selectedActedTrustee]}),
                    },
                },
            },
        })
    }

    const handleChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
        const {name, value} = e.target

        const updatedUser = {
            ...user,
            [name]: value,
        }

        //only run on password update
        if (name === "confirm_password" || name === "password") {
            equalToPassword(updatedUser)
        }

        setUser(updatedUser)
    }

    const handleAttrChange =
        (attrName: string) => async (e: React.ChangeEvent<HTMLInputElement>) => {
            const {name, value} = e.target
            debouncedHandleChange(name, value)
        }

    const debouncedHandleChange = useCallback(
        debounce((name: string, value: string) => {
            setUser((prev) => {
                return {
                    ...prev,
                    attributes: {
                        ...(prev?.attributes ?? {}),
                        [name]: [value],
                    },
                }
            })
        }, 300),
        [user, equalToPassword]
    )

    const handleDateChange =
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

    const handleSelectChange = (attrName: string) => async (e: string) => {
        setUser((prev) => {
            return {
                ...prev,
                attributes: {
                    ...prev?.attributes,
                    [attrName]: [e],
                },
            }
        })
    }

    const handleArraySelectChange = (value: string[]) => {
        setUser((prev) => {
            return {
                ...prev,
                attributes: {
                    ...prev?.attributes,
                    "authorized-election-ids": value,
                },
            }
        })
    }

    const handlePermissionLabelRemoved = (value: string[]) => {
        if (value?.length < permissionLabels?.length) {
            setUser((prev) => {
                return {
                    ...prev,
                    attributes: {
                        ...prev?.attributes,
                        permission_labels: value,
                    },
                }
            })
        }
    }

    const handlePermissionLabelAdded = (value: string[]) => {
        setUser((prev) => {
            return {
                ...prev,
                attributes: {
                    ...prev?.attributes,
                    permission_labels: value,
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

    const handlePhoneNumberChange = (attrName: string) => async (number: string) => {
        const phoneInput = phoneInputs[attrName]
        if (!phoneInput || (phoneInput && phoneInput[0] !== number)) {
            setPhoneInputs((prev) => {
                return {
                    ...prev,
                    [attrName]: [number],
                }
            })
        }
    }

    const aliasRenderer = useAliasRenderer()

    const searched = useRef("")

    const electionFilterToQuery = (searchText: string) => {
        if (searchText && searchText.length > 0) {
            searched.current = searchText.trim()
        }
        return {"name@_ilike,alias@_ilike": searched.current}
    }

    const renderFormField = useCallback(
        (attr: UserProfileAttribute) => {
            if (attr.name) {
                const isCustomAttribute = !userBasicInfo.includes(attr.name)
                const value = isCustomAttribute
                    ? user?.attributes?.[attr.name]
                    : user && user[attr.name as keyof IUser]
                const displayName = attr.display_name ?? ""
                const isRequired = isFieldRequired(attr)
                if (attr.annotations?.inputType === "select") {
                    return (
                        <Grid container spacing={2}>
                            <Grid item xs={12}>
                                <FormControl fullWidth>
                                    <Autocomplete
                                        defaultValue={value || null}
                                        value={value || null}
                                        onChange={(event, newValue) => {
                                            const fieldName = attr.name || ""
                                            const selectedValue = newValue || ""
                                            handleSelectChange(fieldName)(selectedValue)
                                        }}
                                        options={
                                            attr.validations.options?.options
                                                ? [...attr.validations.options.options].sort()
                                                : ["-"]
                                        }
                                        getOptionLabel={(option) => t(option) || String(option)}
                                        renderInput={(params) => (
                                            <TextField
                                                {...params} // Spread all params provided by Autocomplete
                                                label={
                                                    `${getTranslationLabel(
                                                        attr.name,
                                                        attr.display_name,
                                                        t
                                                    )} ${isRequired ? "*" : ""}` || "-"
                                                }
                                                inputProps={{
                                                    ...params.inputProps,
                                                    id: "autocomplete-input",
                                                    required: isRequired,
                                                    name: displayName,
                                                }}
                                                disabled={
                                                    !(
                                                        createMode ||
                                                        !electionEventId ||
                                                        canEditVoters ||
                                                        enabledByVoteNum
                                                    )
                                                }
                                                fullWidth
                                                style={{
                                                    display: "block",
                                                    visibility: "visible",
                                                }}
                                            />
                                        )}
                                        disabled={
                                            !(
                                                createMode ||
                                                !electionEventId ||
                                                canEditVoters ||
                                                enabledByVoteNum
                                            )
                                        }
                                    />
                                </FormControl>
                            </Grid>
                        </Grid>
                    )
                } else if (
                    attr.annotations?.inputType === "multiselect-checkboxes" &&
                    attr.annotations?.inputOptionLabels
                ) {
                    const choices = Object.entries(attr.annotations?.inputOptionLabels)?.map(
                        ([key, value]) => {
                            return {id: key, name: getAttributeLabel(value as string)}
                        }
                    )
                    return (
                        <FormControl component="fieldset">
                            <FormLabel component="legend" style={{margin: 0}}>
                                {getTranslationLabel(attr.name, attr.display_name, t)}
                            </FormLabel>
                            <FormGroup row>
                                {choices.map((choice) => {
                                    return (
                                        <FormControlLabel
                                            key={choice.id}
                                            control={
                                                <Checkbox
                                                    disabled={
                                                        !(
                                                            createMode ||
                                                            !electionEventId ||
                                                            canEditVoters ||
                                                            enabledByVoteNum ||
                                                            (!hasVoted &&
                                                                attr.name === "emailAndOrMobile" &&
                                                                canEditVotersEmailTlf)
                                                        )
                                                    }
                                                    checked={value && value.includes(choice.id)}
                                                    onChange={() =>
                                                        handleCheckboxChange(attr.name ?? "")(
                                                            choice.id
                                                        )
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
                            onChange={handleDateChange(attr.name)}
                            label={getTranslationLabel(attr.name, attr.display_name, t)}
                            disabled={
                                !(
                                    createMode ||
                                    !electionEventId ||
                                    canEditVoters ||
                                    enabledByVoteNum
                                )
                            }
                        />
                    )
                } else if (attr.name.toLowerCase().includes("area")) {
                    return
                } else if (attr.name.toLowerCase().includes("mobile-number")) {
                    return (
                        <PhoneInput
                            handlePhoneNumberChange={handlePhoneNumberChange(attr.name)}
                            label={getTranslationLabel(attr.name, attr.display_name, t)}
                            fullWidth
                            initialValue={value}
                            disabled={
                                !(
                                    createMode ||
                                    !electionEventId ||
                                    canEditVoters ||
                                    enabledByVoteNum ||
                                    (!hasVoted && canEditVotersEmailTlf)
                                )
                            }
                        />
                    )
                } else if (attr.name.toLowerCase().includes("trustee")) {
                    return (
                        <FormControl fullWidth>
                            <SelectActedTrustee
                                label={t("usersAndRolesScreen.users.fields.trustee")}
                                source={createMode ? "attributes.trustee" : "trustee"}
                                defaultValue={value?.[0] ?? ""}
                                tenantId={tenantId}
                                onSelectTrustee={(trustee: string) => {
                                    setSelectedActedTrustee(trustee)
                                }}
                            />
                        </FormControl>
                    )
                } else if (attr.name.toLowerCase().includes("authorized-election-ids")) {
                    return (
                        <>
                            <FormStyles.AutocompleteArrayInput
                                label={getTranslationLabel(attr.name, attr.display_name, t)}
                                className="elections-selector"
                                fullWidth
                                choices={electionsList || []}
                                source="attributes.authorized-election-ids"
                                optionValue="alias"
                                optionText={aliasRenderer}
                                onChange={handleArraySelectChange}
                                disabled={
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum
                                    )
                                }
                            />
                        </>
                    )
                } else if (attr.name.toLowerCase().includes("permission_labels")) {
                    return (
                        <AutocompleteArrayInput
                            key={user?.id || "create"}
                            source={`attributes.${attr.name}`}
                            label={t("usersAndRolesScreen.users.fields.permissionLabel")}
                            defaultValue={permissionLabels}
                            fullWidth
                            onChange={handlePermissionLabelRemoved}
                            onCreate={(newLabel) => {
                                if (newLabel) {
                                    const updatedChoices = [
                                        ...choices,
                                        {id: newLabel, name: newLabel},
                                    ]
                                    const updatedLabels = [...permissionLabels, newLabel]
                                    setChoices(updatedChoices)
                                    setPermissionLabels(updatedLabels)
                                    handlePermissionLabelAdded(updatedLabels)
                                    return newLabel
                                }
                            }}
                            optionText="name"
                            choices={choices}
                            freeSolo={true}
                            disabled={
                                !(
                                    createMode ||
                                    !electionEventId ||
                                    canEditVoters ||
                                    enabledByVoteNum
                                )
                            }
                            onKeyDown={(e) => {
                                if (e.key === "Enter") {
                                    e.preventDefault()
                                    const input = e.target as HTMLInputElement
                                    const newLabel = input.value
                                    if (newLabel) {
                                        const updatedChoices = [
                                            ...choices,
                                            {id: newLabel, name: newLabel},
                                        ]
                                        const updatedLabels = [...permissionLabels, newLabel]
                                        setChoices(updatedChoices)
                                        setPermissionLabels(updatedLabels)
                                        handlePermissionLabelAdded(updatedLabels)
                                        input.value = ""
                                    }
                                }
                            }}
                        />
                    )
                }
                return (
                    <>
                        {isCustomAttribute ? (
                            <FormStyles.TextField
                                label={getTranslationLabel(attr.name, attr.display_name, t)}
                                value={value}
                                onChange={handleAttrChange(attr.name)}
                                disabled={
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum ||
                                        (!hasVoted &&
                                            attr.name === "email" &&
                                            canEditVotersEmailTlf)
                                    )
                                }
                            />
                        ) : (
                            <FormStyles.TextInput
                                key={attr.display_name}
                                label={getTranslationLabel(attr.name, attr.display_name, t)}
                                onChange={handleChange}
                                source={attr.name}
                                required={isFieldRequired(attr)}
                                disabled={
                                    (attr.name === "username" && !createMode) ||
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum ||
                                        (!hasVoted &&
                                            attr.name === "email" &&
                                            canEditVotersEmailTlf)
                                    )
                                }
                            />
                        )}
                    </>
                )
            }
        },
        [user, permissionLabels, choices, electionsList]
    )

    const isFieldRequired = (config: UserProfileAttribute): boolean => {
        // changed: required is controlled from keycloak
        // if the user profile attribute is not null in keycloak (at tenant or election event levels),
        // then the field is required
        // exceot username thai is always required
        if ((config?.required?.roles || config?.name === "username") && config?.name !== "email") {
            return true
        }
        return false
    }

    const formFields = useMemo(() => {
        // to check if fields are required
        return userAttributes?.map((attr) => renderFormField(attr))
    }, [userAttributes, user, permissionLabels, choices, electionsList])

    if (!user && !createMode) {
        return null
    }

    // Update the area selection handler
    const handleAreaSelection = (areaId: string) => {
        if (createMode) {
            setUser((prev) => ({
                ...prev,
                attributes: {
                    ...(prev?.attributes || {}),
                    "area-id": [areaId],
                },
            }))
        } else {
            setUser((prev) => ({
                ...prev,
                area: {
                    id: areaId,
                },
                attributes: {
                    ...(prev?.attributes || {}),
                    "area-id": [areaId],
                },
            }))
        }
    }

    return (
        <PageHeaderStyles.Wrapper>
            <SimpleForm
                toolbar={<SaveButton alwaysEnable={!errorText} />}
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
                        label={`${t("usersAndRolesScreen.users.fields.enabled")} *`}
                        control={
                            <Checkbox
                                disabled={
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum
                                    )
                                }
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
                                {` *`}
                            </ElectionHeaderStyles.Title>
                            <SelectArea
                                tenantId={tenantId}
                                electionEventId={electionEventId}
                                source={createMode ? "attributes.area-id[0]" : "area.id"}
                                onSelectArea={handleAreaSelection}
                                label=""
                                isRequired={true}
                                disabled={
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum
                                    )
                                }
                                customStyle={{
                                    "& legend": {
                                        display: "none",
                                    },
                                }}
                            />
                        </FormControl>
                    )}
                    <>
                        <FormControl fullWidth>
                            <ElectionHeaderStyles.Title>
                                {t("usersAndRolesScreen.users.fields.password")}:
                            </ElectionHeaderStyles.Title>
                            <PasswordInputStyle
                                label={false}
                                source="password"
                                onChange={handleChange}
                                error={!!errorText}
                                disabled={
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum
                                    )
                                }
                            />
                        </FormControl>
                        <FormControl fullWidth>
                            <ElectionHeaderStyles.Title>
                                {t("usersAndRolesScreen.users.fields.repeatPassword")}:
                            </ElectionHeaderStyles.Title>
                            <PasswordInputStyle
                                label={false}
                                source="confirm_password"
                                onChange={handleChange}
                                helperText={errorText}
                                error={!!errorText}
                                disabled={
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum
                                    )
                                }
                            />
                        </FormControl>
                        <InputContainerStyle sx={{flexDirection: "row !important"}}>
                            <InputLabelStyle paddingTop={false}>
                                <Box sx={{display: "flex", gap: "8px"}}>
                                    {t(`usersAndRolesScreen.editPassword.temporatyLabel`)}
                                    <IconTooltip
                                        icon={faInfoCircle}
                                        info={t(`usersAndRolesScreen.editPassword.temporatyInfo`)}
                                    />
                                </Box>
                            </InputLabelStyle>
                            <BooleanInput
                                source=""
                                label={false}
                                onChange={(e) => setTemportay(!temporary)}
                                checked={temporary}
                                disabled={
                                    !(
                                        createMode ||
                                        !electionEventId ||
                                        canEditVoters ||
                                        enabledByVoteNum
                                    )
                                }
                            />
                        </InputContainerStyle>
                    </>
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

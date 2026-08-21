// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
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
    getAttributeLengthBounds,
    getAttributeViolation,
    getStatedLengthBounds,
    getInputOptionLabels,
    getSelectOptionLabel,
    getTranslationLabel,
    resolveOptionLabel,
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
import {CustomAutocompleteArrayInput, ReviewChangesTable} from "@sequentech/ui-essentials"
import {getSaveUserErrorMessage} from "./saveUserError"
import {VOTED_CHANNEL} from "./ListUsers"
import {WizardStyles} from "@/components/styles/WizardStyles"
import {computeRoleDiff, computeUserDiff, UserBaseline} from "@/services/UserEditReviewChanges"

interface ListUserRolesProps {
    userId?: string
    userRoles?: ListUserRolesQuery
    rolesList: Array<IRole>
    activeRoleIds?: string[]
    onToggleRole?: (id: string) => void
}

export interface Trustee {
    id: string
    name: string
}

const getAttributeStringValue = (value: string | string[] | null | undefined): string => {
    if (Array.isArray(value)) {
        return value[0] ?? ""
    }

    return value ?? ""
}

interface AttributeTextInputProps {
    disabled: boolean
    error?: boolean
    helperText?: string
    label: string
    maxLength?: number
    onCommit: (value: string) => void
    onValidate?: (value: string) => void
    required: boolean
    value: string | string[] | null | undefined
    type?: string
}

// Uncontrolled by design: the DOM owns the value while the user types, so a
// re-render triggered by anything else can never fight the input over its
// current text. The parent's attribute value is only read on mount (via
// defaultValue/key) and only written back on blur.
const AttributeTextInput: React.FC<AttributeTextInputProps> = ({
    disabled,
    error,
    helperText,
    label,
    maxLength,
    onCommit,
    onValidate,
    required,
    value,
    type,
}) => {
    const normalizedValue = getAttributeStringValue(value)

    return (
        <FormStyles.TextField
            key={normalizedValue}
            type={type}
            label={label}
            defaultValue={normalizedValue}
            onBlur={(event) => {
                onValidate?.(event.target.value)
                if (event.target.value !== normalizedValue) {
                    onCommit(event.target.value)
                }
            }}
            disabled={disabled}
            error={error}
            helperText={helperText}
            required={required}
            fullWidth
            slotProps={{htmlInput: {maxLength}}}
            InputLabelProps={type === "date" ? {shrink: true} : undefined}
        />
    )
}

export const ListUserRoles: React.FC<ListUserRolesProps> = ({
    userRoles,
    rolesList,
    activeRoleIds,
    onToggleRole,
}) => {
    const effectiveActiveRoleIds =
        activeRoleIds ?? userRoles?.list_user_roles.map((role) => role.id || "") ?? []

    let rows: Array<IRole & {id: string; active: boolean}> = rolesList.map((role) => ({
        ...role,
        id: role.id || "",
        active: effectiveActiveRoleIds.includes(role.id || "") || false,
    }))

    const editRolePermission = (props: GridRenderCellParams<any, boolean>) => async () => {
        const role = (rolesList || []).find((el) => el.id === props.row.id)
        if (!role?.id) {
            return
        }
        onToggleRole?.(role.id)
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

// Datafix voter edits are deferred to a task; the mutation then returns a
// task_execution instead of the edited user, which the caller surfaces as a
// progress widget.
interface EditUserMutationResult {
    edit_user: {
        user?: IUser | null
        task_execution?: {id: string} | null
    }
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
    onTaskLaunched?: (taskExecutionId: string) => void
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
    onTaskLaunched,
}) => {
    const {t} = useTranslation()
    const reviewI18nContext = electionEventId ? "voters" : "users"

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
    const [selectedRoleIds, setSelectedRoleIds] = useState<string[]>([])
    const [phoneInputs, setPhoneInputs] = useState<{[key: string]: string[]}>({})
    const {canEditVoters, canEditVotersWhoVoted, canEditVotersEmailTlf} = useUsersPermissions()
    const [tenantId] = useTenantStore()
    const refresh = useRefresh()
    const notify = useNotify()
    const authContext = useContext(AuthContext)
    const [createUser] = useMutation<CreateUserMutation>(CREATE_USER)
    const [edit_user] = useMutation<EditUserMutationResult>(EDIT_USER)
    const [deleteUserRole] = useMutation<DeleteUserRoleMutation>(DELETE_USER_ROLE)
    const [setUserRole] = useMutation<SetUserRoleMutation>(SET_USER_ROLE)
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
    const [saveError, setSaveError] = useState("")
    const [saving, setSaving] = useState(false)
    const [lengthErrors, setLengthErrors] = useState<Record<string, string>>({})
    // Guarded through a ref as well as state: two clicks dispatched before
    // React commits the first would both read the same stale state.
    const savingRef = useRef(false)
    const closedRef = useRef(false)

    const [step, setStep] = useState<"edit" | "review">("edit")
    const baselineRef = useRef<UserBaseline>({
        user: createMode
            ? {enabled: true, attributes: {}}
            : (record && convertRecordToUser(record)) || {attributes: {}},
        phoneInputs: {},
    })
    const baselineRoleIdsRef = useRef<string[]>([])
    const rolesInitializedRef = useRef(false)
    const reviewHeadingRef = useRef<HTMLHeadingElement>(null)

    // Derived (not snapshotted) so the review table always reflects the live
    // user/phoneInputs/selectedActedTrustee state that handleConfirmChanges
    // actually submits, even if a debounced field update lands after Save.
    const reviewRows = useMemo(() => {
        if (step !== "review") {
            return []
        }
        const userRows = computeUserDiff(
            baselineRef.current,
            {user, phoneInputs, selectedActedTrustee},
            userAttributes,
            t
        )
        const roleRows = computeRoleDiff(
            {activeRoleIds: baselineRoleIdsRef.current},
            {activeRoleIds: selectedRoleIds},
            rolesList,
            t
        )
        return [...userRows, ...roleRows]
    }, [
        step,
        user,
        phoneInputs,
        selectedActedTrustee,
        userAttributes,
        selectedRoleIds,
        rolesList,
        t,
    ])

    useEffect(() => {
        if (!id) {
            return
        }
        setStep("edit")
        setSaveError("")
        setLengthErrors({})
        savingRef.current = false
        closedRef.current = false
        setSaving(false)
    }, [id])

    useEffect(() => {
        rolesInitializedRef.current = false
        baselineRoleIdsRef.current = []
        setSelectedRoleIds([])
    }, [id])

    useEffect(() => {
        if (step === "review" && reviewHeadingRef.current) {
            reviewHeadingRef.current.focus()
        }
    }, [step])

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

    useEffect(() => {
        if (createMode || rolesInitializedRef.current || !userRoles) {
            return
        }

        const initialRoleIds = userRoles.list_user_roles.map((role) => role.id || "")
        baselineRoleIdsRef.current = initialRoleIds
        setSelectedRoleIds(initialRoleIds)
        rolesInitializedRef.current = true
    }, [createMode, userRoles])

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

    const handleSelectedRoles = useCallback((roleId: string) => {
        setSelectedRoleIds((prev) =>
            prev.includes(roleId) ? prev.filter((id) => id !== roleId) : [...prev, roleId]
        )
    }, [])

    const beginSave = (): boolean => {
        if (savingRef.current) {
            return false
        }
        savingRef.current = true
        setSaving(true)
        setSaveError("")

        return true
    }

    const endSave = () => {
        // A form that has asked to close is still mounted and clickable for as
        // long as the drawer animates out, so it stays latched rather than
        // re-arming its buttons for that window. The latch does not outlive the
        // drawer: it is a temporary one, so closing it unmounts this form and
        // the next one starts from fresh refs.
        if (closedRef.current) {
            return
        }
        savingRef.current = false
        setSaving(false)
    }

    // Latched only once the caller has taken over, so a form with nothing to
    // close, or whose caller throws, re-arms instead of going quietly dead.
    const closeForm = () => {
        if (!close) {
            return
        }
        close()
        closedRef.current = true
    }

    // Stated under the field so the bound is known before it is broken, and
    // measured on blur so nothing the admin types is ever altered for them.
    const lengthHint = (attr: UserProfileAttribute): string | undefined => {
        const bounds = getStatedLengthBounds(getAttributeLengthBounds(attr))
        if (!bounds) {
            return undefined
        }
        if (bounds.min !== undefined && bounds.max !== undefined) {
            return t("usersAndRolesScreen.voters.errors.attribute.hintBetween", {
                min: bounds.min,
                max: bounds.max,
            })
        }

        return bounds.min !== undefined
            ? t("usersAndRolesScreen.voters.errors.attribute.hintMin", {min: bounds.min})
            : t("usersAndRolesScreen.voters.errors.attribute.hintMax", {max: bounds.max})
    }

    // What a field reports once it has been touched. Only touched fields are
    // checked, so opening a voter never lights up over data the admin has not
    // been near.
    const attributeError = (attr: UserProfileAttribute, value: string): string | undefined => {
        const bounds = getAttributeLengthBounds(attr)
        const violation = getAttributeViolation(bounds, value, isFieldRequired(attr))
        if (!violation) {
            return undefined
        }

        const field = getTranslationLabel(attr.name, attr.display_name, t)
        // Both bounds together read better as one sentence than as whichever
        // end the value happened to fall off.
        const messageKey =
            violation !== "required" && bounds?.min !== undefined && bounds?.max !== undefined
                ? "invalidLength"
                : violation

        return t(`usersAndRolesScreen.voters.errors.attribute.${messageKey}`, {
            field,
            min: bounds?.min,
            max: bounds?.max,
        })
    }

    const validateAttribute = (attr: UserProfileAttribute) => (value: string) => {
        const name = attr.name
        if (!name) {
            return
        }
        const message = attributeError(attr, value)

        setLengthErrors((previous) => {
            if (previous[name] === message) {
                return previous
            }
            if (!message) {
                const {[name]: _removed, ...rest} = previous
                return rest
            }

            return {...previous, [name]: message}
        })
    }

    const resolveFieldLabel = (field: string): string => {
        const attribute = userAttributes.find((candidate) => candidate.name === field)

        return getTranslationLabel(field, attribute?.display_name, t)
    }

    // The extracted reason is capped, so the raw rejection is logged for whoever
    // has to diagnose it. messageArgs carries the already translated message
    // through react-admin's polyglot provider, which would otherwise look it up
    // as a key and find nothing.
    const reportSaveError = (error: unknown, messageKey: string, reasonKey: string): string => {
        console.error("Error saving voter:", error)
        const message = getSaveUserErrorMessage(error, messageKey, reasonKey, t, resolveFieldLabel)
        notify(message, {type: "error", messageArgs: {_: message}})

        return message
    }

    const onSubmitCreateUser = async () => {
        if (!beginSave()) {
            return
        }
        try {
            await createUserAndPassword()
        } finally {
            endSave()
        }
    }

    const createUserAndPassword = async () => {
        let createdUserId: string | null | undefined
        try {
            const {data} = await createUser({
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
            createdUserId = data?.create_user?.id
        } catch (error) {
            setSaveError(
                reportSaveError(
                    error,
                    "usersAndRolesScreen.voters.errors.createError",
                    "usersAndRolesScreen.voters.errors.createErrorReason"
                )
            )
            return
        }

        // The voter exists from here on, so the form closes whatever happens
        // next: submitting it again would only try to create the voter twice.
        try {
            if ((user?.password?.length ?? 0) > 0 && createdUserId) {
                await handleUpdateUserPassword(createdUserId)
            }
            notify(t("usersAndRolesScreen.voters.errors.createSuccess"), {type: "success"})
        } catch (error) {
            reportSaveError(
                error,
                "usersAndRolesScreen.voters.errors.createPasswordError",
                "usersAndRolesScreen.voters.errors.createPasswordErrorReason"
            )
        }
        refresh()
        closeForm()
    }

    const onSubmit = async () => {
        // Reachable past the disabled button by submitting the form itself.
        if (Object.keys(lengthErrors).length > 0) {
            return
        }
        if (createMode) {
            await onSubmitCreateUser()
            return
        }
        setSaveError("")
        const diff = computeUserDiff(
            baselineRef.current,
            {user, phoneInputs, selectedActedTrustee},
            userAttributes,
            t
        )
        const roleDiff = computeRoleDiff(
            {activeRoleIds: baselineRoleIdsRef.current},
            {activeRoleIds: selectedRoleIds},
            rolesList,
            t
        )
        if (diff.length === 0 && roleDiff.length === 0) {
            notify(t(`usersAndRolesScreen.${reviewI18nContext}.review.noChanges`), {
                type: "info",
            })
            return
        }
        setStep("review")
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

    const handleConfirmChanges = async () => {
        if (!beginSave()) {
            return
        }
        try {
            const result = await handleEditUser()

            // Datafix edits run as a task: surface it as soon as the edit is
            // accepted, so ListUsers can still track an edit that is already
            // running if a later step fails.
            const taskExecutionId = result?.data?.edit_user?.task_execution?.id
            if (taskExecutionId) {
                onTaskLaunched?.(taskExecutionId)
            }

            const baselineRoleIds = baselineRoleIdsRef.current
            const rolesToRemove = baselineRoleIds.filter(
                (roleId) => !selectedRoleIds.includes(roleId)
            )
            const rolesToAdd = selectedRoleIds.filter((roleId) => !baselineRoleIds.includes(roleId))

            for (const roleId of rolesToRemove) {
                await deleteUserRole({
                    variables: {
                        tenantId,
                        roleId,
                        userId: user?.id,
                    },
                })
            }

            for (const roleId of rolesToAdd) {
                await setUserRole({
                    variables: {
                        tenantId,
                        roleId,
                        userId: user?.id,
                    },
                })
            }

            // Only moved once everything landed, so a retry after a failure
            // re-applies the whole set. Keycloak joins and leaves a group
            // idempotently, so re-applying what already landed is harmless.
            baselineRoleIdsRef.current = selectedRoleIds
            if (authContext.userId === user?.id) {
                authContext.updateTokenAndPermissionLabels()
            }
            // A launched task reports its own outcome in the widget, so a
            // premature success toast would contradict it.
            if (!taskExecutionId) {
                notify(t("usersAndRolesScreen.voters.errors.editSuccess"), {type: "success"})
            }
            refresh()
            closeForm()
        } catch (error) {
            // Whatever landed before the failure is already committed, so the
            // list behind the drawer has to catch up either way.
            refresh()
            setSaveError(
                reportSaveError(
                    error,
                    "usersAndRolesScreen.voters.errors.editError",
                    "usersAndRolesScreen.voters.errors.editErrorReason"
                )
            )
        } finally {
            endSave()
        }
    }

    const handleBackToEdit = () => {
        setStep("edit")
        setSaveError("")
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

    const handleDateChange = (attrName: string) => (value: string) => {
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

    const handlePermissionLabelChanged = (value: string[]) => {
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

    const formattedElections = electionsList?.map((e) => {
        return {
            external_id: e.external_id ?? "",
            id: e.id,
            name: aliasRenderer(e),
        }
    })

    const renderFormField = useCallback(
        (attr: UserProfileAttribute, index: number) => {
            if (attr.name === VOTED_CHANNEL) {
                return
            }
            if (attr.name) {
                const isCustomAttribute = !userBasicInfo.includes(attr.name)
                const value = isCustomAttribute
                    ? user?.attributes?.[attr.name]
                    : user && user[attr.name as keyof IUser]
                const displayName = attr.display_name ?? ""
                const isRequired = isFieldRequired(attr)
                const optionLabels = getInputOptionLabels(attr)
                if (attr.annotations?.inputType === "select") {
                    const configuredOptions = attr.validations?.options?.options
                    const selectOptions: string[] = Array.isArray(configuredOptions)
                        ? [...configuredOptions].sort()
                        : ["-"]
                    return (
                        <Grid key={index} container spacing={2}>
                            <Grid size={12}>
                                <FormControl fullWidth>
                                    <Autocomplete
                                        value={getAttributeStringValue(value) || null}
                                        onChange={(event, newValue) => {
                                            const fieldName = attr.name || ""
                                            const selectedValue = newValue || ""
                                            handleSelectChange(fieldName)(selectedValue)
                                        }}
                                        options={selectOptions}
                                        getOptionLabel={(option) =>
                                            getSelectOptionLabel(optionLabels, option, t)
                                        }
                                        renderInput={(params) => (
                                            <TextField
                                                {...params} // Spread all params provided by Autocomplete
                                                label={`${getTranslationLabel(
                                                    attr.name,
                                                    attr.display_name,
                                                    t
                                                )} ${isRequired ? "*" : ""}`}
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
                } else if (attr.annotations?.inputType === "multiselect-checkboxes") {
                    const choices = Object.entries(optionLabels ?? {}).map(([key, value]) => {
                        return {id: key, name: resolveOptionLabel(value, t)}
                    })
                    if (choices.length === 0) {
                        return
                    }
                    return (
                        <FormControl key={index} component="fieldset">
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
                        <AttributeTextInput
                            key={attr.name ?? index}
                            type="date"
                            label={getTranslationLabel(attr.name, attr.display_name, t)}
                            value={value}
                            onCommit={handleDateChange(attr.name)}
                            required={isRequired}
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
                            key={index}
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
                        <FormControl key={index} fullWidth>
                            <SelectActedTrustee
                                label={String(t("usersAndRolesScreen.users.fields.trustee"))}
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
                                key={index}
                                label={getTranslationLabel(attr.name, attr.display_name, t)}
                                className="elections-selector"
                                fullWidth
                                choices={formattedElections || []}
                                source="attributes.authorized-election-ids"
                                optionValue={"external_id"}
                                optionText={"name"}
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
                        <CustomAutocompleteArrayInput
                            key={index}
                            label={String(t("usersAndRolesScreen.users.fields.permissionLabel"))}
                            defaultValue={permissionLabels}
                            onChange={handlePermissionLabelChanged}
                            choices={choices}
                            disabled={
                                !createMode &&
                                !electionEventId &&
                                !canEditVoters &&
                                !enabledByVoteNum
                            }
                        />
                    )
                }
                return (
                    <>
                        {isCustomAttribute ? (
                            <AttributeTextInput
                                key={index}
                                label={getTranslationLabel(attr.name, attr.display_name, t)}
                                error={!!lengthErrors[attr.name]}
                                helperText={lengthErrors[attr.name] ?? lengthHint(attr)}
                                maxLength={getAttributeLengthBounds(attr)?.max}
                                onValidate={validateAttribute(attr)}
                                value={value}
                                onCommit={(newValue) => {
                                    const attrName = attr.name as string
                                    setUser((prev) => ({
                                        ...prev,
                                        attributes: {
                                            ...(prev?.attributes ?? {}),
                                            [attrName]: [newValue],
                                        },
                                    }))
                                }}
                                required={isRequired}
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
                                onBlur={(event) => validateAttribute(attr)(event.target.value)}
                                source={attr.name}
                                helperText={lengthErrors[attr.name] ?? lengthHint(attr)}
                                sx={
                                    lengthErrors[attr.name]
                                        ? {
                                              "& .MuiFormHelperText-root": {
                                                  color: "error.main",
                                              },
                                              "& .MuiOutlinedInput-root fieldset": {
                                                  borderColor: "error.main",
                                              },
                                              "& .MuiInputLabel-root": {color: "error.main"},
                                          }
                                        : undefined
                                }
                                slotProps={{
                                    htmlInput: {maxLength: getAttributeLengthBounds(attr)?.max},
                                }}
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
        [user, permissionLabels, choices, electionsList, lengthErrors]
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
        return userAttributes?.map((attr, index) => (
            <React.Fragment key={attr.name || index}>{renderFormField(attr, index)}</React.Fragment>
        ))
    }, [userAttributes, user, permissionLabels, choices, electionsList, lengthErrors])

    const hasFieldErrors = Object.keys(lengthErrors).length > 0

    const saveErrorAlert = saveError ? (
        <WizardStyles.ErrorMessage variant="body2" role="alert" className="edit-voter-save-error">
            {saveError}
        </WizardStyles.ErrorMessage>
    ) : null

    if (!user && !createMode) {
        return null
    }

    // Update the area selection handler
    const handleAreaSelection = (areaId: string, areaRecord?: {name?: string}) => {
        setSelectedArea(areaId)

        if (createMode) {
            setUser((prev) => ({
                ...prev,
                area: {
                    id: areaId,
                    name: areaRecord?.name,
                },
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
                    name: areaRecord?.name,
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
                toolbar={
                    step === "edit" ? (
                        <SaveButton
                            alwaysEnable={!errorText && !saving && !hasFieldErrors}
                            disabled={saving || hasFieldErrors}
                        />
                    ) : (
                        false
                    )
                }
                record={user}
                onSubmit={onSubmit}
                sanitizeEmptyValues
            >
                <PageHeaderStyles.Title>
                    {t(`usersAndRolesScreen.${reviewI18nContext}.title`)}
                </PageHeaderStyles.Title>
                <PageHeaderStyles.SubTitle>
                    {t(`usersAndRolesScreen.${reviewI18nContext}.subtitle`)}
                </PageHeaderStyles.SubTitle>
                <Box sx={{display: step === "review" ? "none" : undefined}}>
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
                    {createMode && (
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
                                            icon={faInfoCircle as any}
                                            info={String(
                                                t(`usersAndRolesScreen.editPassword.temporatyInfo`)
                                            )}
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
                    )}
                    {isUndefined(electionEventId) ? (
                        <ListUserRoles
                            userRoles={userRoles}
                            rolesList={rolesList}
                            activeRoleIds={createMode ? selectedRolesOnCreate : selectedRoleIds}
                            onToggleRole={
                                createMode ? handleSelectedRolesOnCreate : handleSelectedRoles
                            }
                        />
                    ) : null}
                    {step === "edit" && saveErrorAlert}
                </Box>
                {!createMode && step === "review" && (
                    <Box sx={{width: "100%"}}>
                        <ReviewChangesTable
                            title={t(`usersAndRolesScreen.${reviewI18nContext}.review.title`)}
                            subtitle={t(`usersAndRolesScreen.${reviewI18nContext}.review.subtitle`)}
                            fieldLabel={t(`usersAndRolesScreen.${reviewI18nContext}.review.field`)}
                            currentValueLabel={t(
                                `usersAndRolesScreen.${reviewI18nContext}.review.currentValue`
                            )}
                            newValueLabel={t(
                                `usersAndRolesScreen.${reviewI18nContext}.review.newValue`
                            )}
                            rows={reviewRows}
                            headingRef={reviewHeadingRef}
                        />
                        {saveErrorAlert}
                        <WizardStyles.FooterContainer>
                            <WizardStyles.StyledFooter>
                                <WizardStyles.BackButton
                                    type="button"
                                    onClick={handleBackToEdit}
                                    disabled={saving}
                                    className="edit-voter-review-edit-button"
                                >
                                    {t("common.label.edit")}
                                </WizardStyles.BackButton>
                                <WizardStyles.NextButton
                                    type="button"
                                    onClick={handleConfirmChanges}
                                    disabled={saving}
                                    className="edit-voter-review-confirm-button"
                                >
                                    {t(`usersAndRolesScreen.${reviewI18nContext}.review.confirm`)}
                                </WizardStyles.NextButton>
                            </WizardStyles.StyledFooter>
                        </WizardStyles.FooterContainer>
                    </Box>
                )}
            </SimpleForm>
        </PageHeaderStyles.Wrapper>
    )
}

// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {forwardRef, useEffect, useImperativeHandle, useMemo, useState} from "react"
import {useMutation, useQuery} from "@apollo/client"
import {
    Accordion,
    AccordionDetails,
    AccordionSummary,
    Box,
    Checkbox,
    FormControlLabel,
    Grid,
    TextField,
    Tooltip,
    Typography,
} from "@mui/material"
import ExpandMoreIcon from "@mui/icons-material/ExpandMore"
import InfoOutlinedIcon from "@mui/icons-material/InfoOutlined"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {IPermissions} from "@/types/keycloak"
import {
    GET_REALM_PASSWORD_POLICY,
    GetRealmPasswordPolicyQuery,
    RealmPasswordPolicy,
    UPDATE_REALM_PASSWORD_POLICY,
    UpdateRealmPasswordPolicyMutation,
} from "@/queries/RealmPasswordPolicy"

const MIN_PASSWORD_LENGTH = 1
const MAX_PASSWORD_LENGTH = 256

const DEFAULT_PASSWORD_POLICY: RealmPasswordPolicy = {
    configured: false,
    minimum_length: 12,
    maximum_length: 72,
    include_uppercase: true,
    include_lowercase: true,
    include_digits: true,
    include_special_characters: true,
}

export type PasswordPolicyValidationError = "lengthRange" | "minimumExceedsMaximum"

export const validatePasswordPolicy = (
    policy: RealmPasswordPolicy
): PasswordPolicyValidationError | undefined => {
    if (
        !Number.isInteger(policy.minimum_length) ||
        !Number.isInteger(policy.maximum_length) ||
        policy.minimum_length < MIN_PASSWORD_LENGTH ||
        policy.minimum_length > MAX_PASSWORD_LENGTH ||
        policy.maximum_length < MIN_PASSWORD_LENGTH ||
        policy.maximum_length > MAX_PASSWORD_LENGTH
    ) {
        return "lengthRange"
    }
    if (policy.minimum_length > policy.maximum_length) {
        return "minimumExceedsMaximum"
    }
    return undefined
}

export interface PasswordPolicyAccordionHandle {
    save: () => Promise<boolean>
}

interface PasswordPolicyAccordionProps {
    electionEventId?: string
    canEdit: boolean
    expanded: boolean
    onChange: () => void
    onDirty: () => void
}

interface PasswordPolicyFieldLabelProps {
    label: string
    information: string
}

const PasswordPolicyFieldLabel = ({label, information}: PasswordPolicyFieldLabelProps) => (
    <Box component="span" sx={{display: "inline-flex", alignItems: "center", gap: 0.5}}>
        <Typography component="span" variant="body2">
            {label}
        </Typography>
        <Tooltip title={information} arrow placement="top">
            <InfoOutlinedIcon
                color="action"
                fontSize="small"
                tabIndex={0}
                aria-label={information}
                sx={{cursor: "help"}}
            />
        </Tooltip>
    </Box>
)

export const PasswordPolicyAccordion = forwardRef<
    PasswordPolicyAccordionHandle,
    PasswordPolicyAccordionProps
>(({electionEventId, canEdit, expanded, onChange, onDirty}, ref) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const [passwordPolicy, setPasswordPolicy] =
        useState<RealmPasswordPolicy>(DEFAULT_PASSWORD_POLICY)
    const [dirty, setDirty] = useState(false)

    const {
        data,
        loading,
        error: queryError,
        refetch,
    } = useQuery<GetRealmPasswordPolicyQuery>(GET_REALM_PASSWORD_POLICY, {
        variables: {
            election_event_id: electionEventId,
        },
        skip: !electionEventId,
        fetchPolicy: "network-only",
        context: {
            headers: {
                "x-hasura-role": IPermissions.ELECTION_EVENT_READ,
            },
        },
    })
    const [updatePasswordPolicy] = useMutation<UpdateRealmPasswordPolicyMutation>(
        UPDATE_REALM_PASSWORD_POLICY,
        {
            context: {
                headers: {
                    "x-hasura-role": IPermissions.ELECTION_EVENT_WRITE,
                },
            },
        }
    )

    useEffect(() => {
        if (data?.get_realm_password_policy && !dirty) {
            setPasswordPolicy(data.get_realm_password_policy)
        }
    }, [data, dirty])

    const validationError = useMemo(() => validatePasswordPolicy(passwordPolicy), [passwordPolicy])

    const markDirty = () => {
        setDirty(true)
        onDirty()
    }

    const setNumberValue = (field: "minimum_length" | "maximum_length", value: string) => {
        setPasswordPolicy((current) => ({
            ...current,
            [field]: value === "" ? 0 : Number(value),
        }))
        markDirty()
    }

    const setBooleanValue = (
        field:
            | "include_uppercase"
            | "include_lowercase"
            | "include_digits"
            | "include_special_characters",
        value: boolean
    ) => {
        setPasswordPolicy((current) => ({
            ...current,
            [field]: value,
        }))
        markDirty()
    }

    useImperativeHandle(
        ref,
        () => ({
            save: async () => {
                if (!dirty) {
                    return true
                }
                if (validationError) {
                    notify(
                        t(`electionEventScreen.field.passwordPolicy.errors.${validationError}`),
                        {type: "error"}
                    )
                    return false
                }
                if (loading || queryError || !data) {
                    notify(t("electionEventScreen.edit.password_policy_not_loaded"), {
                        type: "error",
                    })
                    return false
                }
                if (!electionEventId) {
                    return false
                }

                try {
                    const response = await updatePasswordPolicy({
                        variables: {
                            election_event_id: electionEventId,
                            minimum_length: passwordPolicy.minimum_length,
                            maximum_length: passwordPolicy.maximum_length,
                            include_uppercase: passwordPolicy.include_uppercase,
                            include_lowercase: passwordPolicy.include_lowercase,
                            include_digits: passwordPolicy.include_digits,
                            include_special_characters: passwordPolicy.include_special_characters,
                        },
                    })
                    if (!response.data?.update_realm_password_policy.updated) {
                        throw new Error("Keycloak password policy was not updated")
                    }
                    setDirty(false)
                    await refetch()
                    return true
                } catch (error) {
                    console.error(error)
                    notify(t("electionEventScreen.edit.password_policy_update_error"), {
                        type: "error",
                    })
                    return false
                }
            },
        }),
        [
            data,
            dirty,
            electionEventId,
            loading,
            notify,
            passwordPolicy,
            queryError,
            refetch,
            t,
            updatePasswordPolicy,
            validationError,
        ]
    )

    const inputsDisabled = !canEdit || loading || Boolean(queryError) || !data

    return (
        <Accordion sx={{width: "100%"}} expanded={expanded} onChange={onChange}>
            <AccordionSummary
                expandIcon={<ExpandMoreIcon id="election-event-data-password-policy" />}
            >
                <ElectionHeaderStyles.Wrapper>
                    <ElectionHeaderStyles.Title>
                        {t("electionEventScreen.edit.password_policy")}
                    </ElectionHeaderStyles.Title>
                </ElectionHeaderStyles.Wrapper>
            </AccordionSummary>
            <AccordionDetails>
                {queryError ? (
                    <Typography color="error">
                        {t("electionEventScreen.edit.password_policy_load_error")}
                    </Typography>
                ) : loading ? (
                    <Typography>{t("loading")}</Typography>
                ) : (
                    <>
                        {!passwordPolicy.configured && (
                            <Typography color="warning.main" sx={{marginBottom: 2}}>
                                {t("electionEventScreen.field.passwordPolicy.notConfigured")}
                            </Typography>
                        )}
                        <Grid container spacing={2}>
                            <Grid size={{xs: 12, sm: 6, md: 3}}>
                                <PasswordPolicyFieldLabel
                                    label={String(
                                        t("electionEventScreen.field.passwordPolicy.minimumLength")
                                    )}
                                    information={String(
                                        t(
                                            "electionEventScreen.field.passwordPolicy.help.minimumLength"
                                        )
                                    )}
                                />
                                <TextField
                                    fullWidth
                                    type="number"
                                    disabled={inputsDisabled}
                                    sx={{marginTop: 1}}
                                    value={passwordPolicy.minimum_length}
                                    onChange={(event) =>
                                        setNumberValue("minimum_length", event.target.value)
                                    }
                                    slotProps={{
                                        htmlInput: {
                                            "min": MIN_PASSWORD_LENGTH,
                                            "max": MAX_PASSWORD_LENGTH,
                                            "aria-label": t(
                                                "electionEventScreen.field.passwordPolicy.minimumLength"
                                            ),
                                        },
                                    }}
                                />
                            </Grid>
                            <Grid size={{xs: 12, sm: 6, md: 3}}>
                                <PasswordPolicyFieldLabel
                                    label={String(
                                        t("electionEventScreen.field.passwordPolicy.maximumLength")
                                    )}
                                    information={String(
                                        t(
                                            "electionEventScreen.field.passwordPolicy.help.maximumLength"
                                        )
                                    )}
                                />
                                <TextField
                                    fullWidth
                                    type="number"
                                    disabled={inputsDisabled}
                                    sx={{marginTop: 1}}
                                    value={passwordPolicy.maximum_length}
                                    onChange={(event) =>
                                        setNumberValue("maximum_length", event.target.value)
                                    }
                                    slotProps={{
                                        htmlInput: {
                                            "min": MIN_PASSWORD_LENGTH,
                                            "max": MAX_PASSWORD_LENGTH,
                                            "aria-label": t(
                                                "electionEventScreen.field.passwordPolicy.maximumLength"
                                            ),
                                        },
                                    }}
                                />
                            </Grid>
                        </Grid>
                        <Box
                            sx={{
                                display: "grid",
                                gridTemplateColumns: {
                                    xs: "1fr",
                                    md: "repeat(2, minmax(240px, 1fr))",
                                },
                                gap: 1,
                                marginTop: 2,
                            }}
                        >
                            <FormControlLabel
                                control={
                                    <Checkbox
                                        disabled={inputsDisabled}
                                        checked={passwordPolicy.include_uppercase}
                                        onChange={(event) =>
                                            setBooleanValue(
                                                "include_uppercase",
                                                event.target.checked
                                            )
                                        }
                                    />
                                }
                                label={
                                    <PasswordPolicyFieldLabel
                                        label={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.includeUppercase"
                                            )
                                        )}
                                        information={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.help.includeUppercase"
                                            )
                                        )}
                                    />
                                }
                            />
                            <FormControlLabel
                                control={
                                    <Checkbox
                                        disabled={inputsDisabled}
                                        checked={passwordPolicy.include_lowercase}
                                        onChange={(event) =>
                                            setBooleanValue(
                                                "include_lowercase",
                                                event.target.checked
                                            )
                                        }
                                    />
                                }
                                label={
                                    <PasswordPolicyFieldLabel
                                        label={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.includeLowercase"
                                            )
                                        )}
                                        information={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.help.includeLowercase"
                                            )
                                        )}
                                    />
                                }
                            />
                            <FormControlLabel
                                control={
                                    <Checkbox
                                        disabled={inputsDisabled}
                                        checked={passwordPolicy.include_digits}
                                        onChange={(event) =>
                                            setBooleanValue("include_digits", event.target.checked)
                                        }
                                    />
                                }
                                label={
                                    <PasswordPolicyFieldLabel
                                        label={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.includeDigits"
                                            )
                                        )}
                                        information={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.help.includeDigits"
                                            )
                                        )}
                                    />
                                }
                            />
                            <FormControlLabel
                                control={
                                    <Checkbox
                                        disabled={inputsDisabled}
                                        checked={passwordPolicy.include_special_characters}
                                        onChange={(event) =>
                                            setBooleanValue(
                                                "include_special_characters",
                                                event.target.checked
                                            )
                                        }
                                    />
                                }
                                label={
                                    <PasswordPolicyFieldLabel
                                        label={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.includeSpecialCharacters"
                                            )
                                        )}
                                        information={String(
                                            t(
                                                "electionEventScreen.field.passwordPolicy.help.includeSpecialCharacters"
                                            )
                                        )}
                                    />
                                }
                            />
                        </Box>
                        {validationError && (
                            <Typography color="error" sx={{marginTop: 1}}>
                                {t(
                                    `electionEventScreen.field.passwordPolicy.errors.${validationError}`
                                )}
                            </Typography>
                        )}
                    </>
                )}
            </AccordionDetails>
        </Accordion>
    )
})

PasswordPolicyAccordion.displayName = "PasswordPolicyAccordion"

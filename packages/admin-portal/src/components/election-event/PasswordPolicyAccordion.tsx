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
    className: string
}

const PasswordPolicyFieldLabel = ({
    label,
    information,
    className,
}: PasswordPolicyFieldLabelProps) => (
    <Box
        component="span"
        className={`election-event-password-policy-field-label ${className}`}
        sx={{display: "inline-flex", alignItems: "center", gap: 0.5}}
    >
        <Typography
            component="span"
            variant="body2"
            className={`election-event-password-policy-field-label-text ${className}-text`}
        >
            {label}
        </Typography>
        <Tooltip
            className={`election-event-password-policy-tooltip-trigger ${className}-tooltip-trigger`}
            title={information}
            arrow
            placement="top"
            slotProps={{
                popper: {
                    className: `election-event-password-policy-tooltip-popper ${className}-tooltip-popper`,
                },
                tooltip: {
                    className: `election-event-password-policy-tooltip ${className}-tooltip`,
                },
            }}
        >
            <InfoOutlinedIcon
                className={`election-event-password-policy-information-icon ${className}-information-icon`}
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
        <Accordion
            className="election-event-password-policy-accordion"
            sx={{width: "100%"}}
            expanded={expanded}
            onChange={onChange}
        >
            <AccordionSummary
                className="election-event-password-policy-summary"
                expandIcon={
                    <ExpandMoreIcon
                        id="election-event-data-password-policy"
                        className="election-event-password-policy-expand-icon"
                    />
                }
            >
                <ElectionHeaderStyles.Wrapper className="election-event-password-policy-header">
                    <ElectionHeaderStyles.Title className="election-event-password-policy-title">
                        {t("electionEventScreen.edit.password_policy")}
                    </ElectionHeaderStyles.Title>
                </ElectionHeaderStyles.Wrapper>
            </AccordionSummary>
            <AccordionDetails className="election-event-password-policy-details">
                {queryError ? (
                    <Typography className="election-event-password-policy-load-error" color="error">
                        {t("electionEventScreen.edit.password_policy_load_error")}
                    </Typography>
                ) : loading ? (
                    <Typography className="election-event-password-policy-loading">
                        {t("loading")}
                    </Typography>
                ) : (
                    <Box className="election-event-password-policy-content">
                        {!passwordPolicy.configured && (
                            <Typography
                                className="election-event-password-policy-not-configured"
                                color="warning.main"
                                sx={{marginBottom: 2}}
                            >
                                {t("electionEventScreen.field.passwordPolicy.notConfigured")}
                            </Typography>
                        )}
                        <Grid
                            className="election-event-password-policy-length-fields"
                            container
                            spacing={2}
                        >
                            <Grid
                                className="election-event-password-policy-minimum-length-field"
                                size={{xs: 12, sm: 6, md: 3}}
                            >
                                <PasswordPolicyFieldLabel
                                    className="election-event-password-policy-minimum-length-label"
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
                                    className="election-event-password-policy-minimum-length-input"
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
                                            "className":
                                                "election-event-password-policy-minimum-length-native-input",
                                        },
                                    }}
                                />
                            </Grid>
                            <Grid
                                className="election-event-password-policy-maximum-length-field"
                                size={{xs: 12, sm: 6, md: 3}}
                            >
                                <PasswordPolicyFieldLabel
                                    className="election-event-password-policy-maximum-length-label"
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
                                    className="election-event-password-policy-maximum-length-input"
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
                                            "className":
                                                "election-event-password-policy-maximum-length-native-input",
                                        },
                                    }}
                                />
                            </Grid>
                        </Grid>
                        <Box
                            className="election-event-password-policy-character-requirements"
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
                                className="election-event-password-policy-include-uppercase-field"
                                control={
                                    <Checkbox
                                        className="election-event-password-policy-include-uppercase-checkbox"
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
                                        className="election-event-password-policy-include-uppercase-label"
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
                                className="election-event-password-policy-include-lowercase-field"
                                control={
                                    <Checkbox
                                        className="election-event-password-policy-include-lowercase-checkbox"
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
                                        className="election-event-password-policy-include-lowercase-label"
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
                                className="election-event-password-policy-include-digits-field"
                                control={
                                    <Checkbox
                                        className="election-event-password-policy-include-digits-checkbox"
                                        disabled={inputsDisabled}
                                        checked={passwordPolicy.include_digits}
                                        onChange={(event) =>
                                            setBooleanValue("include_digits", event.target.checked)
                                        }
                                    />
                                }
                                label={
                                    <PasswordPolicyFieldLabel
                                        className="election-event-password-policy-include-digits-label"
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
                                className="election-event-password-policy-include-special-characters-field"
                                control={
                                    <Checkbox
                                        className="election-event-password-policy-include-special-characters-checkbox"
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
                                        className="election-event-password-policy-include-special-characters-label"
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
                            <Typography
                                className="election-event-password-policy-validation-error"
                                color="error"
                                sx={{marginTop: 1}}
                            >
                                {t(
                                    `electionEventScreen.field.passwordPolicy.errors.${validationError}`
                                )}
                            </Typography>
                        )}
                    </Box>
                )}
            </AccordionDetails>
        </Accordion>
    )
})

PasswordPolicyAccordion.displayName = "PasswordPolicyAccordion"

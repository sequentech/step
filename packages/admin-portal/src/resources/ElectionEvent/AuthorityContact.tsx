// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Election authority contact" in the Transparency tab: how voters reach the
// authority, for example to report an audit mismatch. Four optional fields,
// stored with the election event's other settings. Windmill drafts the trust
// policy's `contact` lines from them, and voters' apps show the contact from
// the pinned policy only. They are inputs of the policy, so they are locked
// once voting has opened, like the other inputs.

import React, {useEffect, useState} from "react"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Alert, Box, Button, TextField, Typography} from "@mui/material"
import {IPermissions} from "@/types/keycloak"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {
    authorityContactPatch,
    CONTACT_KINDS,
    contactErrors,
    EContactKind,
    IAuthorityContact,
    readAuthorityContact,
    sameContact,
} from "@/services/AuthorityContact"
import {
    SET_AUTHORITY_CONTACT,
    SetAuthorityContactMutation,
    SetAuthorityContactMutationVariables,
} from "@/queries/Transparency"
import {BOARD, BoardSection} from "./TransparencyOverview"

const ROLE = {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_WRITE}}

const KEYS = "transparencyTab.authorityContact"

type TNotice = {severity: "success" | "error"; text: string}

const INPUT_MODE: Record<EContactKind, "text" | "email" | "tel" | "url"> = {
    [EContactKind.NAME]: "text",
    [EContactKind.EMAIL]: "email",
    [EContactKind.PHONE]: "tel",
    [EContactKind.URL]: "url",
}

const AUTOCOMPLETE: Record<EContactKind, string> = {
    [EContactKind.NAME]: "off",
    [EContactKind.EMAIL]: "off",
    [EContactKind.PHONE]: "off",
    [EContactKind.URL]: "off",
}

export interface IAuthorityContactSectionProps {
    electionEventId: string
    /** The election event's `presentation`, as its record has it. */
    presentation?: unknown
    /** Whether the admin may change the event's settings. */
    canChange: boolean
    /** The policy's inputs are frozen: voting has opened. */
    locked: boolean
    /** A policy is sealed: saving changes the draft, and the Board seals a
     * successor to carry the change. */
    sealed: boolean
    /** Called once the contact is stored, so that the draft policy and the
     * event's record are read again. */
    onChanged?: () => void | Promise<void>
}

export const AuthorityContactSection: React.FC<IAuthorityContactSectionProps> = ({
    electionEventId,
    presentation,
    canChange,
    locked,
    sealed,
    onChanged,
}) => {
    const {t} = useTranslation()
    const [values, setValues] = useState<IAuthorityContact>(() =>
        readAuthorityContact(presentation)
    )
    const [saved, setSaved] = useState<IAuthorityContact>(() => readAuthorityContact(presentation))
    const [notice, setNotice] = useState<TNotice | undefined>()
    const [save, {loading}] = useMutation<
        SetAuthorityContactMutation,
        SetAuthorityContactMutationVariables
    >(SET_AUTHORITY_CONTACT, {context: ROLE})

    // A fresh read of the record is the contact, whatever was typed before it.
    useEffect(() => {
        const stored = readAuthorityContact(presentation)
        setValues(stored)
        setSaved(stored)
    }, [presentation])

    const editable = canChange && !locked
    const errors = contactErrors(values)
    const dirty = !sameContact(values, saved)

    const submit = async (event: React.FormEvent) => {
        event.preventDefault()
        if (!editable || !dirty || errors.length > 0) return
        setNotice(undefined)
        try {
            const result = await save({
                variables: {electionEventId, patch: authorityContactPatch(values)},
            })
            // A null `authority_contact` means that nothing was stored: the
            // event has no presentation to append to.
            if (!result.data?.update_sequent_backend_election_event_by_pk?.authority_contact) {
                throw new Error(t(`${KEYS}.notStored`))
            }
            setSaved(values)
            setNotice({severity: "success", text: t(`${KEYS}.saved`)})
            await onChanged?.()
        } catch (e) {
            setNotice({
                severity: "error",
                text: t(`${KEYS}.saveError`, {
                    reason: getGraphQLActionErrorReason(e) ?? t("transparencyTab.unknownError"),
                }),
            })
        }
    }

    return (
        <BoardSection title={t(`${KEYS}.title`)} testId="authority-contact">
            <Typography sx={{fontSize: "14px", lineHeight: "20px", color: BOARD.muted}}>
                {t(`${KEYS}.intro`)}
            </Typography>
            {notice ? (
                <Alert
                    severity={notice.severity}
                    role={notice.severity === "error" ? "alert" : "status"}
                    data-testid="authority-contact-notice"
                    onClose={() => setNotice(undefined)}
                >
                    {notice.text}
                </Alert>
            ) : null}
            <Box
                component="form"
                noValidate
                onSubmit={submit}
                aria-label={t(`${KEYS}.title`)}
                sx={{display: "flex", flexDirection: "column", gap: "14px"}}
            >
                <Box
                    sx={{
                        display: "grid",
                        gridTemplateColumns: "repeat(auto-fill, minmax(260px, 1fr))",
                        gap: "14px",
                    }}
                >
                    {CONTACT_KINDS.map((kind) => {
                        const invalid = errors.includes(kind)
                        return (
                            <TextField
                                key={kind}
                                label={t(`${KEYS}.${kind}.label`)}
                                value={values[kind]}
                                onChange={(e) => setValues({...values, [kind]: e.target.value})}
                                disabled={!editable || loading}
                                error={invalid}
                                helperText={
                                    invalid
                                        ? t(`${KEYS}.${kind}.invalid`)
                                        : t(`${KEYS}.${kind}.help`)
                                }
                                size="small"
                                slotProps={{
                                    htmlInput: {
                                        "data-testid": `authority-contact-${kind}`,
                                        "inputMode": INPUT_MODE[kind],
                                        "autoComplete": AUTOCOMPLETE[kind],
                                        "spellCheck": false,
                                    },
                                }}
                            />
                        )
                    })}
                </Box>
                {editable ? (
                    <Box
                        sx={{display: "flex", alignItems: "center", gap: "12px", flexWrap: "wrap"}}
                    >
                        <Button
                            type="submit"
                            variant="contained"
                            disabled={!dirty || errors.length > 0 || loading}
                            data-testid="authority-contact-save"
                        >
                            {t(`${KEYS}.save`)}
                        </Button>
                        {sealed && dirty ? (
                            <Typography
                                data-testid="authority-contact-successor"
                                sx={{fontSize: "13px", lineHeight: "19px", color: BOARD.subtle}}
                            >
                                {t(`${KEYS}.successor`)}
                            </Typography>
                        ) : null}
                    </Box>
                ) : (
                    <Typography
                        data-testid="authority-contact-read-only"
                        sx={{fontSize: "13px", color: BOARD.subtle}}
                    >
                        {t(locked ? `${KEYS}.locked` : `${KEYS}.readOnly`)}
                    </Typography>
                )}
            </Box>
            <Typography sx={{fontSize: "12px", lineHeight: "17px", color: BOARD.subtle}}>
                {t(`${KEYS}.footer`)}
            </Typography>
        </BoardSection>
    )
}

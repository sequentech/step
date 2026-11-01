// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

// "Updates from hubs" in the Transparency tab (design board AdminHubUpdates):
// two switches for the voting portal's invitations to get email updates from
// the hubs the policy names, and the read-only list of those hubs. Both
// switches are on by default and are stored with the election event's other
// voting portal settings.

import React, {useEffect, useId, useState} from "react"
import {useMutation} from "@apollo/client"
import {useTranslation} from "react-i18next"
import {Alert, Box, Switch, Typography} from "@mui/material"
import type {Policy} from "@vitrine/verify"
import {IPermissions} from "@/types/keycloak"
import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {
    EHubUpdate,
    hubUpdatesPatch,
    IHubUpdates,
    offeredHubs,
    readHubUpdates,
} from "@/services/HubUpdates"
import {
    SET_HUB_UPDATES,
    SetHubUpdatesMutation,
    SetHubUpdatesMutationVariables,
} from "@/queries/Transparency"
import {BOARD, BoardSection} from "./TransparencyOverview"

const ROLE = {headers: {"x-hasura-role": IPermissions.ELECTION_EVENT_WRITE}}

const KEYS = "transparencyTab.hubUpdates"

type TNotice = {severity: "success" | "error"; text: string}

interface ISwitchRowProps {
    name: keyof IHubUpdates
    value: EHubUpdate
    disabled: boolean
    onToggle: (name: keyof IHubUpdates, value: EHubUpdate) => void
}

const SwitchRow: React.FC<ISwitchRowProps> = ({name, value, disabled, onToggle}) => {
    const {t} = useTranslation()
    const labelId = useId()
    const helpId = useId()
    return (
        <Box
            data-testid={`hub-updates-${name}`}
            sx={{
                display: "flex",
                alignItems: "flex-start",
                gap: "12px",
                p: "12px 0",
                borderTop: `1px solid ${BOARD.border}`,
            }}
        >
            <Switch
                checked={value === EHubUpdate.ON}
                disabled={disabled}
                onChange={(event) =>
                    onToggle(name, event.target.checked ? EHubUpdate.ON : EHubUpdate.OFF)
                }
                slotProps={{
                    input: {
                        "role": "switch",
                        "aria-labelledby": labelId,
                        "aria-describedby": helpId,
                    },
                }}
                sx={{
                    "mt": "-6px",
                    "& .MuiSwitch-switchBase.Mui-checked": {color: BOARD.white},
                    "& .MuiSwitch-switchBase.Mui-checked + .MuiSwitch-track": {
                        backgroundColor: BOARD.navy,
                        opacity: 1,
                    },
                    "& .Mui-focusVisible .MuiSwitch-thumb": {
                        outline: `2px solid ${BOARD.navy}`,
                        outlineOffset: "2px",
                    },
                }}
            />
            <Box sx={{flex: 1, minWidth: 0}}>
                <Typography
                    id={labelId}
                    sx={{fontSize: "15px", lineHeight: "22px", fontWeight: 500, color: BOARD.text}}
                >
                    {t(`${KEYS}.${name}.label`)}
                </Typography>
                <Typography
                    id={helpId}
                    sx={{fontSize: "13px", lineHeight: "19px", color: BOARD.subtle}}
                >
                    {t(`${KEYS}.${name}.help`)}
                </Typography>
            </Box>
        </Box>
    )
}

export interface IHubUpdatesSectionProps {
    electionEventId: string
    /** The election event's `presentation`, as its record has it. */
    presentation?: unknown
    /** The policy the event runs under, if it has one. */
    policy?: Policy
    /** Whether the admin may change the event's settings; if not, the
     * switches are shown as they are. */
    canChange: boolean
    /** Called once the settings are stored, so the event's record is read again. */
    onChanged?: () => void
}

export const HubUpdatesSection: React.FC<IHubUpdatesSectionProps> = ({
    electionEventId,
    presentation,
    policy,
    canChange,
    onChanged,
}) => {
    const {t} = useTranslation()
    const [stored, setStored] = useState<IHubUpdates | undefined>()
    const [notice, setNotice] = useState<TNotice | undefined>()
    const [save, {loading}] = useMutation<SetHubUpdatesMutation, SetHubUpdatesMutationVariables>(
        SET_HUB_UPDATES,
        {context: ROLE}
    )

    // A fresh read of the record is the settings, whatever was saved before it.
    useEffect(() => setStored(undefined), [presentation])

    const current = stored ?? readHubUpdates(presentation)
    const hubs = offeredHubs(policy)

    const toggle = async (name: keyof IHubUpdates, value: EHubUpdate) => {
        const next = {...current, [name]: value}
        setNotice(undefined)
        try {
            const result = await save({
                variables: {electionEventId, patch: hubUpdatesPatch(next)},
            })
            // A null `hub_updates` means that nothing was stored: the event
            // has no presentation to append to.
            if (!result.data?.update_sequent_backend_election_event_by_pk?.hub_updates) {
                throw new Error(t(`${KEYS}.notStored`))
            }
            setStored(next)
            setNotice({severity: "success", text: t(`${KEYS}.saved`)})
            onChanged?.()
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
        <>
            <BoardSection title={t(`${KEYS}.title`)} testId="hub-updates">
                <Typography sx={{fontSize: "14px", lineHeight: "20px", color: BOARD.muted}}>
                    {t(`${KEYS}.intro`)}
                </Typography>
                {notice ? (
                    <Alert
                        severity={notice.severity}
                        role={notice.severity === "error" ? "alert" : "status"}
                        data-testid="hub-updates-notice"
                        onClose={() => setNotice(undefined)}
                    >
                        {notice.text}
                    </Alert>
                ) : null}
                <Box>
                    {(["confirmation", "castEmail"] as const).map((name) => (
                        <SwitchRow
                            key={name}
                            name={name}
                            value={current[name]}
                            disabled={!canChange || loading}
                            onToggle={toggle}
                        />
                    ))}
                </Box>
                {canChange ? null : (
                    <Typography
                        data-testid="hub-updates-read-only"
                        sx={{fontSize: "13px", color: BOARD.subtle}}
                    >
                        {t(`${KEYS}.readOnly`)}
                    </Typography>
                )}
                <Box
                    data-testid="hub-updates-hubs"
                    sx={{borderTop: `1px solid ${BOARD.border}`, pt: "12px"}}
                >
                    <Typography
                        component="h4"
                        sx={{
                            fontSize: "15px",
                            lineHeight: "22px",
                            fontWeight: 500,
                            color: BOARD.text,
                        }}
                    >
                        {t(`${KEYS}.hubs.title`)}
                    </Typography>
                    <Typography sx={{fontSize: "13px", lineHeight: "19px", color: BOARD.subtle}}>
                        {t(`${KEYS}.hubs.help`)}
                    </Typography>
                    {hubs.length === 0 ? (
                        <Typography
                            data-testid="hub-updates-no-hubs"
                            sx={{mt: "10px", fontSize: "13.5px", color: BOARD.muted}}
                        >
                            {t(`${KEYS}.hubs.none`)}
                        </Typography>
                    ) : (
                        <>
                            <Box
                                component="ul"
                                sx={{
                                    display: "grid",
                                    gridTemplateColumns: "repeat(auto-fill, minmax(240px, 1fr))",
                                    gap: "8px",
                                    listStyle: "none",
                                    m: "10px 0 0",
                                    p: 0,
                                }}
                            >
                                {hubs.map((hub) => (
                                    <Box
                                        component="li"
                                        key={hub.name}
                                        data-testid={`hub-updates-hub-${hub.name}`}
                                        sx={{
                                            display: "flex",
                                            alignItems: "center",
                                            flexWrap: "wrap",
                                            gap: "4px 10px",
                                            p: "10px 12px",
                                            border: `1px solid ${BOARD.border}`,
                                            borderRadius: "6px",
                                            fontSize: "14px",
                                            lineHeight: "20px",
                                        }}
                                    >
                                        <Box
                                            component="b"
                                            sx={{fontWeight: 600, color: BOARD.navy}}
                                        >
                                            {hub.name}
                                        </Box>
                                        <Box component="span" sx={{color: BOARD.subtle}}>
                                            {t("transparencyTab.overview.runBy", {
                                                party: hub.party,
                                            })}
                                        </Box>
                                    </Box>
                                ))}
                            </Box>
                            <Typography
                                sx={{
                                    mt: "10px",
                                    fontSize: "13px",
                                    lineHeight: "19px",
                                    color: BOARD.subtle,
                                }}
                            >
                                {t(`${KEYS}.hubs.order`)}
                            </Typography>
                        </>
                    )}
                </Box>
            </BoardSection>
            <Typography
                sx={{m: "-10px 2px 0", fontSize: "12px", lineHeight: "17px", color: BOARD.subtle}}
            >
                {t(`${KEYS}.footer`)}
            </Typography>
        </>
    )
}

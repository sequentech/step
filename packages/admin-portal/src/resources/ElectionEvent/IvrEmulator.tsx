// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useState} from "react"
import {BooleanInput, useGetList, useRecordContext} from "react-admin"
import {FormProvider, useForm} from "react-hook-form"
import {Alert, AlertTitle, Box, Button, Stack} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    Sequent_Backend_Ballot_Style,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {IvrApiStatus, useIvrEmulator} from "@/providers/IvrEmulatorContextProvider"
import {v4} from "uuid"
import {EmulatorConfig} from "@/services/IvrEmulator"
import {IvrCall, IvrCallStatus} from "@sequentech/ui-essentials"
import SelectArea from "@/components/area/SelectArea"
import {useFormContext, useWatch} from "react-hook-form"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {FormStyles} from "@/components/styles/FormStyles"

const CALLER_NUMBER = "+1234567890"

const generateContactId = (): string => v4()

type ConfigFormValues = {
    areaId: string
    electionIds: string[]
    blacklistCaller: boolean
}

const ConfigFormBody: React.FC<{
    electionEvent: Sequent_Backend_Election_Event
    onStartSession: (config: EmulatorConfig) => void
}> = ({electionEvent, onStartSession}) => {
    const {t} = useTranslation()
    const aliasRenderer = useAliasRenderer()
    const {control, getValues} = useFormContext<ConfigFormValues>()

    const areaId = useWatch({control, name: "areaId"})
    const electionIds = useWatch({control, name: "electionIds"})

    const {data: rawBallotStyles} = useGetList<Sequent_Backend_Ballot_Style>(
        "sequent_backend_ballot_style",
        {
            pagination: {page: 1, perPage: 300},
            // Sort in ascending order so newer styles overwrite older ones
            //  when constructing the map.
            sort: {field: "created_at", order: "DESC"},
            filter: {
                tenant_id: electionEvent.tenant_id,
                election_event_id: electionEvent.id,
                area_id: areaId,
                deleted_at: {
                    format: "hasura-raw-query",
                    value: {_is_null: true},
                },
            },
        },
        {
            enabled: !!areaId,
        }
    )

    const availableBallotStyles = useMemo<Map<string, string>>(() => {
        return new Map(
            rawBallotStyles?.flatMap((style) => {
                if (
                    style.election_id &&
                    typeof style.election_id === "string" &&
                    style.ballot_eml &&
                    !style.deleted_at
                ) {
                    return [[style.election_id, style.ballot_eml]]
                } else {
                    return []
                }
            }) ?? []
        )
    }, [rawBallotStyles])

    const {data: rawElections} = useGetList<Sequent_Backend_Election>("sequent_backend_election", {
        pagination: {page: 1, perPage: 300},
        sort: {field: "name", order: "DESC"},
        filter: {
            tenant_id: electionEvent.tenant_id,
            election_event_id: electionEvent.id,
        },
    })

    const electionChoices = useMemo<{id: string; name: string}[]>(
        () =>
            rawElections
                ?.filter((e) => availableBallotStyles.has(e.id))
                .map((e) => ({
                    id: e.id,
                    name: aliasRenderer(e),
                }))
                .sort((a, b) => a.name.localeCompare(b.name)) ?? [],
        [rawElections, availableBallotStyles]
    )

    const resolvedBallotStyles = useMemo<Map<string, string>>(() => {
        let filtered = availableBallotStyles
            .entries()
            .filter(([electionId, _style]) => electionIds.includes(electionId))
        return new Map(filtered)
    }, [electionIds, availableBallotStyles])

    const generateConfig = (data: ConfigFormValues): EmulatorConfig => {
        return {
            tenant_id: electionEvent.tenant_id,
            election_event_id: electionEvent.id,
            caller_number: CALLER_NUMBER,
            contact_id: generateContactId(),
            blacklisted_numbers: data.blacklistCaller ? [CALLER_NUMBER] : [],
            open_elections: Array.from(resolvedBallotStyles.keys()),
            election_event: JSON.stringify(electionEvent),
            ballot_styles: Array.from(resolvedBallotStyles.values()),
        }
    }

    const onSubmit = () => {
        let config = generateConfig(getValues())
        onStartSession(config)
    }

    return (
        <Box gap={1} sx={{"& .MuiFormHelperText-root": {display: "none"}}}>
            <Stack>
                <SelectArea
                    tenantId={electionEvent.tenant_id}
                    electionEventId={electionEvent.id}
                    source="areaId"
                    label={t("electionEventScreen.ivr.emulator.area")}
                />
                <FormStyles.AutocompleteArrayInput
                    label={t("electionEventScreen.ivr.emulator.elections")}
                    choices={electionChoices}
                    source="electionIds"
                />
                <BooleanInput
                    label={t("electionEventScreen.ivr.emulator.blacklistCaller")}
                    source="blacklistCaller"
                />
                {resolvedBallotStyles.size < 1 ? (
                    <Alert severity="warning">
                        {t("electionEventScreen.ivr.emulator.noStylesFound")}
                    </Alert>
                ) : null}
            </Stack>
            <Button
                sx={{marginLeft: "auto"}}
                variant="contained"
                onClick={onSubmit}
                disabled={resolvedBallotStyles.size < 1}
            >
                {t("electionEventScreen.ivr.emulator.startSession")}
            </Button>
        </Box>
    )
}

const ConfigForm: React.FC<{
    electionEvent: Sequent_Backend_Election_Event
    onStartSession: (config: EmulatorConfig) => void
}> = ({electionEvent, onStartSession}) => {
    const methods = useForm<ConfigFormValues>({
        defaultValues: {areaId: "", electionIds: [], blacklistCaller: false},
    })

    return (
        <FormProvider {...methods}>
            <form>
                <ConfigFormBody electionEvent={electionEvent} onStartSession={onStartSession} />
            </form>
        </FormProvider>
    )
}

export const IvrEmulator: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {status: apiStatus, api} = useIvrEmulator()
    const [emulatorStatus, setEmulatorStatus] = useState<IvrCallStatus | undefined>(undefined)
    const [config, setConfig] = useState<EmulatorConfig | undefined>(undefined)

    const statusAlert = useMemo(() => {
        switch (apiStatus) {
            case IvrApiStatus.UNAVAILABLE:
                return (
                    <Alert severity="warning">
                        {t("electionEventScreen.ivr.emulator.apiStatus.unavailable")}
                    </Alert>
                )
            case IvrApiStatus.LOADING:
                return (
                    <Alert severity="info">
                        {t("electionEventScreen.ivr.emulator.apiStatus.loading")}
                    </Alert>
                )
            case IvrApiStatus.ERROR:
                return (
                    <Alert severity="error">
                        {t("electionEventScreen.ivr.emulator.apiStatus.error")}
                    </Alert>
                )
            case IvrApiStatus.READY:
                return null
        }
    }, [apiStatus])

    if (!record?.id) {
        return null
    }

    const onStartSession = (config: EmulatorConfig): void => {
        setEmulatorStatus("Ready")
        setConfig(config)
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <ElectionHeaderStyles.SubTitle>
                {t("electionEventScreen.ivr.emulator.infoMsg")}
            </ElectionHeaderStyles.SubTitle>

            <div>
                {statusAlert}
                {api ? (
                    <>
                        <Alert severity="info">
                            <AlertTitle>
                                {t("electionEventScreen.ivr.emulator.hints.title")}
                            </AlertTitle>
                            <Box component="ul">
                                <li>
                                    {t("electionEventScreen.ivr.emulator.hints.publishRequired")}
                                </li>
                                <li>
                                    {t(
                                        "electionEventScreen.ivr.emulator.hints.eventChangesImmediate"
                                    )}
                                </li>
                                <li>{t("electionEventScreen.ivr.emulator.hints.credentials")}</li>
                            </Box>
                        </Alert>
                        {record && !emulatorStatus ? (
                            <ConfigForm electionEvent={record} onStartSession={onStartSession} />
                        ) : null}
                        {api && config && emulatorStatus && record ? (
                            <div>
                                <IvrCall
                                    start={() => new api.IvrEmulatorDriver(config)}
                                    onStatusChange={(status) =>
                                        emulatorStatus && setEmulatorStatus(status)
                                    }
                                    placeholder={(expected) =>
                                        t("electionEventScreen.ivr.emulator.inputPlaceholder", {
                                            maxDigits: expected.max_digits,
                                            validInputs: expected.valid_inputs,
                                            timeout: expected.timeout,
                                        })
                                    }
                                    timeoutLabel={t("electionEventScreen.ivr.emulator.sendTimeout")}
                                    sendLabel={t("electionEventScreen.ivr.emulator.sendDtmf")}
                                    disconnectedLabel={t(
                                        "electionEventScreen.ivr.emulator.disconnected"
                                    )}
                                />

                                <Button
                                    sx={{marginLeft: "auto"}}
                                    variant="contained"
                                    onClick={() => setEmulatorStatus(undefined)}
                                >
                                    {t("electionEventScreen.ivr.emulator.endSession")}
                                </Button>
                            </div>
                        ) : null}
                    </>
                ) : null}
            </div>
        </Box>
    )
}

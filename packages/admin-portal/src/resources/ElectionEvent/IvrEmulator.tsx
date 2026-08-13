// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useRef, useState} from "react"
import {BooleanInput, useGetList, useRecordContext} from "react-admin"
import {FormProvider, useForm} from "react-hook-form"
import {Alert, AlertTitle, Box, Button, Stack, TextField} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    Sequent_Backend_Ballot_Style,
    Sequent_Backend_Election,
    Sequent_Backend_Election_Event,
} from "@/gql/graphql"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {IvrApiStatus, useIvrEmulator} from "@/providers/IvrEmulatorContextProvider"
import {v4} from "uuid"
import Paper from "@mui/material/Paper"
import {
    IvrEmulatorApi,
    Action,
    IvrEmulatorDriver,
    PromptInfo,
    EmulatorConfig,
} from "@/services/IvrEmulator"
import DialpadIcon from "@mui/icons-material/Dialpad"
import TimerIcon from "@mui/icons-material/Timer"
import {theme} from "@sequentech/ui-essentials"
import SelectArea from "@/components/area/SelectArea"
import {useFormContext, useWatch} from "react-hook-form"
import {useAliasRenderer} from "@/hooks/useAliasRenderer"
import {FormStyles} from "@/components/styles/FormStyles"

type ExpectedInput = Extract<Action, {type: "ExpectInput"}>
type Status = "Ready" | "Running" | "ExpectingInput" | "Disconnected"

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

const PromptLine: React.FC<{id: number; prompt: PromptInfo}> = ({id, prompt}) => {
    const promptBody = useMemo(() => {
        // Strip off the root ssml tag.
        return prompt.prompt_text.replace(/^<speak>/, "").replace(/<\/speak>$/, "")
    }, [prompt])
    const lang = useMemo(() => {
        return prompt.language.slice(0, 2).toUpperCase()
    }, [prompt])
    const langTitle = useMemo(() => {
        return `${prompt.language}, ${prompt.voice_id}`
    }, [prompt])

    return (
        <Box
            key={id}
            sx={{display: "grid", gridTemplateColumns: "3ch minmax(0, 1fr)", columnGap: 1}}
        >
            <Box
                title={langTitle}
                sx={{whiteSpace: "nowrap", borderRight: 1, borderColor: "divider", pr: 1}}
            >
                {lang}
            </Box>
            <Box sx={{minWidth: 0, whiteSpace: "pre-wrap", overflowWrap: "anywhere"}}>
                {promptBody}
            </Box>
        </Box>
    )
}

const EmulatorInterface: React.FC<{
    api: IvrEmulatorApi
    config: EmulatorConfig
    onStatusChange?: (status: Status) => void
}> = ({api, config, onStatusChange}) => {
    const {t} = useTranslation()
    const [prompts, setPrompts] = useState<[number, PromptInfo][]>([])
    const emulator = useRef<IvrEmulatorDriver | undefined>(undefined)
    const toDispose = useRef(new WeakSet<IvrEmulatorDriver>())
    const inFlight = useRef(new WeakSet<IvrEmulatorDriver>())
    const [expectedInput, setExpectedInput] = useState<ExpectedInput | undefined>()
    const [status, setStatus] = useState<Status>("Disconnected")
    const [error, setError] = useState<string>("")
    const [input, setInput] = useState<string>("")
    const nextLogId = useRef(0)

    const addPrompt = (prompt: PromptInfo) => {
        let id = nextLogId.current++
        setPrompts((current) => [...current, [id, prompt]])
    }

    const canSendInput = useMemo<boolean>(
        () => status === "ExpectingInput" && Boolean(input.trim()),
        [input, status]
    )

    const changeStatus = (value: Status): void => {
        setStatus(value)
        onStatusChange?.(value)
    }

    const startSession = (config: EmulatorConfig): void => {
        if (emulator.current) {
            console.warn("Refusing to start another emulator session while there's one running")
            return
        }

        try {
            console.log("Creating a new session with config", config)
            emulator.current = new api.IvrEmulatorDriver(config)
            runEmulator(emulator.current)
        } catch (e) {
            console.error("Failed to create the emulator", e)
        }
    }

    const sendTimeout = () => {
        const current = emulator.current
        if (!current) {
            console.warn("Attempted to sendTimeout without an active emulator")
            return
        }
        if (inFlight.current.has(current)) {
            return
        }

        current.send_timeout()
        runEmulator(current)
    }

    const sendInput = () => {
        const current = emulator.current
        if (!current) {
            console.warn("Attempted to sendInput without an active emulator")
            return
        }
        if (inFlight.current.has(current)) {
            return
        }

        current.send_input(input)
        runEmulator(current)
        setInput("")
    }

    const releaseDisposed = (disposed: IvrEmulatorDriver): void => {
        if (!toDispose.current.has(disposed) || inFlight.current.has(disposed)) {
            return
        }
        console.log("Releasing disposed emulator")
        toDispose.current.delete(disposed)
        disposed.free()
    }

    // disposeEmulator must not be called again after the emulator has been released.
    const disposeEmulator = (disposed: IvrEmulatorDriver): void => {
        console.debug("Disposing emulator")
        if (emulator.current === disposed) {
            emulator.current = undefined
            console.debug("Cleared current emulator")
        }
        toDispose.current.add(disposed)
        releaseDisposed(disposed)
    }

    const executeEmulatorLoop = async (emulator: IvrEmulatorDriver) => {
        while (true) {
            changeStatus("Running")
            const action = await emulator.execute(true)

            // If disposal was requested while we were executing the wasm,
            //  such as the component being unmounted, stop working
            //  and allow releasing the emulator with its normal flow (the finally block).
            if (toDispose.current.has(emulator)) {
                return
            }
            switch (action.type) {
                case "Prompt":
                    addPrompt(action.prompt)
                    break
                case "Noop":
                    break
                case "ExpectInput":
                    addPrompt(action.prompt)
                    setExpectedInput(action)
                    changeStatus("ExpectingInput")
                    return
                case "Disconnect":
                    addPrompt(action.prompt)
                    changeStatus("Disconnected")
                    disposeEmulator(emulator)
                    return
            }
        }
    }

    const runEmulator = (emulator: IvrEmulatorDriver) => {
        if (inFlight.current.has(emulator)) {
            return
        }

        inFlight.current.add(emulator)
        executeEmulatorLoop(emulator)
            .catch((e) => {
                console.error("Failed to execute the emulator", e)
                if (!toDispose.current.has(emulator)) {
                    setError(`${e}`)
                    disposeEmulator(emulator)
                }
            })
            .finally(() => {
                inFlight.current.delete(emulator)
                releaseDisposed(emulator)
            })
    }

    useEffect(() => {
        setStatus("Ready")
        startSession(config)

        return () => {
            if (emulator.current) {
                disposeEmulator(emulator.current)
            }
        }
    }, [])

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 1}}>
            {error ? <Alert severity="error">{error}</Alert> : null}

            <Paper variant="outlined" sx={{p: theme.spacing(1), fontFamily: "monospace"}}>
                {prompts.map(([id, prompt]) => (
                    <PromptLine id={id} prompt={prompt} />
                ))}
            </Paper>

            {status !== "Disconnected" ? (
                <Box sx={{display: "flex", gap: 1}}>
                    <form
                        style={{width: "100%"}}
                        onSubmit={(e) => {
                            e.preventDefault()
                            canSendInput && sendInput()
                        }}
                    >
                        <TextField
                            value={input}
                            onChange={(e) => setInput(e.target.value.replace(/[^0-9*#]/g, ""))}
                            slotProps={{
                                htmlInput: {
                                    pattern: "[0-9*#]*",
                                    maxLength: expectedInput?.max_digits,
                                },
                            }}
                            disabled={!expectedInput}
                            autoFocus
                            sx={{fontFamily: "monospace"}}
                            placeholder={t("electionEventScreen.ivr.emulator.inputPlaceholder", {
                                maxDigits: expectedInput?.max_digits ?? "",
                                validInputs: expectedInput?.valid_inputs ?? "",
                                timeout: expectedInput?.timeout ?? 0,
                            })}
                        />
                    </form>
                    <div
                        style={{
                            display: "flex",
                            flexDirection: "row",
                            gap: theme.spacing(1),
                            padding: `${theme.spacing(2)} 0px ${theme.spacing(2)} 0px`,
                        }}
                    >
                        <Button
                            title={t("electionEventScreen.ivr.emulator.sendTimeout")}
                            onClick={sendTimeout}
                            disabled={status !== "ExpectingInput"}
                        >
                            <TimerIcon />
                        </Button>
                        <Button
                            title={t("electionEventScreen.ivr.emulator.sendDtmf")}
                            variant="outlined"
                            onClick={sendInput}
                            disabled={!canSendInput}
                        >
                            <DialpadIcon />
                        </Button>
                    </div>
                </Box>
            ) : null}

            {status === "Disconnected" ? (
                <ElectionHeaderStyles.SubTitle sx={{fontStyle: "italic"}}>
                    {t("electionEventScreen.ivr.emulator.disconnected")}
                </ElectionHeaderStyles.SubTitle>
            ) : null}
        </Box>
    )
}

export const IvrEmulator: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {status: apiStatus, api} = useIvrEmulator()
    const [emulatorStatus, setEmulatorStatus] = useState<Status | undefined>(undefined)
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
                                <EmulatorInterface
                                    api={api}
                                    config={config}
                                    onStatusChange={(status) =>
                                        emulatorStatus && setEmulatorStatus(status)
                                    }
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

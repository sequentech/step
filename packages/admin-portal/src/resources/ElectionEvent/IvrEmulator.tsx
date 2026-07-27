// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useEffect, useMemo, useRef, useState} from "react"
import {BooleanInput, useGetList, useRecordContext} from "react-admin"
import {FormProvider, useForm} from "react-hook-form"
import {Alert, Box, Button, Stack, TextField} from "@mui/material"
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
            sort: {field: "created_at", order: "ASC"},
            filter: {
                "tenant_id": electionEvent.tenant_id,
                "election_event_id": electionEvent.id,
                "area_id": areaId,
                "deleted_at@_is_null": null,
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
                    style.ballot_eml
                ) {
                    return [[style.election_id, style.ballot_eml]]
                } else {
                    return []
                }
            }) ?? []
        )
    }, [rawBallotStyles])

    const {data: rawElections} = useGetList<Sequent_Backend_Election>(
        "sequent_backend_election",
        {
            pagination: {page: 1, perPage: 300},
            sort: {field: "name", order: "DESC"},
            filter: {
                "tenant_id": electionEvent.tenant_id,
                "election_event_id": electionEvent.id,
                "id@_in": Array.from(availableBallotStyles.keys()),
            },
        },
        {
            enabled: availableBallotStyles.size > 0,
        }
    )

    const electionChoices = useMemo<{id: string; name: string}[]>(
        () =>
            rawElections
                ?.map((e) => ({
                    id: e.id,
                    name: aliasRenderer(e),
                }))
                .sort((a, b) => a.name.localeCompare(b.name)) ?? [],
        [rawElections]
    )

    const selectedBallotStyles = useMemo<Map<string, string>>(() => {
        let filtered = availableBallotStyles
            .entries()
            .filter(([electionId, _style]) => electionIds.includes(electionId))
        return new Map(filtered)
    }, [availableBallotStyles, electionIds])

    const generateConfig = (data: ConfigFormValues): EmulatorConfig => {
        return {
            tenant_id: electionEvent.tenant_id,
            election_event_id: electionEvent.id,
            caller_number: CALLER_NUMBER,
            contact_id: generateContactId(),
            blacklisted_numbers: data.blacklistCaller ? [CALLER_NUMBER] : [],
            open_elections: Array.from(selectedBallotStyles.keys()),
            election_event: JSON.stringify(electionEvent),
            ballot_styles: Array.from(selectedBallotStyles.values()),
        }
    }

    const onSubmit = () => {
        let config = generateConfig(getValues())
        onStartSession(config)
    }

    return (
        <Box gap={1}>
            <Stack gap={0}>
                <SelectArea
                    tenantId={electionEvent.tenant_id}
                    electionEventId={electionEvent.id}
                    source="areaId"
                    label="Area"
                />
                <FormStyles.AutocompleteArrayInput
                    label="Elections"
                    choices={electionChoices}
                    source="electionIds"
                />
                <BooleanInput label="Blacklist caller" source="blacklistCaller" />
                Resolved {selectedBallotStyles.size} ballot styles for the selected area and
                elections
            </Stack>
            <Button variant="contained" onClick={onSubmit} disabled={selectedBallotStyles.size < 1}>
                Start new session
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

const EmulatorInterface: React.FC<{
    api: IvrEmulatorApi
    config: EmulatorConfig
    onStatusChange?: (status: Status) => void
}> = ({api, config, onStatusChange}) => {
    const [prompts, setPrompts] = useState<PromptInfo[]>([])
    const emulator = useRef<IvrEmulatorDriver | undefined>(undefined)
    const [expectedInput, setExpectedInput] = useState<ExpectedInput | undefined>()
    const addPrompt = (prompt: PromptInfo) => {
        setPrompts([...prompts, prompt])
    }
    const [status, setStatus] = useState<Status>("Disconnected")
    const [error, setError] = useState<string>("")
    const [input, setInput] = useState<string>("")
    const [executing, setExecuting] = useState<boolean>(false)

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

    const sendTimeout = async () => {
        if (!emulator.current) {
            console.warn("Attempted to sendTimeout without an active emulator")
            return
        }
        emulator.current.send_timeout()
        runEmulator(emulator.current)
    }

    const sendInput = async () => {
        if (!emulator.current) {
            console.warn("Attempted to sendInput without an active emulator")
            return
        }
        emulator.current.send_input(input)
        runEmulator(emulator.current)
    }

    const disposeEmulator = (disposed: IvrEmulatorDriver): void => {
        console.info("Disposing the emulator")
        if (emulator.current === disposed) {
            emulator.current = undefined
            console.info("Cleared current emulator")
        }
        disposed.free()
    }

    const executeEmulatorLoop = async (emulator: IvrEmulatorDriver) => {
        while (true) {
            changeStatus("Running")
            const action = await emulator.execute(true)
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
        if (executing) {
            return
        }

        setExecuting(true)
        executeEmulatorLoop(emulator)
            .catch((e) => {
                console.error("Failed to execute the emulator", e)
                setError(`${e}`)
            })
            .finally(() => setExecuting(false))
    }

    useEffect(() => {
        setStatus("Ready")
        startSession(config)
    }, [])

    return (
        <Box sx={{display: "flex-col", gap: 1}}>
            {error ? <Alert severity="error">{error}</Alert> : null}
            {status === "Disconnected" ? <Alert severity="info">Disconnected</Alert> : null}

            <Paper variant="outlined">
                {prompts.map((prompt) => (
                    <Box component="div" sx={{fontFamily: "monospace"}}>
                        {prompt.prompt_text},{prompt.language},{prompt.voice_id}
                    </Box>
                ))}
            </Paper>

            <Box sx={{display: "flex", gap: 1}}>
                <TextField
                    value={input}
                    onChange={(e) => setInput(e.target.value.replace(/[^0-9*#]/g, ""))}
                    slotProps={{
                        htmlInput: {pattern: "[0-9*#]*", maxLength: expectedInput?.max_digits},
                    }}
                    onKeyDown={sendInput}
                    disabled={!canSendInput}
                    autoFocus
                    sx={{fontFamily: "monospace"}}
                    placeholder={`Enter your input (max digis=${expectedInput?.max_digits ?? ""}, valid inputs=${expectedInput?.valid_inputs ?? ""}, timeout=${expectedInput?.timeout ?? 0}s)`}
                />
                <div
                    style={{
                        display: "flex",
                        flexDirection: "row",
                        gap: theme.spacing(1),
                        padding: `${theme.spacing(2)} 0px ${theme.spacing(2)} 0px`,
                    }}
                >
                    <Button onClick={sendTimeout} disabled={status !== "ExpectingInput"}>
                        <TimerIcon />
                    </Button>
                    <Button variant="outlined" onClick={sendInput} disabled={!canSendInput}>
                        <DialpadIcon />
                    </Button>
                </div>
            </Box>
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
                        The emulator system is not available in your environment
                    </Alert>
                )
            case IvrApiStatus.LOADING:
                return <Alert severity="info">Loading the emulator system...</Alert>
            case IvrApiStatus.ERROR:
                return <Alert severity="error">Error loading the emulator</Alert>
            case IvrApiStatus.READY:
                return <Alert severity="success">The emulator system is loaded and ready</Alert>
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
                Select an area and desired elections to experience the IVR session.
            </ElectionHeaderStyles.SubTitle>

            <div>
                {statusAlert}
                {record && !emulatorStatus ? (
                    <ConfigForm electionEvent={record} onStartSession={onStartSession} />
                ) : null}
                {api && config ? (
                    <div>
                        <EmulatorInterface
                            api={api}
                            config={config}
                            onStatusChange={setEmulatorStatus}
                        />

                        <Button variant="contained" onClick={() => setEmulatorStatus(undefined)}>
                            Close
                        </Button>
                    </div>
                ) : null}
            </div>
        </Box>
    )
}

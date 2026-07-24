// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useMemo, useRef, useState} from "react"
import {useRecordContext} from "react-admin"
import {Alert, Box, Button, TextField} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ElectionHeaderStyles} from "@/components/styles/ElectionHeaderStyles"
import {IvrApiStatus, useIvrEmulator} from "@/providers/IvrEmulatorContextProvider"
import {v4} from "uuid"
import Paper from "@mui/material/Paper"
import {IvrEmulatorApi, Action, IvrEmulatorDriver, PromptInfo} from "@/services/IvrEmulator"
import DialpadIcon from "@mui/icons-material/Dialpad"
import TimerIcon from "@mui/icons-material/Timer"
import {theme} from "@sequentech/ui-essentials"

type ExpectedInput = Extract<Action, {type: "ExpectInput"}>
type Status = "Ready" | "Running" | "ExpectingInput" | "Disconnected"

const newContactId = (): string => {
    return v4()
}

const EmulatorInterface: React.FC<{api: IvrEmulatorApi; onDisconnect?: () => void}> = ({
    api,
    onDisconnect,
}) => {
    const emulator = useRef<IvrEmulatorDriver | undefined>(undefined)
    const [prompts, setPrompts] = useState<PromptInfo[]>([])
    const [expectedInput, setExpectedInput] = useState<ExpectedInput | undefined>()
    const addPrompt = (prompt: PromptInfo) => {
        setPrompts([...prompts, prompt])
    }
    const [status, setStatus] = useState<Status>("Disconnected")
    const [error, setError] = useState<string>("")
    const [input, setInput] = useState<string>("")
    const [executing, setExecuting] = useState<boolean>(false)

    const startSession = () => {
        if (emulator.current) {
            console.warn("Attempted to start another emulator session while there's one running")
            return
        }
        try {
            emulator.current = new api.IvrEmulatorDriver("+123", newContactId())
            runEmulator(emulator.current)
            setStatus("Ready")
        } catch (e) {
            console.error("Failed to create the emulator", e)
        }
    }

    const disposeEmulator = (localEmulator: IvrEmulatorDriver) => {
        if (localEmulator === emulator.current) {
            emulator.current = undefined
        }
        localEmulator.free()
    }

    const canSendInput = useMemo<boolean>(() => {
        return status === "ExpectingInput" && Boolean(input.trim())
    }, [input, status])

    const sendTimeout = async () => {
        if (emulator.current) {
            emulator.current.send_timeout()
            runEmulator(emulator.current)
        } else {
            console.warn("Attempted to sendTimeout while emulator is undefined")
        }
    }

    const sendInput = async () => {
        if (emulator.current) {
            emulator.current.send_input(input)
            runEmulator(emulator.current)
        } else {
            console.warn("Attempted to sendInput while emulator is undefined")
        }
    }

    const executeEmulatorLoop = async (emulator: IvrEmulatorDriver) => {
        while (true) {
            setStatus("Running")
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
                    setStatus("ExpectingInput")
                    return
                case "Disconnect":
                    addPrompt(action.prompt)
                    setStatus("Disconnected")
                    disposeEmulator(emulator)
                    onDisconnect?.()
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
                    placeholder={`Enter your input (max digis=${expectedInput?.max_digits}, valid inputs=${expectedInput?.valid_inputs}, timeout=${expectedInput?.timeout}s)`}
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

            {status === "Disconnected" ? (
                <Button variant="contained" onClick={startSession}>
                    Start new session
                </Button>
            ) : null}
        </Box>
    )
}

export const IvrEmulator: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const {status: apiStatus, api} = useIvrEmulator()

    const statusMsg = useMemo(() => {
        switch (apiStatus) {
            case IvrApiStatus.UNAVAILABLE:
                return "The IVR emulator is not available in your environment"
            case IvrApiStatus.LOADING:
                return "Loading..."
            case IvrApiStatus.ERROR:
                return "Error loading the IVR emulator"
            case IvrApiStatus.READY:
                return "The emulator is ready"
        }
    }, [apiStatus])

    if (!record?.id) {
        return null
    }

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 2}}>
            <ElectionHeaderStyles.SubTitle>
                {t("electionEventScreen.ivr.config.infoMsg")}
            </ElectionHeaderStyles.SubTitle>

            <div>`{statusMsg}`</div>
            {api ? <EmulatorInterface api={api} /> : null}
        </Box>
    )
}

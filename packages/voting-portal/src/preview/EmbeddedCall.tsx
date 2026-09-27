// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useCallback, useEffect, useState} from "react"
import {Alert, Box, ThemeProvider, Typography} from "@mui/material"
import {
    IvrCall,
    IvrEmulatorError,
    loadIvrEmulator,
    theme,
    type IvrCallStatus,
    type IvrEmulatorApi,
} from "@sequentech/ui-essentials"
import {
    EmbedMessageType,
    fillLabel,
    type CallLabels,
    type CallRequest,
    type EmbedReply,
} from "./embed"

/** What a call says when the framing tool sends no words of its own. */
export const DEFAULT_CALL_LABELS: Required<CallLabels> = {
    input: "Keys to press",
    placeholder: "Up to {{maxDigits}} of {{validInputs}}, within {{timeout}}s",
    timeout: "Say nothing",
    send: "Press these keys",
    disconnected: "The call ended.",
    connecting: "Connecting to the telephone emulator…",
}

type Loaded =
    | {state: "loading"}
    | {state: "ready"; api: IvrEmulatorApi}
    | {state: "absent"}
    | {state: "broken"; why: string}

export interface EmbeddedCallProps {
    request: CallRequest
    reply: (message: EmbedReply) => void
    /** Fetches and starts the emulator; `loadIvrEmulator` by default, a fake in tests. */
    load?: (url: string) => Promise<IvrEmulatorApi>
}

/**
 * A telephone call, placed from another tool: the IVR emulator the framing window names,
 * given the configuration it sends, through the same `IvrCall` the Admin Portal uses.
 *
 * A fetch that fails is `absent` rather than a failure: the emulator is a separate build
 * that a deployment serves, and a framing tool opened without one is an ordinary case.
 */
export const EmbeddedCall: React.FC<EmbeddedCallProps> = ({
    request,
    reply,
    load = loadIvrEmulator,
}) => {
    const [loaded, setLoaded] = useState<Loaded>({state: "loading"})
    const labels = {...DEFAULT_CALL_LABELS, ...request.labels}

    useEffect(() => {
        let current = true
        setLoaded({state: "loading"})
        reply({type: EmbedMessageType.CALLING, status: "loading"})
        load(request.emulatorUrl).then(
            (api) => {
                if (current) setLoaded({state: "ready", api})
            },
            (error: unknown) => {
                if (!current) return
                if (error instanceof IvrEmulatorError && error.operation === "fetch") {
                    setLoaded({state: "absent"})
                    reply({type: EmbedMessageType.CALLING, status: "absent"})
                } else {
                    const why = error instanceof Error ? error.message : String(error)
                    setLoaded({state: "broken", why})
                    reply({type: EmbedMessageType.FAILED, issues: [why]})
                }
            }
        )
        return () => {
            current = false
        }
    }, [request, load, reply])

    const onStatusChange = useCallback(
        (status: IvrCallStatus) => reply({type: EmbedMessageType.CALLING, status}),
        [reply]
    )

    const api = loaded.state === "ready" ? loaded.api : undefined
    return (
        <ThemeProvider theme={theme}>
            <Box className="embedded-call" sx={{p: 2}}>
                {loaded.state === "loading" ? (
                    <Typography role="status" color="text.secondary">
                        {labels.connecting}
                    </Typography>
                ) : null}
                {loaded.state === "absent" ? (
                    <Typography role="status" color="text.secondary">
                        No telephone emulator is served at {request.emulatorUrl}.
                    </Typography>
                ) : null}
                {loaded.state === "broken" ? <Alert severity="error">{loaded.why}</Alert> : null}
                {api ? (
                    <IvrCall
                        start={() => new api.IvrEmulatorDriver(request.config)}
                        onStatusChange={onStatusChange}
                        placeholder={(expected) =>
                            fillLabel(labels.placeholder, {
                                maxDigits: expected.max_digits,
                                validInputs: expected.valid_inputs,
                                timeout: expected.timeout,
                            })
                        }
                        inputLabel={labels.input}
                        timeoutLabel={labels.timeout}
                        sendLabel={labels.send}
                        disconnectedLabel={labels.disconnected}
                    />
                ) : null}
            </Box>
        </ThemeProvider>
    )
}

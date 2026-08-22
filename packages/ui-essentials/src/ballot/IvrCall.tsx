// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import DialpadIcon from "@mui/icons-material/Dialpad"
import TimerIcon from "@mui/icons-material/Timer"
import Alert from "@mui/material/Alert"
import Box from "@mui/material/Box"
import Button from "@mui/material/Button"
import Paper from "@mui/material/Paper"
import TextField from "@mui/material/TextField"
import Typography from "@mui/material/Typography"
import React, {useEffect, useMemo, useRef, useState} from "react"

import {theme} from "../services/theme"

/**
 * One thing said down the line, as the emulator reports it.
 *
 * Structural, not the WebAssembly's generated `PromptInfo`. The generated
 * declaration is a build product of the IVR Lambda that lands in whichever app
 * fetched it, and a library that imported it could only be used by that one app.
 * These four shapes are the whole contract, and a driver satisfies them by having
 * the right fields rather than by importing anything from here.
 */
export interface IvrPrompt {
    prompt_text: string
    language: string
    voice_id: string
}

export interface IvrExpectedInput {
    prompt: IvrPrompt
    /** Which keys the caller may press, as the Lambda words it. */
    valid_inputs: string
    max_digits: number
    /** Seconds the real call would wait before giving up on the caller. */
    timeout: number
}

export type IvrAction =
    | {type: "Prompt"; prompt: IvrPrompt}
    | ({type: "ExpectInput"} & IvrExpectedInput)
    | {type: "Disconnect"; prompt: IvrPrompt}
    | {type: "Noop"}

/**
 * One call in progress.
 *
 * Satisfied by the emulator WebAssembly's `IvrEmulatorDriver` as generated, which
 * is why the names are snake_case: this describes something that exists rather
 * than proposing an interface for it to adopt.
 */
export interface IvrCallDriver {
    execute(untilIo: boolean): Promise<IvrAction>
    send_input(input: string): void
    send_timeout(): void
    free(): void
}

export type IvrCallStatus = "Ready" | "Running" | "ExpectingInput" | "Disconnected"

export interface IIvrCallProps {
    /**
     * Begin a call, and hand back the thing driving it.
     *
     * A factory rather than a driver, because the driver owns WebAssembly memory
     * and this component owns when that memory is freed. Handing one in would
     * leave two owners for one `free()`, and the second call is a crash rather
     * than a leak.
     */
    start: () => IvrCallDriver
    onStatusChange?: (status: IvrCallStatus) => void
    /** What to say in the box, given what the call is waiting for. */
    placeholder?: (expected: IvrExpectedInput) => string
    /** The button that lets the caller's patience run out. */
    timeoutLabel?: string
    /** The button that presses the keys. */
    sendLabel?: string
    /** The line under a call that has ended. */
    disconnectedLabel?: string
}

/**
 * One line of the transcript.
 *
 * The language goes in a fixed three-character gutter so the prompts line up down
 * the left, and its `title` carries the voice as well — which voice read a prompt
 * is the answer to "why does this sound wrong", and it is worth having without
 * spending a column on it.
 */
export const IvrPromptLine: React.FC<{prompt: IvrPrompt}> = ({prompt}) => {
    // The Lambda wraps every prompt in SSML. Stripping only the root tag is
    // deliberate: what is left says `<break time="500ms"/>` where the call pauses,
    // and somebody reading a transcript to work out why a prompt sounds rushed
    // needs to see that.
    const body = useMemo(
        () => prompt.prompt_text.replace(/^<speak>/, "").replace(/<\/speak>$/, ""),
        [prompt]
    )

    return (
        <Box sx={{display: "grid", gridTemplateColumns: "3ch minmax(0, 1fr)", columnGap: 1}}>
            <Box
                title={`${prompt.language}, ${prompt.voice_id}`}
                sx={{whiteSpace: "nowrap", borderRight: 1, borderColor: "divider", pr: 1}}
            >
                {prompt.language.slice(0, 2).toUpperCase()}
            </Box>
            <Box sx={{minWidth: 0, whiteSpace: "pre-wrap", overflowWrap: "anywhere"}}>{body}</Box>
        </Box>
    )
}

/**
 * A telephone call, run against the emulator and shown as it happens.
 *
 * Lifted out of the Admin Portal's `IvrEmulator`, which is where it was written and
 * where it stays in use. What made it worth lifting is that the Election Architect
 * needs the same thing — somebody configuring a call flow should be able to hear it
 * out loud before an election runs on it — and the alternative was a second copy of
 * the loop below.
 *
 * **The loop is the part that had to move.** Rendering a transcript is easy; what is
 * not is that the driver holds WebAssembly memory, `execute` is asynchronous, and
 * React unmounts components in the middle of things. So a driver being executed is
 * marked in flight, a driver asked to go away is marked for disposal, and `free()`
 * happens only where both are known — which is why disposal is two `WeakSet`s and
 * not a boolean. Every one of those states is reachable by closing a dialog at the
 * wrong moment, and getting it wrong is a page that crashes in compiled code.
 *
 * The host supplies the words. This component knows the shape of a call and nothing
 * about which product is showing it, so its two apps' translations stay in their own
 * files rather than one of them importing the other's.
 */
export const IvrCall: React.FC<IIvrCallProps> = ({
    start,
    onStatusChange,
    placeholder,
    timeoutLabel,
    sendLabel,
    disconnectedLabel,
}) => {
    const [prompts, setPrompts] = useState<Array<[number, IvrPrompt]>>([])
    const driver = useRef<IvrCallDriver | undefined>(undefined)
    const toDispose = useRef(new WeakSet<IvrCallDriver>())
    const inFlight = useRef(new WeakSet<IvrCallDriver>())
    const [expected, setExpected] = useState<IvrExpectedInput | undefined>()
    const [status, setStatus] = useState<IvrCallStatus>("Disconnected")
    const [error, setError] = useState<string>("")
    const [input, setInput] = useState<string>("")
    const nextLineId = useRef(0)

    const addPrompt = (prompt: IvrPrompt): void => {
        const id = nextLineId.current++
        setPrompts((current) => [...current, [id, prompt]])
    }

    const canSend = useMemo<boolean>(
        () => status === "ExpectingInput" && Boolean(input.trim()),
        [input, status]
    )

    const changeStatus = (value: IvrCallStatus): void => {
        setStatus(value)
        onStatusChange?.(value)
    }

    const releaseDisposed = (disposed: IvrCallDriver): void => {
        if (!toDispose.current.has(disposed) || inFlight.current.has(disposed)) {
            return
        }
        toDispose.current.delete(disposed)
        disposed.free()
    }

    // Must not be called again once the driver has been released.
    const dispose = (disposed: IvrCallDriver): void => {
        if (driver.current === disposed) {
            driver.current = undefined
        }
        toDispose.current.add(disposed)
        releaseDisposed(disposed)
    }

    const loop = async (current: IvrCallDriver): Promise<void> => {
        for (;;) {
            changeStatus("Running")
            const action = await current.execute(true)

            // Disposal may have been asked for while the WebAssembly was running —
            // the component unmounting, most often. Stop, and let the `finally`
            // below release it through the normal path.
            if (toDispose.current.has(current)) {
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
                    setExpected(action)
                    changeStatus("ExpectingInput")
                    return
                case "Disconnect":
                    addPrompt(action.prompt)
                    changeStatus("Disconnected")
                    dispose(current)
                    return
            }
        }
    }

    const run = (current: IvrCallDriver): void => {
        if (inFlight.current.has(current)) {
            return
        }

        inFlight.current.add(current)
        loop(current)
            .catch((e) => {
                console.error("Failed to execute the emulator", e)
                if (!toDispose.current.has(current)) {
                    setError(`${e}`)
                    dispose(current)
                }
            })
            .finally(() => {
                inFlight.current.delete(current)
                releaseDisposed(current)
            })
    }

    const sendTimeout = (): void => {
        const current = driver.current
        if (!current || inFlight.current.has(current)) {
            return
        }
        current.send_timeout()
        run(current)
    }

    const sendInput = (): void => {
        const current = driver.current
        if (!current || inFlight.current.has(current)) {
            return
        }
        current.send_input(input)
        run(current)
        setInput("")
    }

    useEffect(() => {
        setStatus("Ready")
        try {
            driver.current = start()
        } catch (e) {
            console.error("Failed to create the emulator", e)
            setError(`${e}`)
            return
        }
        run(driver.current)

        return () => {
            if (driver.current) {
                dispose(driver.current)
            }
        }
    }, [])

    return (
        <Box sx={{display: "flex", flexDirection: "column", gap: 1}} data-testid="ivr-call">
            {error ? <Alert severity="error">{error}</Alert> : null}

            <Paper variant="outlined" sx={{p: theme.spacing(1), fontFamily: "monospace"}}>
                {prompts.map(([id, prompt]) => (
                    <IvrPromptLine key={id} prompt={prompt} />
                ))}
            </Paper>

            {status !== "Disconnected" ? (
                <Box sx={{display: "flex", gap: 1}}>
                    <form
                        style={{width: "100%"}}
                        onSubmit={(e) => {
                            e.preventDefault()
                            if (canSend) {
                                sendInput()
                            }
                        }}
                    >
                        <TextField
                            value={input}
                            // A telephone keypad has twelve keys and no others, so
                            // anything else is dropped as it is typed rather than
                            // reported afterwards.
                            onChange={(e) => setInput(e.target.value.replace(/[^0-9*#]/g, ""))}
                            slotProps={{
                                htmlInput: {
                                    "pattern": "[0-9*#]*",
                                    "maxLength": expected?.max_digits,
                                    "data-testid": "ivr-call-input",
                                },
                            }}
                            disabled={!expected}
                            autoFocus
                            sx={{fontFamily: "monospace"}}
                            placeholder={
                                expected && placeholder ? placeholder(expected) : undefined
                            }
                        />
                    </form>
                    <Box
                        sx={{
                            display: "flex",
                            flexDirection: "row",
                            gap: 1,
                            padding: `${theme.spacing(2)} 0px`,
                        }}
                    >
                        <Button
                            title={timeoutLabel}
                            aria-label={timeoutLabel}
                            onClick={sendTimeout}
                            disabled={status !== "ExpectingInput"}
                        >
                            <TimerIcon />
                        </Button>
                        <Button
                            title={sendLabel}
                            aria-label={sendLabel}
                            variant="outlined"
                            onClick={sendInput}
                            disabled={!canSend}
                        >
                            <DialpadIcon />
                        </Button>
                    </Box>
                </Box>
            ) : null}

            {status === "Disconnected" && disconnectedLabel ? (
                <Typography variant="body2" sx={{fontStyle: "italic"}}>
                    {disconnectedLabel}
                </Typography>
            ) : null}
        </Box>
    )
}

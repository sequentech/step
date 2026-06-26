// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {createContext, useCallback, useContext, useState} from "react"
import {
    SessionHandle,
    IEmulatorOutputLine,
    EEmulatorSessionStatus,
    ENextAction,
} from "@/types/emulator"
import {EmulatorService} from "@/services/EmulatorService"

interface EmulatorContextProps {
    status: EEmulatorSessionStatus
    outputLines: IEmulatorOutputLine[]
    errorMessage: string | null
    startSession: (electionEvent: unknown, ballotStyle: unknown) => void
    sendInput: (input: string) => void
    resetSession: () => void
}

const defaultEmulatorContext: EmulatorContextProps = {
    status: EEmulatorSessionStatus.IDLE,
    outputLines: [],
    errorMessage: null,
    startSession: () => undefined,
    sendInput: () => undefined,
    resetSession: () => undefined,
}

export const EmulatorContext = createContext<EmulatorContextProps>(defaultEmulatorContext)

interface EmulatorContextProviderProps {
    children: React.ReactNode
}

export const EmulatorContextProvider = ({children}: EmulatorContextProviderProps) => {
    const [status, setStatus] = useState<EEmulatorSessionStatus>(EEmulatorSessionStatus.IDLE)
    const [sessionHandle, setSessionHandle] = useState<SessionHandle | null>(null)
    const [outputLines, setOutputLines] = useState<IEmulatorOutputLine[]>([])
    const [errorMessage, setErrorMessage] = useState<string | null>(null)

    const appendLines = useCallback((lines: string[]) => {
        const now = Date.now()
        const newLines: IEmulatorOutputLine[] = lines.map((text, index) => ({
            text,
            timestamp: now + index,
        }))
        setOutputLines((prev) => [...prev, ...newLines])
    }, [])

    const startSession = useCallback((electionEvent: unknown, ballotStyle: unknown) => {
        setStatus(EEmulatorSessionStatus.INITIALIZING)
        setOutputLines([])
        setErrorMessage(null)
        setSessionHandle(null)

        try {
            const result = EmulatorService.init(electionEvent, ballotStyle)
            setSessionHandle(result.sessionHandle)
            appendLines(result.lines)

            if (result.nextAction === ENextAction.EXPECT_INPUT) {
                setStatus(EEmulatorSessionStatus.AWAITING_INPUT)
            } else {
                setStatus(EEmulatorSessionStatus.DISCONNECTED)
            }
        } catch (err) {
            const message = err instanceof Error ? err.message : String(err)
            setErrorMessage(message)
            setStatus(EEmulatorSessionStatus.ERROR)
        }
    }, [appendLines])

    const sendInput = useCallback(
        (input: string) => {
            if (!sessionHandle) {
                return
            }

            // Echo the user input into the output log
            appendLines([`> ${input}`])
            setStatus(EEmulatorSessionStatus.PROCESSING)

            try {
                const result = EmulatorService.sendInput(sessionHandle, input)
                appendLines(result.lines)

                if (result.nextAction === ENextAction.EXPECT_INPUT) {
                    setStatus(EEmulatorSessionStatus.AWAITING_INPUT)
                } else {
                    setStatus(EEmulatorSessionStatus.DISCONNECTED)
                    setSessionHandle(null)
                }
            } catch (err) {
                const message = err instanceof Error ? err.message : String(err)
                setErrorMessage(message)
                setStatus(EEmulatorSessionStatus.ERROR)
            }
        },
        [sessionHandle, appendLines]
    )

    const resetSession = useCallback(() => {
        setStatus(EEmulatorSessionStatus.IDLE)
        setSessionHandle(null)
        setOutputLines([])
        setErrorMessage(null)
    }, [])

    return (
        <EmulatorContext.Provider
            value={{
                status,
                outputLines,
                errorMessage,
                startSession,
                sendInput,
                resetSession,
            }}
        >
            {children}
        </EmulatorContext.Provider>
    )
}

export const useEmulatorStore = () => useContext(EmulatorContext)

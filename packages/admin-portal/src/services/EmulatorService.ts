// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {SessionHandle, IEmulatorReply} from "@/types/emulator"

/**
 * Thin adapter around the WASM emulator module.
 *
 * Every method delegates to the global WASM bindings that are expected to be
 * available at runtime (loaded by the host page).  The interface is kept
 * intentionally narrow so the rest of the app never touches the WASM layer
 * directly.
 */

/* eslint-disable @typescript-eslint/no-explicit-any */
declare global {
    interface Window {
        emulatorWasm?: {
            init(electionEvent: any, ballotStyle: any): SessionHandle
            send_input(sessionHandle: SessionHandle, input: string): IEmulatorReply
        }
    }
}
/* eslint-enable @typescript-eslint/no-explicit-any */

export const EmulatorService = {
    /**
     * Initialise a new emulator session for the given election event and
     * ballot style.
     */
    init(electionEvent: unknown, ballotStyle: unknown): IEmulatorReply & {sessionHandle: SessionHandle} {
        const wasm = window.emulatorWasm
        if (!wasm) {
            throw new Error("Emulator WASM module is not loaded")
        }
        const sessionHandle = wasm.init(electionEvent, ballotStyle)
        // After init the emulator immediately produces its first batch of
        // output, so we call send_input with an empty string to retrieve it.
        const reply = wasm.send_input(sessionHandle, "")
        return {...reply, sessionHandle}
    },

    /**
     * Send user input to the running emulator session.
     */
    sendInput(sessionHandle: SessionHandle, input: string): IEmulatorReply {
        const wasm = window.emulatorWasm
        if (!wasm) {
            throw new Error("Emulator WASM module is not loaded")
        }
        return wasm.send_input(sessionHandle, input)
    },
}

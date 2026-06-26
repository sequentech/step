// SPDX-FileCopyrightText: 2025 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * Opaque handle returned by the WASM `init` call, identifying an emulator
 * session.
 */
export type SessionHandle = string

/**
 * Signals what the emulator expects after delivering its output lines.
 */
export enum ENextAction {
    /** The emulator is waiting for user input. */
    EXPECT_INPUT = "expect_input",
    /** The session has ended. */
    DISCONNECT = "disconnect",
}

/**
 * A single line of output produced by the emulator.
 */
export interface IEmulatorOutputLine {
    text: string
    timestamp: number
}

/**
 * The reply returned by the WASM `init` and `send_input` calls.
 */
export interface IEmulatorReply {
    /** Lines of text the emulator wants to display. */
    lines: string[]
    /** What the emulator expects next. */
    nextAction: ENextAction
}

/**
 * Overall status of the emulator session as seen by the UI.
 */
export enum EEmulatorSessionStatus {
    /** No session has been started yet. */
    IDLE = "idle",
    /** The WASM `init` call is in progress. */
    INITIALIZING = "initializing",
    /** The emulator is waiting for user input. */
    AWAITING_INPUT = "awaiting_input",
    /** A `send_input` call is being processed. */
    PROCESSING = "processing",
    /** The emulator signalled disconnect — session is over. */
    DISCONNECTED = "disconnected",
    /** An error occurred. */
    ERROR = "error",
}

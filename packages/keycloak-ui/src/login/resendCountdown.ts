// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The seconds the resend countdown (re)starts from on this render, or undefined to keep the one
 * already running. A send that just happened starts the full wait; a page shown again without a
 * send continues the last send's wait as the server counts it (`resendSecondsLeft`), so a reload or
 * Back never restarts it, and a countdown never runs past what the server allows.
 */
export function resendCountdownStart(
    fullSeconds: number,
    sendAttempted: boolean,
    serverSecondsLeft: number | undefined
): number | undefined {
    if (sendAttempted) {
        return fullSeconds
    }
    if (serverSecondsLeft === undefined || !Number.isFinite(serverSecondsLeft)) {
        return undefined
    }
    return Math.min(Math.max(serverSecondsLeft, 0), fullSeconds)
}

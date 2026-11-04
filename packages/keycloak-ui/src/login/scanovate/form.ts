// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

// The form actions the scanovate-authenticator reads from the capture pages.
export enum CaptureAction {
    // Both Scanovate pages are done: Keycloak has the results server to server.
    Capture = "capture",
    // Start over with new one-time tokens, without using up an attempt.
    Retry = "retry",
}

export const ACTION_PARAM = "action"

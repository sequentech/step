// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import type {TFunction} from "i18next"

/**
 * The backend gives every refused send, unavailable channel and failed
 * callback one reason string, shared by the executor, the log and the UI.
 * Known reasons are shown translated; any other is shown as the backend wrote
 * it, so a reason is never hidden.
 */
export const KNOWN_REASONS: Record<string, string> = {
    "Callback address not configured": "CALLBACK_NOT_CONFIGURED",
    "Unsupported provider: replace this account": "UNSUPPORTED_PROVIDER",
    "Enter the credentials again for the new address": "CREDENTIALS_FOLLOW_ENDPOINT",
    "No contact channel": "NO_CONTACT_CHANNEL",
    "Can't be retried: no stored template": "NOT_RETRYABLE_NO_TEMPLATE",
}

export const reasonText = (t: TFunction, reason: string | null | undefined): string => {
    const text = reason?.trim()
    if (!text) return ""
    const code = KNOWN_REASONS[text]
    return code ? String(t(`messaging.reason.${code}`)) : text
}

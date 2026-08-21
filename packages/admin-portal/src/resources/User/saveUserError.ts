// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {getGraphQLActionErrorReason} from "@/services/graphqlActionError"
import {getPasswordPolicyMessage} from "./editPasswordError"

export type TranslateMessage = (key: string, options?: Record<string, unknown>) => string

/**
 * Message shown when saving a voter fails: the reason the backend gave when
 * there is one, and the bare failure message when there is not.
 */
export const getSaveUserErrorMessage = (
    error: unknown,
    messageKey: string,
    reasonKey: string,
    t: TranslateMessage
): string => {
    // The voter editor submits passwords through the same `edit_user` action as
    // EditPassword, so a rejected password reads the same way in both places
    // rather than falling back to Harvest's untranslated text.
    const reason = getPasswordPolicyMessage(error, t) ?? getGraphQLActionErrorReason(error)

    return reason ? t(reasonKey, {reason}) : t(messageKey)
}

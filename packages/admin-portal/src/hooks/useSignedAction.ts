// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useCallback} from "react"
import {
    useSigningRequest,
    type IOpenSigningRequestOptions,
    type ISigningRequestContext,
} from "@/components/signing/SigningProvider"

/** The optional `signing_request` a protected action's route answers. */
export interface ISigningRequiredReply {
    signing_request?: {id: string} | null
}

/** The signing request a route answered instead of running the action, if it did. */
export const signingRequestOf = (reply: ISigningRequiredReply | null | undefined): string | null =>
    reply?.signing_request?.id ?? null

/** The signing panel, where a `SigningProvider` hosts one. */
export const useOptionalSigningRequest = (): ISigningRequestContext | null => {
    try {
        return useSigningRequest()
    } catch {
        return null
    }
}

/**
 * For the routes of protected actions: when the answer is a signing request,
 * opens its panel, which says the action waits for signatures, with the
 * signing dialog when the viewer can sign.
 * Returns whether it did, so the caller skips what it does once the action ran;
 * without a `SigningProvider` there is no panel, and it doesn't.
 */
export const useSignedAction = () => {
    const signing = useOptionalSigningRequest()
    return useCallback(
        (
            reply: ISigningRequiredReply | null | undefined,
            options?: IOpenSigningRequestOptions
        ): boolean => {
            const requestId = signingRequestOf(reply)
            if (!requestId || !signing) return false
            // The dialog opens at once when the viewer can sign it.
            signing.open(requestId, {sign: true, ...options})
            return true
        },
        [signing]
    )
}

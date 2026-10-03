// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {useCallback, useRef, useState} from "react"
import {Button} from "@mui/material"
import {useTranslation} from "react-i18next"
import {
    signingRequestOf,
    useOptionalSigningRequest,
    useSignedAction,
    type ISigningRequiredReply,
} from "@/hooks/useSignedAction"
import type {ISigningPanelData} from "@/lib/signing/api"
import {hex} from "@/lib/signing/der"
import {SigningRequestStatus} from "@/lib/signing/types"

/** What a trustee's key step sends: the key share file's text, and its hash or request. */
export interface IKeyShareSubmission {
    privateKeyBase64: string
    keyShareSha256?: string
    signingRequestId?: string
}

/**
 * What `/check-private-key` and `/restore-private-key` answer; `signing_request`
 * is the request the trustee signs before the step is recorded.
 */
export interface IKeyShareAnswer extends ISigningRequiredReply {
    is_valid: boolean
}

export enum KeyShareCheck {
    /** The step was recorded. */
    Verified = "verified",
    /** The file is not the trustee's key share. */
    Invalid = "invalid",
    /** The trustee signs first; the step is recorded once they have signed. */
    Signing = "signing",
}

/** SHA-256 of the key share file's text (UTF-8), lowercase hex: what the signed request names. */
export const keyShareSha256 = async (text: string): Promise<string> =>
    hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(text)))

export interface IKeyShareSigningOptions {
    submit: (submission: IKeyShareSubmission) => Promise<IKeyShareAnswer>
    /** The step was recorded with the trustee's signature. */
    onRecorded: () => void
    /** Recording the signed step failed; `error` says why. */
    onFailed: (error: string) => void
}

/**
 * A trustee's key step (confirm or contribute a key share) behind their own
 * signature: the file is checked and hashed here; when the event's rule needs
 * signatures the route answers a request naming that hash, the signing panel
 * opens, and once the trustee has signed the step is taken again with the
 * request. The key share itself only goes to the step's own route, never into
 * the request.
 */
export const useKeyShareSigning = ({submit, onRecorded, onFailed}: IKeyShareSigningOptions) => {
    const {t} = useTranslation()
    const signing = useOptionalSigningRequest()
    const openSigned = useSignedAction()
    // The file's text stays in memory only until the signed step is taken.
    const keyShare = useRef<string | null>(null)
    const finishing = useRef(false)
    const [waiting, setWaiting] = useState(false)

    const finish = useCallback(
        async (requestId: string) => {
            if (finishing.current) return
            const text = keyShare.current
            if (text === null) {
                // E.g. after a reload: the file has to be dropped again.
                onFailed(t("signing.keyShare.dropAgain"))
                return
            }
            finishing.current = true
            try {
                const answer = await submit({privateKeyBase64: text, signingRequestId: requestId})
                setWaiting(false)
                if (!answer.is_valid) {
                    onFailed(t("signing.keyShare.notTaken"))
                    return
                }
                signing?.close()
                onRecorded()
            } catch (error) {
                setWaiting(false)
                onFailed(error instanceof Error ? error.message : String(error))
            } finally {
                keyShare.current = null
                finishing.current = false
            }
        },
        [submit, onRecorded, onFailed, signing, t]
    )

    const isCompleted = (data: ISigningPanelData) =>
        data.request.status === SigningRequestStatus.Completed

    const check = useCallback(
        async (text: string): Promise<KeyShareCheck> => {
            keyShare.current = null
            setWaiting(false)
            const answer = await submit({
                privateKeyBase64: text,
                keyShareSha256: await keyShareSha256(text),
            })
            if (!answer.is_valid) return KeyShareCheck.Invalid
            if (!signingRequestOf(answer)) return KeyShareCheck.Verified
            keyShare.current = text
            // The panel opens with the signing dialog, as for every protected action.
            const opened = openSigned(answer, {
                onChange: (data) => {
                    if (isCompleted(data)) {
                        finish(data.request.id).catch(() => undefined)
                    }
                },
                // A request signed earlier (e.g. before a reload) is recorded from its panel.
                completionActions: (data) =>
                    isCompleted(data) ? (
                        <Button
                            variant="contained"
                            onClick={() => {
                                finish(data.request.id).catch(() => undefined)
                            }}
                        >
                            {t("signing.keyShare.record")}
                        </Button>
                    ) : null,
            })
            if (!opened) {
                keyShare.current = null
                throw new Error(t("signing.widget.cantSign"))
            }
            setWaiting(true)
            return KeyShareCheck.Signing
        },
        [submit, openSigned, finish, t]
    )

    return {check, waiting}
}

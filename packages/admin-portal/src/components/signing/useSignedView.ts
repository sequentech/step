// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useMemo, useState} from "react"
import type {ISigningPanelData} from "@/lib/signing/api"
import {
    PayloadMismatchError,
    PayloadProblem,
    payloadToSign,
    signedView,
    type ISignedView,
} from "@/lib/signing/request"

export interface ISignedViewState {
    /** What the payload signs; `null` when it was refused. */
    view: ISignedView | null
    problem: PayloadProblem | null
    /** The payload also hashes to `payload_sha256`: signing may start. */
    verified: boolean
}

const problemOf = (error: unknown) =>
    error instanceof PayloadMismatchError ? error.problem : PayloadProblem.Mismatch

/** The request as its canonical payload describes it (see `signedView`). */
export const useSignedView = (data: ISigningPanelData): ISignedViewState => {
    const checked = useMemo(() => {
        try {
            return {view: signedView(data), problem: null}
        } catch (error) {
            return {view: null, problem: problemOf(error)}
        }
    }, [data])
    const [digest, setDigest] = useState<{
        data: ISigningPanelData
        problem: PayloadProblem | null
    }>()

    useEffect(() => {
        let current = true
        payloadToSign(data).then(
            () => current && setDigest({data, problem: null}),
            (error: unknown) => current && setDigest({data, problem: problemOf(error)})
        )
        return () => {
            current = false
        }
    }, [data])

    const digestProblem = digest?.data === data ? digest.problem : null
    return {
        view: checked.view,
        problem: checked.problem ?? digestProblem,
        verified: !checked.problem && digest?.data === data && !digest.problem,
    }
}

/** The translation key telling the signer why the request can't be signed as shown. */
export const problemMessage = (problem: PayloadProblem): string =>
    problem === PayloadProblem.Document
        ? "signing.widget.documentMismatch"
        : "signing.widget.mismatch"

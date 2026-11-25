// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useContext, useEffect, useRef, useState} from "react"
import {useMutation} from "@apollo/client"
import {Button} from "@mui/material"
import {Dialog} from "@sequentech/ui-essentials"
import {useNotify} from "react-admin"
import {useTranslation} from "react-i18next"
import {v4 as uuidv4} from "uuid"
import {MANUAL_VERIFICATION} from "@/queries/ManualVerification"
import type {ManualVerificationMutation, ManualVerificationMutationVariables} from "@/gql/graphql"
import {useSignedAction} from "@/hooks/useSignedAction"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {IPermissions} from "@/types/keycloak"
import type {ISigningPanelData} from "@/lib/signing/api"
import {verificationResult, verificationRetrieval} from "./manualVerificationAction"
import {DownloadDocument} from "./DownloadDocument"

const messages = "usersAndRolesScreen.voters"
/** Only this mounted action may handle its result. A failed retry keeps its original initiation. */
function useVerificationDocument() {
    const [fetch] = useMutation<ManualVerificationMutation, ManualVerificationMutationVariables>(
        MANUAL_VERIFICATION,
        {
            fetchPolicy: "no-cache",
            context: {headers: {"x-hasura-role": IPermissions.VOTER_MANUALLY_VERIFY}},
        }
    )
    const {t} = useTranslation()
    const notify = useNotify()
    const mounted = useRef(false)
    const inFlight = useRef(false)
    const [busy, setBusy] = useState(false)
    const [failedAttempts, setFailedAttempts] = useState(0)
    const [documentId, setDocumentId] = useState<string>()
    useEffect(() => {
        mounted.current = true
        return () => {
            mounted.current = false
        }
    }, [])
    const request = async (
        variables: ManualVerificationMutationVariables,
        onWaiting?: (requestId: string) => void
    ) => {
        if (inFlight.current || documentId) return
        inFlight.current = true
        setBusy(true)
        try {
            const response = await fetch({variables})
            if (!mounted.current) return
            if (response.errors?.length) throw new Error("Manual verification refused")
            const result = verificationResult(response.data?.get_manual_verification_pdf)
            if (result.requestId) {
                if (!onWaiting) throw new Error("Manual verification not settled")
                onWaiting(result.requestId)
            } else {
                setDocumentId(result.documentId)
            }
        } catch {
            if (mounted.current) {
                setFailedAttempts((count) => count + 1)
                notify(t(`${messages}.notifications.manualVerificationError`), {type: "error"})
            }
        } finally {
            inFlight.current = false
            if (mounted.current) setBusy(false)
        }
    }
    return {busy, request, documentId, failedAttempts, clear: () => setDocumentId(undefined)}
}

const VerificationDownload = ({
    documentId,
    electionEventId,
    onDone,
}: {
    documentId: string
    electionEventId: string
    onDone: () => void
}) => {
    const notify = useNotify()
    const {t} = useTranslation()
    return (
        <DownloadDocument
            documentId={documentId}
            electionEventId={electionEventId}
            fileName={`manual-verify-${electionEventId}-${documentId}.pdf`}
            withProgress
            onDownload={() => {
                notify(t(`${messages}.notifications.manualVerificationSuccess`), {type: "success"})
                onDone()
            }}
        />
    )
}

export const ManualVerificationDialog = ({
    tenantId,
    electionEventId,
    voterId,
    reachable,
    onClose,
}: {
    tenantId: string
    electionEventId: string
    voterId: string
    reachable: boolean
    onClose: () => void
}) => {
    const {t} = useTranslation()
    const notify = useNotify()
    const openSigning = useSignedAction()
    const [initiationId] = useState(() => uuidv4())
    const {request, busy, documentId, failedAttempts} = useVerificationDocument()
    return (
        <Dialog
            key={failedAttempts}
            variant="warning"
            open
            okEnabled={() => reachable && !busy && !documentId}
            ok={String(t(`${messages}.manualVerification.verify`))}
            cancel={String(t("common.label.cancel"))}
            title={String(t(`${messages}.manualVerification.label`))}
            errorMessage={
                reachable ? undefined : t(`${messages}.manualVerification.noEmailOrPhone`)
            }
            handleClose={(confirmed) => {
                if (!confirmed) return onClose()
                if (!reachable) return
                void request({tenantId, electionEventId, voterId, initiationId}, (id) => {
                    if (!openSigning({signing_request: {id}}, {eventId: electionEventId})) {
                        notify(t(`${messages}.manualVerification.waiting`), {type: "info"})
                    }
                    onClose()
                })
            }}
        >
            {t(`${messages}.manualVerification.body`)}
            {documentId && (
                <VerificationDownload
                    documentId={documentId}
                    electionEventId={electionEventId}
                    onDone={onClose}
                />
            )}
        </Dialog>
    )
}

const RetrieveVerificationDocument = ({
    variables,
}: {
    variables: ManualVerificationMutationVariables
}) => {
    const {t} = useTranslation()
    const {request, busy, documentId, clear} = useVerificationDocument()
    return (
        <>
            <Button
                variant="contained"
                disabled={busy || !!documentId}
                onClick={() => void request(variables)}
            >
                {t(`${messages}.manualVerification.download`)}
            </Button>
            {documentId && (
                <VerificationDownload
                    documentId={documentId}
                    electionEventId={variables.electionEventId}
                    onDone={clear}
                />
            )}
        </>
    )
}

export const ManualVerificationCompletion = ({data}: {data: ISigningPanelData}) => {
    const {userId, isAuthorized} = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    if (!isAuthorized(true, tenantId, IPermissions.VOTER_MANUALLY_VERIFY)) return null
    const variables = verificationRetrieval(data, userId, tenantId)
    return variables ? (
        <RetrieveVerificationDocument key={JSON.stringify(variables)} variables={variables} />
    ) : null
}

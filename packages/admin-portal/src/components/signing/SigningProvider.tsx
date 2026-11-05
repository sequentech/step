// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import React, {
    createContext,
    useCallback,
    useContext,
    useEffect,
    useMemo,
    useRef,
    useState,
} from "react"
import {useApolloClient} from "@apollo/client"
import {Alert, Button, Snackbar} from "@mui/material"
import {useTranslation} from "react-i18next"
import {AuthContext} from "@/providers/AuthContextProvider"
import {useTenantStore} from "@/providers/TenantContextProvider"
import {createSigningApi, type ISigningApi, type ISigningPanelData} from "@/lib/signing/api"
import {takeResume} from "@/lib/signing/request"
import type {SigningAction} from "@/lib/signing/types"
import type {IPermissions} from "@/types/keycloak"
import {SigningRequestPanel} from "./SigningRequestPanel"

export interface IOpenSigningRequestOptions {
    /** Opens the signing dialog as soon as the request loads, if the viewer can sign it. */
    sign?: boolean
    /** Replaces the provider's completion actions for this request. */
    completionActions?: (data: ISigningPanelData) => React.ReactNode
    /** After each change the viewer made (a signature, a cancellation). */
    onChange?: (data: ISigningPanelData) => void
    /** Only a request of this election event is shown (a handover note names it). */
    eventId?: string
}

export interface ISigningRequestContext {
    /** Opens the request panel for a request id, e.g. the `signing_request` a guarded route answered. */
    open: (requestId: string, options?: IOpenSigningRequestOptions) => void
    close: () => void
    /** The request whose panel is open. */
    requestId: string | null
}

const SigningContext = createContext<ISigningRequestContext | null>(null)

/** Opens a signing request's panel from anywhere below `SigningProvider`. */
export const useSigningRequest = (): ISigningRequestContext => {
    const context = useContext(SigningContext)
    if (!context) {
        throw new Error("useSigningRequest must be used within a SigningProvider")
    }
    return context
}

export interface ISigningProviderProps {
    children?: React.ReactNode
    /** The Harvest calls; the Hasura actions by default. */
    api?: ISigningApi
    /** What each action's completed request offers, e.g. downloading the signed PDF. */
    completionActions?: Partial<Record<SigningAction, (data: ISigningPanelData) => React.ReactNode>>
    /** Where a handover leaves its note; session storage by default. */
    storage?: Storage
}

const RenderFailure: React.FC<{open: boolean; onClose: () => void}> = ({open, onClose}) => {
    const {t} = useTranslation()
    return (
        <Snackbar open={open} anchorOrigin={{vertical: "bottom", horizontal: "right"}}>
            <Alert
                severity="error"
                action={
                    <Button color="inherit" size="small" onClick={onClose}>
                        {t("signing.widget.close")}
                    </Button>
                }
            >
                {t("signing.widget.renderError")}
            </Alert>
        </Snackbar>
    )
}

/**
 * Keeps a request that can't be shown (a malformed answer, a failing
 * completion action) from unmounting the portal around it.
 */
export class SigningErrorBoundary extends React.Component<
    {children: React.ReactNode; open: boolean; onClose: () => void},
    {failed: boolean}
> {
    state = {failed: false}

    static getDerivedStateFromError() {
        return {failed: true}
    }

    render() {
        return this.state.failed ? (
            <RenderFailure open={this.props.open} onClose={this.props.onClose} />
        ) : (
            this.props.children
        )
    }
}

interface IOpened extends IOpenSigningRequestOptions {
    requestId: string
    /** Each opening starts afresh, even for the same request. */
    nonce: number
}

/**
 * Hosts the signing request panel and its dialog, opened with
 * `useSigningRequest().open(id)`. After a handover, once the next member has
 * signed in, it reopens the request and its dialog for them.
 */
export const SigningProvider: React.FC<ISigningProviderProps> = ({
    children,
    api,
    completionActions,
    storage,
}) => {
    const client = useApolloClient()
    const {isAuthenticated, userId, isAuthorized} = useContext(AuthContext)
    const [tenantId] = useTenantStore()
    // Read at each call, so the api (and the actions it remembers) outlives a token refresh.
    const holdsRef = useRef<(permission: IPermissions) => boolean>(() => false)
    holdsRef.current = (permission) => isAuthorized(true, tenantId, permission)
    const signingApi = useMemo(
        () =>
            api ??
            createSigningApi(client, undefined, {
                holds: (permission) => holdsRef.current(permission),
            }),
        [api, client]
    )
    const [opened, setOpened] = useState<IOpened | null>(null)
    const [visible, setVisible] = useState(false)

    const open = useCallback((requestId: string, options: IOpenSigningRequestOptions = {}) => {
        setOpened((previous) => ({requestId, ...options, nonce: (previous?.nonce ?? 0) + 1}))
        setVisible(true)
    }, [])
    const close = useCallback(() => setVisible(false), [])

    useEffect(() => {
        if (!isAuthenticated || !userId || !tenantId) return
        let target: Storage | null = storage ?? null
        try {
            target ??= window.sessionStorage
        } catch {
            return
        }
        const resume = takeResume(target, {tenantId})
        if (resume) {
            open(resume.requestId, {sign: true, eventId: resume.eventId})
        }
    }, [isAuthenticated, userId, tenantId, storage, open])

    const context = useMemo(
        () => ({open, close, requestId: visible ? (opened?.requestId ?? null) : null}),
        [open, close, visible, opened]
    )
    const actions = opened
        ? (opened.completionActions ??
          ((data: ISigningPanelData) => completionActions?.[data.request.action]?.(data)))
        : undefined

    return (
        <SigningContext.Provider value={context}>
            {children}
            {opened ? (
                <SigningErrorBoundary key={opened.nonce} open={visible} onClose={close}>
                    <SigningRequestPanel
                        requestId={opened.requestId}
                        api={signingApi}
                        open={visible}
                        onClose={close}
                        autoSign={opened.sign}
                        completionActions={actions}
                        onChange={opened.onChange}
                        storage={storage}
                        expectedEventId={opened.eventId}
                    />
                </SigningErrorBoundary>
            ) : null}
        </SigningContext.Provider>
    )
}

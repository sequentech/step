// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
// SPDX-License-Identifier: AGPL-3.0-only

import React, {useEffect, useLayoutEffect, useMemo, useState} from "react"
import {Provider} from "react-redux"
import {ApolloProvider} from "@apollo/client/react"
import {Alert, AlertTitle, ThemeProvider} from "@mui/material"
import {useTranslation} from "react-i18next"
import {Loader, theme} from "@sequentech/ui-essentials"
import {AuthContext} from "../providers/AuthContextProvider"
import {SettingsContext} from "../providers/SettingsContextProvider"
import {BallotSelectionAdapter} from "../components/BallotSelectionAdapter"
import {WasmWrapper} from "../providers/WasmWrapper"
import {useEncryptBallotForReview} from "../hooks/useEncryptBallotForReview"
import {BallotStyleConfigurationError} from "../services/BallotStyles"
import {store} from "../store/store"
import {createPreviewApolloClient, PREVIEW_SETTINGS, previewAuth} from "./context"
import {loadPreviewSnapshot, preparePreviewScreen, type PreviewSession} from "./session"

export interface VoterPreviewProps extends React.PropsWithChildren {
    /** A new session object reloads the voter state; the same one keeps it. */
    session: PreviewSession
    /** Receives the portal's logout calls, such as when no vote remains. */
    onLogout?: (redirectUrl?: string) => void
    /** A voting portal language that wins over the event's default language. */
    language?: string
    /** Receives the reason the portal could not load the session's document. */
    onLoadError?: (error: unknown) => void
}

const LoadError: React.FC<{error: unknown}> = ({error}) => {
    const {t} = useTranslation()
    const message =
        error instanceof BallotStyleConfigurationError
            ? t(error.translationKey, error.translationParams)
            : String(error instanceof Error ? error.message : error)
    return (
        <Alert severity="error" className="preview-load-error" sx={{margin: 2}}>
            <AlertTitle className="preview-load-error-title">
                The portal could not load this snapshot
            </AlertTitle>
            {message}
        </Alert>
    )
}

const SessionGate: React.FC<
    {session: PreviewSession; onLoadError?: (error: unknown) => void} & React.PropsWithChildren
> = ({session, onLoadError, children}) => {
    const {encryptAndStoreBallot} = useEncryptBallotForReview()
    const [loaded, setLoaded] = useState<{session: PreviewSession; error?: unknown}>()

    // Before paint, so no screen renders against the previous session's state.
    useLayoutEffect(() => {
        try {
            loadPreviewSnapshot(session.snapshot, store.dispatch)
            preparePreviewScreen(session, store.getState(), store.dispatch, encryptAndStoreBallot)
            setLoaded({session})
        } catch (error) {
            setLoaded({session, error})
            onLoadError?.(error)
        }
        // The callback is read at load time only, so a new one does not reload the session.
    }, [session, encryptAndStoreBallot])

    if (loaded?.session !== session) return <Loader />
    if (loaded.error) return <LoadError error={loaded.error} />
    return <>{children}</>
}

/**
 * The voting portal's providers for a synthetic snapshot: its theme, settings with
 * authentication disabled, a voter reaching it by the snapshot's channel, a GraphQL client
 * that rejects every operation, the production WASM gate and Redux store. The snapshot
 * enters the store through the production preview loader; nothing is fetched.
 */
export const VoterPreview: React.FC<VoterPreviewProps> = ({
    session,
    onLogout,
    language,
    onLoadError,
    children,
}) => {
    const [client] = useState(createPreviewApolloClient)
    const {i18n} = useTranslation()
    // An explicit language counts as the voter's choice, as a `lang` parameter does.
    const [defaultLanguageTouched, setDefaultLanguageTouched] = useState(language !== undefined)

    useEffect(() => {
        if (language === undefined) return
        setDefaultLanguageTouched(true)
        if (i18n.language !== language) void i18n.changeLanguage(language)
    }, [language, i18n])

    const settings = useMemo(
        () => ({
            loaded: true,
            globalSettings: PREVIEW_SETTINGS,
            defaultLanguageTouched,
            setDefaultLanguageTouched,
            setDisableAuth: () => undefined,
        }),
        [defaultLanguageTouched]
    )
    const auth = useMemo(
        () => previewAuth(session.snapshot.channel, (redirectUrl) => onLogout?.(redirectUrl)),
        [session.snapshot.channel, onLogout]
    )

    return (
        <ThemeProvider theme={theme}>
            <SettingsContext.Provider value={settings}>
                <AuthContext.Provider value={auth}>
                    <ApolloProvider client={client}>
                        <Provider store={store}>
                            <BallotSelectionAdapter>
                                <WasmWrapper>
                                    <SessionGate session={session} onLoadError={onLoadError}>
                                        {children}
                                    </SessionGate>
                                </WasmWrapper>
                            </BallotSelectionAdapter>
                        </Provider>
                    </ApolloProvider>
                </AuthContext.Provider>
            </SettingsContext.Provider>
        </ThemeProvider>
    )
}

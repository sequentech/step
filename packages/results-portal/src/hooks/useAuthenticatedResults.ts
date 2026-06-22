// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only

import {useEffect, useMemo, useState} from "react"
import Keycloak from "keycloak-js"
import {GlobalSettings} from "@/providers/SettingsContextProvider"

interface AuthState {
    loading: boolean
    token?: string
    error?: string
}

export const useAuthenticatedResults = (
    settings: GlobalSettings,
    tenantId?: string,
    eventId?: string,
    required = false
): AuthState => {
    const [state, setState] = useState<AuthState>({loading: required})

    const realm = useMemo(
        () => (tenantId && eventId ? `tenant-${tenantId}-event-${eventId}` : undefined),
        [tenantId, eventId]
    )

    useEffect(() => {
        if (!required || settings.DISABLE_AUTH) {
            setState({loading: false})
            return
        }

        if (!realm) {
            setState({loading: true})
            return
        }

        let mounted = true
        const keycloak = new Keycloak({
            url: settings.KEYCLOAK_URL,
            realm,
            clientId: settings.ONLINE_VOTING_CLIENT_ID,
        })

        const init = async () => {
            try {
                const authenticated = await keycloak.init({
                    onLoad: "login-required",
                    checkLoginIframe: false,
                })

                if (mounted) {
                    setState({
                        loading: false,
                        token: authenticated ? keycloak.token : undefined,
                    })
                }
            } catch (error) {
                if (mounted) {
                    setState({
                        loading: false,
                        error: error instanceof Error ? error.message : "Authentication failed",
                    })
                }
            }
        }

        void init()

        return () => {
            mounted = false
        }
    }, [required, realm, settings.DISABLE_AUTH, settings.KEYCLOAK_URL, settings.ONLINE_VOTING_CLIENT_ID])

    return state
}

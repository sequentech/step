// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React, {Suspense, useContext} from "react"
import {useTranslation} from "react-i18next"
import {Tabs} from "@/components/Tabs"
import {AuthContext} from "@/providers/AuthContextProvider"
import {IPermissions} from "@/types/keycloak"
import {PhoneBlacklist} from "./PhoneBlacklist"
import {IvrConfig} from "./IvrConfig"
import {IvrPrompts} from "./IvrPrompts"
import {IvrEmulator} from "./IvrEmulator"
import {Box} from "@mui/material"
import {IvrEmulatorContextProvider} from "@/providers/IvrEmulatorContextProvider"

const ConfigTab: React.FC = () => (
    <Suspense fallback={<div>Loading...</div>}>
        <IvrConfig />
    </Suspense>
)

const BlacklistTab: React.FC = () => (
    <Suspense fallback={<div>Loading...</div>}>
        <PhoneBlacklist />
    </Suspense>
)

const PromptsTab: React.FC = () => (
    <Suspense fallback={<div>Loading...</div>}>
        <IvrPrompts />
    </Suspense>
)

const EmulatorTab: React.FC = () => (
    <Suspense fallback={<div>Loading...</div>}>
        <IvrEmulatorContextProvider>
            <IvrEmulator />
        </IvrEmulatorContextProvider>
    </Suspense>
)

export const EditElectionEventIvr: React.FC = () => {
    const {t} = useTranslation()
    const authContext = useContext(AuthContext)

    const tabs: Array<{label: string; component: React.ComponentType<any>}> = []

    tabs.push({
        label: t("electionEventScreen.ivr.tabs.config"),
        component: ConfigTab,
    })

    tabs.push({
        label: t("electionEventScreen.ivr.tabs.prompts"),
        component: PromptsTab,
    })

    if (authContext.isAuthorized(true, authContext.tenantId, IPermissions.PHONE_BLACKLIST_READ)) {
        tabs.push({
            label: t("electionEventScreen.ivr.tabs.blacklist"),
            component: BlacklistTab,
        })
    }

    tabs.push({
        label: t("electionEventScreen.ivr.tabs.emulator"),
        component: EmulatorTab,
    })

    return (
        <Box sx={{margin: "-1.5rem 0 0 0"}}>
            <Tabs elements={tabs} />
        </Box>
    )
}

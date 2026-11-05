// SPDX-FileCopyrightText: 2026 Sequent Tech Inc <legal@sequentech.io>
//
// SPDX-License-Identifier: AGPL-3.0-only
import React from "react"
import {useTranslation} from "react-i18next"
import {useRecordContext} from "react-admin"
import {Box, Typography} from "@mui/material"
import {EElectionEventLockedDown} from "@sequentech/ui-core"
import {Tabs} from "@/components/Tabs"
import type {Sequent_Backend_Election_Event} from "@/gql/graphql"
import {ProtectedActionsTab} from "./ProtectedActionsTab"
import {CertificatesTab} from "./CertificatesTab"
import {RequestsTab} from "./RequestsTab"
import {useSignaturesAccess} from "./useSigningSettings"

/**
 * Election Event > Signatures: one sub-tab per part of the signing settings,
 * each shown only with its read permission.
 */
export const EditElectionEventSignatures: React.FC = () => {
    const {t} = useTranslation()
    const record = useRecordContext<Sequent_Backend_Election_Event>()
    const access = useSignaturesAccess()
    if (!record?.id) {
        return null
    }
    const lockedDown = record.presentation?.locked_down === EElectionEventLockedDown.LOCKED_DOWN
    const props = {electionEventId: record.id, access, lockedDown}
    const tabs: Array<{
        label: string
        component: React.ComponentType<typeof props>
        props: typeof props
    }> = []
    if (access.rulesRead) {
        tabs.push({
            label: t("signing.tab.protectedActions"),
            component: ProtectedActionsTab,
            props,
        })
    }
    if (access.certificatesRead) {
        tabs.push({label: t("signing.tab.certificates"), component: CertificatesTab, props})
    }
    if (access.requestsRead) {
        tabs.push({label: t("signing.tab.requests"), component: RequestsTab, props})
    }
    return (
        <Box>
            <Typography variant="body2" color="text.secondary" sx={{maxWidth: "70rem", mb: 2}}>
                {t("signing.tab.intro")}
            </Typography>
            <Tabs elements={tabs} />
        </Box>
    )
}
